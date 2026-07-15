//! Frame flattening (§8): cel ordering, opacity, visibility, blending,
//! tilemap materialization.
//!
//! Remaining gaps: group blend/opacity buffers (header flag bit 1) — groups
//! composite as pass-through folders for now.

use std::borrow::Cow;
use std::fmt;

use crate::composite::blend::{blend, mul_un8, Rgba};
use crate::file::AseFile;
use crate::model::{
    BlendMode, Cel, CelContent, CelImage, CelTilemap, ColorDepth, LayerType, Palette,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    BadFrame(usize),
    BrokenLink { frame: usize, layer: usize },
    MissingTileset { layer: usize },
    /// Construct not composited yet (currently: external tilesets).
    Unsupported(&'static str),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BadFrame(i) => write!(f, "frame {i} out of range"),
            RenderError::BrokenLink { frame, layer } => {
                write!(f, "linked cel resolution failed (frame {frame}, layer {layer})")
            }
            RenderError::MissingTileset { layer } => {
                write!(f, "tilemap layer {layer} references a missing tileset")
            }
            RenderError::Unsupported(what) => write!(f, "unsupported for rendering: {what}"),
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
                if on_background { [r, g, b, 255] } else { [r, g, b, a] }
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
    let strip = ts.pixels.as_ref().ok_or(RenderError::Unsupported("external tileset"))?;

    let bpp = file.header.color_depth.bytes_per_pixel();
    let (tw, th) = (ts.tile_width as usize, ts.tile_height as usize);
    let (w, h) = (tm.width as usize * tw, tm.height as usize * th);

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

    Ok(CelImage { width: w as u16, height: h as u16, pixels: out })
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
                let frame = file.frames.get(*target as usize).ok_or(RenderError::BrokenLink {
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
    Err(RenderError::BrokenLink { frame: frame_index, layer: cel.layer_index })
}

/// Flattens one frame to canvas-sized RGBA (§8).
pub fn render_frame(file: &AseFile, frame_index: usize) -> Result<RgbaImage, RenderError> {
    let frame = file.frames.get(frame_index).ok_or(RenderError::BadFrame(frame_index))?;
    let palette = file.palette_for(frame_index);
    let (w, h) = (file.header.width as usize, file.header.height as usize);
    let mut buf = vec![0u8; w * h * 4];

    // Draw order: layer index + z-index, ties broken by z-index (§8, NOTE.5).
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
        let layer = &file.layers[cel.layer_index];
        let Some(img) = resolve_pixels(file, frame_index, cel)? else {
            continue;
        };

        // Background layers ignore blend mode and both opacities (gotcha #6).
        let background = layer.is_background();
        let (mode, opacity) = if background {
            (BlendMode::Normal, 255)
        } else {
            (layer.blend_mode, mul_un8(layer.opacity as i32, cel.opacity as i32))
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
    }

    Ok(RgbaImage { width: w as u32, height: h as u32, pixels: buf })
}
