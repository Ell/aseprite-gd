//! Frame flattening (§8): cel ordering, opacity, visibility, blending,
//! tilemap materialization, group buffers.
//!
//! Two composition paths (§6.2 NOTE.6): when the header says group blend
//! modes are valid (flag bit 1), each group is composited into its own
//! buffer and blended onto the backdrop with the group's mode/opacity;
//! otherwise groups are pass-through folders and cels composite flat.

use std::borrow::Cow;
use std::fmt;

use crate::composite::blend::{Rgba, blend, mul_un8};
use crate::file::AseFile;
use crate::limits::{MAX_CANVAS_DIM, MAX_IMAGE_BYTES};
use crate::model::{
    BlendMode, Cel, CelContent, CelImage, CelTilemap, ColorDepth, LayerType, Palette,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    BadFrame(usize),
    BrokenLink {
        frame: usize,
        layer: usize,
    },
    MissingTileset {
        layer: usize,
    },
    /// Construct not composited yet (currently: external tilesets).
    Unsupported(&'static str),
    /// A safety limit from [`crate::limits`] was exceeded while compositing
    /// (e.g. a materialized tilemap larger than any legitimate sprite).
    LimitExceeded(&'static str),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BadFrame(i) => write!(f, "frame {i} out of range"),
            RenderError::BrokenLink { frame, layer } => {
                write!(
                    f,
                    "linked cel resolution failed (frame {frame}, layer {layer})"
                )
            }
            RenderError::MissingTileset { layer } => {
                write!(f, "tilemap layer {layer} references a missing tileset")
            }
            RenderError::Unsupported(what) => write!(f, "unsupported for rendering: {what}"),
            RenderError::LimitExceeded(what) => {
                write!(f, "safety limit exceeded while rendering: {what}")
            }
        }
    }
}

impl std::error::Error for RenderError {}

/// A flattened frame: straight-alpha RGBA, row-major, canvas-sized.
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Converts one cel pixel to RGBA per the sprite color depth (§7).
/// Returns None for transparent-by-definition pixels (indexed transparent
/// index on non-background layers, out-of-range indices).
fn pixel_to_rgba(
    depth: ColorDepth,
    palette: &Palette,
    transparent_index: u8,
    on_background: bool,
    px: &[u8],
) -> Option<Rgba> {
    match depth {
        ColorDepth::Rgba => Some([px[0], px[1], px[2], px[3]]),
        ColorDepth::Grayscale => Some([px[0], px[0], px[0], px[1]]),
        ColorDepth::Indexed => {
            let idx = px[0];
            if idx == transparent_index && !on_background {
                return None; // gotcha #7
            }
            // Out-of-range indices are transparent; on a background layer
            // every index paints opaque (§7).
            palette.entries.get(idx as usize).map(|&[r, g, b, a]| {
                if on_background {
                    [r, g, b, 255]
                } else {
                    [r, g, b, a]
                }
            })
        }
    }
}

