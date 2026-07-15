//! Cel chunk 0x2005 (§6.3), including zlib decompression under limits.

use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;

use crate::error::ParseError;
use crate::limits::{MAX_IMAGE_BYTES, MAX_TOTAL_DECOMPRESSED_BYTES};
use crate::model::{Cel, CelContent, CelImage, CelTilemap, ColorDepth, Tile};
use crate::read::Reader;
use crate::Result;

/// Running decompression budget across one file (zip-bomb guard).
#[derive(Default)]
pub struct InflateBudget {
    pub total: usize,
}

impl InflateBudget {
    fn charge(&mut self, offset: usize, bytes: usize) -> Result<()> {
        if bytes > MAX_IMAGE_BYTES {
            return Err(ParseError::LimitExceeded { offset, what: "cel image size" });
        }
        self.total += bytes;
        if self.total > MAX_TOTAL_DECOMPRESSED_BYTES {
            return Err(ParseError::LimitExceeded { offset, what: "total decompressed size" });
        }
        Ok(())
    }
}

/// Inflates one zlib stream that must decompress to exactly `expected` bytes.
pub fn inflate_exact(
    r: &mut Reader,
    chunk_end: usize,
    expected: usize,
    budget: &mut InflateBudget,
) -> Result<Vec<u8>> {
    let offset = r.pos();
    budget.charge(offset, expected)?;
    let compressed = r.bytes(chunk_end - offset)?;
    let out = decompress_to_vec_zlib_with_limit(compressed, expected)
        .map_err(|_| ParseError::Invalid { offset, what: "zlib stream" })?;
    if out.len() != expected {
        return Err(ParseError::Invalid { offset, what: "decompressed size" });
    }
    Ok(out)
}

/// Parses a cel chunk payload. `chunk_end` is the absolute end of the chunk —
/// compressed data runs to it (§6.3). Returns None for zero-sized images,
/// which Aseprite treats as "no cel".
pub fn parse_cel(
    r: &mut Reader,
    chunk_end: usize,
    depth: ColorDepth,
    num_layers: usize,
    budget: &mut InflateBudget,
) -> Result<Option<Cel>> {
    let start = r.pos();
    let layer_index = r.u16()? as usize;
    let x = r.i16()?;
    let y = r.i16()?;
    let opacity = r.u8()?;
    let cel_type = r.u16()?;
    let z_index = r.i16()?;
    r.skip(5)?; // reserved

    // A cel pointing past the layer list is invalid; skip it (gotcha #5).
    if layer_index >= num_layers {
        return Ok(None);
    }

    let content = match cel_type {
        0 => {
            // Raw image (legacy)
            let width = r.u16()?;
            let height = r.u16()?;
            if width == 0 || height == 0 {
                return Ok(None);
            }
            let len = width as usize * height as usize * depth.bytes_per_pixel();
            budget.charge(r.pos(), len)?;
            CelContent::Image(CelImage { width, height, pixels: r.bytes(len)?.to_vec() })
        }
        1 => CelContent::Linked(r.u16()?),
        2 => {
            let width = r.u16()?;
            let height = r.u16()?;
            if width == 0 || height == 0 {
                return Ok(None);
            }
            let len = width as usize * height as usize * depth.bytes_per_pixel();
            let pixels = inflate_exact(r, chunk_end, len, budget)?;
            CelContent::Image(CelImage { width, height, pixels })
        }
        3 => {
            let width = r.u16()?;
            let height = r.u16()?;
            let bits_offset = r.pos();
            let bits_per_tile = r.u16()?;
            // Only 32-bit tiles were ever shipped (gotcha #15).
            if bits_per_tile != 32 {
                return Err(ParseError::Invalid { offset: bits_offset, what: "bits per tile" });
            }
            let id_mask = r.u32()?;
            let x_mask = r.u32()?;
            let y_mask = r.u32()?;
            let d_mask = r.u32()?;
            r.skip(10)?;
            if width == 0 || height == 0 {
                return Ok(None);
            }
            let count = width as usize * height as usize;
            let raw = inflate_exact(r, chunk_end, count * 4, budget)?;
            let shift = id_mask.trailing_zeros();
            let tiles = raw
                .chunks_exact(4)
                .map(|c| {
                    let t = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                    Tile {
                        index: if id_mask == 0 { 0 } else { (t & id_mask) >> shift },
                        x_flip: x_mask != 0 && (t & x_mask) == x_mask,
                        y_flip: y_mask != 0 && (t & y_mask) == y_mask,
                        d_flip: d_mask != 0 && (t & d_mask) == d_mask,
                    }
                })
                .collect();
            CelContent::Tilemap(CelTilemap { width, height, tiles })
        }
        _ => return Err(ParseError::Invalid { offset: start, what: "cel type" }),
    };

    Ok(Some(Cel { layer_index, x, y, opacity, z_index, content, extra: None, user_data: Default::default() }))
}
