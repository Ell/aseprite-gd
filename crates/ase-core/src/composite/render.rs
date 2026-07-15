//! Frame flattening (§8): cel ordering, opacity, visibility, blending.
//!
//! Current scope: RGBA sprites, pass-through groups (group blend/opacity
//! buffers per header flag bit 1 are TODO), tilemap cels TODO. Grayscale and
//! indexed sprites convert per §7.

use std::fmt;

use crate::composite::blend::{blend, mul_un8, Rgba};
use crate::file::AseFile;
use crate::model::{BlendMode, CelContent, CelImage, ColorDepth, LayerType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    BadFrame(usize),
    BrokenLink { frame: usize, layer: usize },
    /// Construct not composited yet (tilemap cels, group blend buffers).
    Unsupported(&'static str),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BadFrame(i) => write!(f, "frame {i} out of range"),
            RenderError::BrokenLink { frame, layer } => {
                write!(f, "linked cel resolution failed (frame {frame}, layer {layer})")
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
    file: &AseFile,
    on_background: bool,
    px: &[u8],
) -> Option<Rgba> {
    match file.header.color_depth {
        ColorDepth::Rgba => Some([px[0], px[1], px[2], px[3]]),
        ColorDepth::Grayscale => Some([px[0], px[0], px[0], px[1]]),
        ColorDepth::Indexed => {
            let idx = px[0];
            if idx == file.header.transparent_index && !on_background {
                return None; // gotcha #7
            }
            // Out-of-range indices are transparent; on a background layer
            // every index paints opaque (§7).
            file.palette.entries.get(idx as usize).map(|&[r, g, b, a]| {
                if on_background { [r, g, b, 255] } else { [r, g, b, a] }
            })
        }
    }
}

/// Resolves a cel to drawable content: follows linked cels to their image but
/// keeps the link's own position/opacity (gotcha #13).
fn resolve_image<'a>(
    file: &'a AseFile,
    frame_index: usize,
    layer_index: usize,
    content: &'a CelContent,
) -> Result<Option<&'a CelImage>, RenderError> {
    let mut content = content;
    // Chains are normalized by Aseprite but resolved defensively here; each
    // hop must go strictly backward, so frame_index bounds the hops.
    for _ in 0..=frame_index {
        match content {
            CelContent::Image(img) => return Ok(Some(img)),
            CelContent::Tilemap(_) => return Err(RenderError::Unsupported("tilemap cel")),
            CelContent::Linked(target) => {
                let target = *target as usize;
                let frame = file.frames.get(target).ok_or(RenderError::BrokenLink {
                    frame: frame_index,
                    layer: layer_index,
                })?;
                match frame.cels.iter().find(|c| c.layer_index == layer_index) {
                    Some(c) => content = &c.content,
                    None => return Ok(None), // dangling link: treat as empty cel
                }
            }
        }
    }
    Err(RenderError::BrokenLink { frame: frame_index, layer: layer_index })
}

/// Flattens one frame to canvas-sized RGBA (§8).
pub fn render_frame(file: &AseFile, frame_index: usize) -> Result<RgbaImage, RenderError> {
    let frame = file.frames.get(frame_index).ok_or(RenderError::BadFrame(frame_index))?;
    let (w, h) = (file.header.width as usize, file.header.height as usize);
    let mut buf = vec![0u8; w * h * 4];

    // Draw order: layer index + z-index, ties broken by z-index (§8, NOTE.5).
    let mut order: Vec<&crate::model::Cel> = frame
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
        let Some(img) = resolve_image(file, frame_index, cel.layer_index, &cel.content)? else {
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
                let Some(src) = pixel_to_rgba(file, background, src_px) else {
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