/// Expands a tilemap cel into a flat pixel buffer by copying tiles out of the
/// tileset strip, applying flips (§6.3 type 3: transpose, then X, then Y).
fn materialize_tilemap(
    file: &AseFile,
    layer_index: usize,
    tm: &CelTilemap,
) -> Result<CelImage, RenderError> {
    let LayerType::Tilemap { tileset_index } = file.layers[layer_index].layer_type else {
        return Err(RenderError::MissingTileset { layer: layer_index });
    };
    let ts = file
        .tilesets
        .iter()
        .find(|t| t.id == tileset_index)
        .ok_or(RenderError::MissingTileset { layer: layer_index })?;
    let strip = ts
        .pixels
        .as_ref()
        .ok_or(RenderError::Unsupported("external tileset"))?;

    let bpp = file.header.color_depth.bytes_per_pixel();
    let (tw, th) = (ts.tile_width as usize, ts.tile_height as usize);
    let (w, h) = (tm.width as usize * tw, tm.height as usize * th);
    // All four factors are file-derived u16s: the materialized buffer can
    // reach petabytes (and `w as u16` below would truncate) without a cap.
    if w > MAX_CANVAS_DIM as usize || h > MAX_CANVAS_DIM as usize {
        return Err(RenderError::LimitExceeded(
            "materialized tilemap dimensions",
        ));
    }
    if w * h * bpp > MAX_IMAGE_BYTES {
        return Err(RenderError::LimitExceeded("materialized tilemap size"));
    }

    // Empty cells stay transparent: alpha 0 for RGBA/grayscale (zeroed), the
    // transparent index for indexed sprites.
    let fill = match file.header.color_depth {
        ColorDepth::Indexed => file.header.transparent_index,
        _ => 0,
    };
    let mut out = vec![fill; w * h * bpp];

    for (i, tile) in tm.tiles.iter().enumerate() {
        if ts.zero_is_empty() && tile.index == 0 {
            continue;
        }
        let idx = tile.index as usize;
        if idx >= ts.num_tiles as usize {
            continue; // out-of-range tile: treat as empty
        }
        let (cell_x, cell_y) = ((i % tm.width as usize) * tw, (i / tm.width as usize) * th);
        for y in 0..th {
            for x in 0..tw {
                // D-flip transposes; only meaningful for square tiles (all
                // Aseprite tilesets are), clamped defensively otherwise.
                let (mut sx, mut sy) = if tile.d_flip {
                    (y.min(tw - 1), x.min(th - 1))
                } else {
                    (x, y)
                };
                if tile.x_flip {
                    sx = tw - 1 - sx;
                }
                if tile.y_flip {
                    sy = th - 1 - sy;
                }
                let src = ((idx * th + sy) * tw + sx) * bpp;
                let dst = ((cell_y + y) * w + cell_x + x) * bpp;
                out[dst..dst + bpp].copy_from_slice(&strip[src..src + bpp]);
            }
        }
    }

    Ok(CelImage {
        width: w as u16,
        height: h as u16,
        pixels: out,
    })
}

/// Resolves a cel to drawable pixels: follows linked cels to their content
/// (keeping the link's own position/opacity, gotcha #13), materializes
/// tilemaps. Returns None for dangling links (treated as empty).
fn resolve_pixels<'a>(
    file: &'a AseFile,
    frame_index: usize,
    cel: &'a Cel,
) -> Result<Option<Cow<'a, CelImage>>, RenderError> {
    let mut content = &cel.content;
    // Chains are normalized by Aseprite but resolved defensively; each hop
    // must go strictly backward, so frame_index bounds the hops.
    for _ in 0..=frame_index {
        match content {
            CelContent::Image(img) => return Ok(Some(Cow::Borrowed(img))),
            CelContent::Tilemap(tm) => {
                return materialize_tilemap(file, cel.layer_index, tm).map(|i| Some(Cow::Owned(i)));
            }
            CelContent::Linked(target) => {
                let frame = file
                    .frames
                    .get(*target as usize)
                    .ok_or(RenderError::BrokenLink {
                        frame: frame_index,
                        layer: cel.layer_index,
                    })?;
                match frame.cels.iter().find(|c| c.layer_index == cel.layer_index) {
                    Some(c) => content = &c.content,
                    None => return Ok(None),
                }
            }
        }
    }
    Err(RenderError::BrokenLink {
        frame: frame_index,
        layer: cel.layer_index,
    })
}

/// Composites one cel onto `buf` (canvas-sized RGBA).
fn draw_cel(
    file: &AseFile,
    frame_index: usize,
    palette: &crate::model::Palette,
    cel: &Cel,
    buf: &mut [u8],
    w: usize,
    h: usize,
) -> Result<(), RenderError> {
    let layer = &file.layers[cel.layer_index];
    let Some(img) = resolve_pixels(file, frame_index, cel)? else {
        return Ok(());
    };

    // Background layers ignore blend mode and both opacities (gotcha #6).
    let background = layer.is_background();
    let (mode, opacity) = if background {
        (BlendMode::Normal, 255)
    } else {
        (
            layer.blend_mode,
            mul_un8(layer.opacity as i32, cel.opacity as i32),
        )
    };

    let bpp = file.header.color_depth.bytes_per_pixel();
    for row in 0..img.height as usize {
        let dy = cel.y as isize + row as isize;
        if dy < 0 || dy >= h as isize {
            continue; // cels may extend past the canvas (gotcha #14)
        }
        for col in 0..img.width as usize {
            let dx = cel.x as isize + col as isize;
            if dx < 0 || dx >= w as isize {
                continue;
            }
            let src_px = &img.pixels[(row * img.width as usize + col) * bpp..][..bpp];
            let Some(src) = pixel_to_rgba(
                file.header.color_depth,
                palette,
                file.header.transparent_index,
                background,
                src_px,
            ) else {
                continue;
            };
            let di = (dy as usize * w + dx as usize) * 4;
            let back: Rgba = buf[di..di + 4].try_into().unwrap();
            let out = blend(mode, back, src, opacity);
            buf[di..di + 4].copy_from_slice(&out);
        }
    }
    Ok(())
}

/// Recursive group compositing (§6.2 NOTE.6): renders the children of
/// `parent` in order; each child group is flattened into its own transparent
/// buffer, then blended with the group's mode/opacity. Cel z-index reorders
/// siblings within their group.
fn render_children(
    file: &AseFile,
    frame_index: usize,
    palette: &crate::model::Palette,
    parent: Option<usize>,
    buf: &mut [u8],
    w: usize,
    h: usize,
) -> Result<(), RenderError> {
    let frame = &file.frames[frame_index];
    // (sort key, tie-break z, layer index)
    let mut items: Vec<(i32, i16, usize)> = Vec::new();
    for (i, layer) in file.layers.iter().enumerate() {
        if layer.parent != parent || !layer.is_visible() || layer.is_reference() {
            continue;
        }
        if layer.layer_type == LayerType::Group {
            items.push((i as i32, 0, i));
        } else if let Some(cel) = frame.cels.iter().find(|c| c.layer_index == i) {
            items.push((i as i32 + cel.z_index as i32, cel.z_index, i));
        }
    }
    items.sort_by_key(|&(key, z, _)| (key, z));

    for (_, _, index) in items {
        let layer = &file.layers[index];
        if layer.layer_type == LayerType::Group {
            let mut scratch = vec![0u8; w * h * 4];
            render_children(file, frame_index, palette, Some(index), &mut scratch, w, h)?;
            let opacity = layer.opacity as i32;
            for (back_px, src_px) in buf.chunks_exact_mut(4).zip(scratch.chunks_exact(4)) {
                if src_px[3] == 0 {
                    continue;
                }
                let back: Rgba = back_px.try_into().unwrap();
                let src: Rgba = src_px.try_into().unwrap();
                back_px.copy_from_slice(&blend(layer.blend_mode, back, src, opacity));
            }
        } else if let Some(cel) = frame.cels.iter().find(|c| c.layer_index == index) {
            draw_cel(file, frame_index, palette, cel, buf, w, h)?;
        }
    }
    Ok(())
}

/// Flattens one frame to canvas-sized RGBA (§8).
pub fn render_frame(file: &AseFile, frame_index: usize) -> Result<RgbaImage, RenderError> {
    let frame = file
        .frames
        .get(frame_index)
        .ok_or(RenderError::BadFrame(frame_index))?;
    let palette = file.palette_for(frame_index);
    let (w, h) = (file.header.width as usize, file.header.height as usize);
    let mut buf = vec![0u8; w * h * 4];

    if file.header.group_blend_valid() {
        render_children(file, frame_index, palette, None, &mut buf, w, h)?;
        return Ok(RgbaImage {
            width: w as u32,
            height: h as u32,
            pixels: buf,
        });
    }

    // Pass-through groups: flat draw order over all cels — layer index +
    // z-index, ties broken by z-index (§8, NOTE.5).
    let mut order: Vec<&Cel> = frame
        .cels
        .iter()
        .filter(|c| {
            let layer = &file.layers[c.layer_index];
            layer.layer_type != LayerType::Group // group cels are invalid (gotcha #5)
                && !layer.is_reference()
                && file.layer_visible_in_tree(c.layer_index)
        })
        .collect();
    order.sort_by_key(|c| (c.layer_index as i32 + c.z_index as i32, c.z_index));

    for cel in order {
        draw_cel(file, frame_index, palette, cel, &mut buf, w, h)?;
    }

    Ok(RgbaImage {
        width: w as u32,
        height: h as u32,
        pixels: buf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CelContent, CelImage, ColorDepth, Frame, Header, Layer, Palette};

    /// 1x1 canvas: base gray(100) layer, plus a group containing one
    /// Addition-blend child cel of gray(100).
    fn group_file(header_flags: u32) -> AseFile {
        let header = Header {
            frames: 1,
            width: 1,
            height: 1,
            color_depth: ColorDepth::Rgba,
            flags: header_flags,
            default_frame_duration_ms: 100,
            transparent_index: 0,
            num_colors: 256,
            pixel_ratio: (1, 1),
            grid_x: 0,
            grid_y: 0,
            grid_width: 0,
            grid_height: 0,
        };
        let layer = |layer_type, parent: Option<usize>, blend_mode| Layer {
            flags: 1, // visible
            layer_type,
            child_level: if parent.is_none() { 0 } else { 1 },
            blend_mode,
            opacity: 255,
            name: String::new(),
            parent,
            uuid: None,
            user_data: Default::default(),
        };
        let cel = |layer_index| crate::model::Cel {
            layer_index,
            x: 0,
            y: 0,
            opacity: 255,
            z_index: 0,
            content: CelContent::Image(CelImage {
                width: 1,
                height: 1,
                pixels: vec![100, 100, 100, 255],
            }),
            extra: None,
            user_data: Default::default(),
        };
        AseFile {
            header,
            layers: vec![
                layer(LayerType::Image, None, BlendMode::Normal),
                layer(LayerType::Group, None, BlendMode::Normal),
                layer(LayerType::Image, Some(1), BlendMode::Addition),
            ],
            frames: vec![Frame {
                duration_ms: 100,
                cels: vec![cel(0), cel(2)],
            }],
            tags: vec![],
            tilesets: vec![],
            slices: vec![],
            palettes: vec![Palette::default()],
            user_data: Default::default(),
            color_profile: None,
            external_files: vec![],
        }
    }

    #[test]
    fn group_buffer_isolates_child_blend_from_backdrop() {
        // Pass-through (flag clear): the Addition child blends against the
        // base layer -> 100 + 100 = 200.
        let flat = render_frame(&group_file(1), 0).unwrap();
        assert_eq!(&flat.pixels, &[200, 200, 200, 255]);

        // Buffered (flag bit 1): the child composites into the group's own
        // transparent buffer (Addition over alpha 0 falls back to normal ->
        // gray 100), then the group blends Normal over the base -> 100.
        let buffered = render_frame(&group_file(1 | 2), 0).unwrap();
        assert_eq!(&buffered.pixels, &[100, 100, 100, 255]);
    }

    #[test]
    fn group_blend_and_opacity_apply_to_buffer() {
        // Group set to Addition at half opacity: buffer gray(100) added onto
        // base gray(100) at opacity 128 -> 100 + round(100*128/255) = 150.
        let mut file = group_file(1 | 2);
        file.layers[1].blend_mode = BlendMode::Addition;
        file.layers[1].opacity = 128;
        file.layers[2].blend_mode = BlendMode::Normal;
        let out = render_frame(&file, 0).unwrap();
        assert_eq!(&out.pixels, &[150, 150, 150, 255]);
    }

    #[test]
    fn hidden_group_hides_children_in_buffered_mode() {
        let mut file = group_file(1 | 2);
        file.layers[1].flags = 0; // group invisible
        let out = render_frame(&file, 0).unwrap();
        assert_eq!(&out.pixels, &[100, 100, 100, 255], "only the base layer");
    }

    /// `tilemap cells x tile size` are all file-derived u16s; unchecked, the
    /// materialized buffer for a hostile file reaches petabytes.
    #[test]
    fn oversized_tilemap_materialization_is_rejected() {
        use crate::model::{CelTilemap, Tileset};

        let mut file = group_file(1);
        file.layers[0].layer_type = LayerType::Tilemap { tileset_index: 0 };
        file.layers.truncate(1);
        file.tilesets = vec![Tileset {
            id: 0,
            flags: 0,
            num_tiles: 1,
            tile_width: u16::MAX,
            tile_height: u16::MAX,
            base_index: 1,
            name: String::new(),
            pixels: Some(Vec::new()),
            external: None,
            user_data: Default::default(),
            tile_user_data: Vec::new(),
        }];
        file.frames[0].cels = vec![crate::model::Cel {
            layer_index: 0,
            x: 0,
            y: 0,
            opacity: 255,
            z_index: 0,
            content: CelContent::Tilemap(CelTilemap {
                width: u16::MAX,
                height: u16::MAX,
                tiles: Vec::new(),
            }),
            extra: None,
            user_data: Default::default(),
        }];

        assert!(matches!(
            render_frame(&file, 0),
            Err(RenderError::LimitExceeded(
                "materialized tilemap dimensions"
            ))
        ));
    }
}
