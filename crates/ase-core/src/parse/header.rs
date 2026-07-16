//! File header (§3 of docs/ase-format-reference.md).

use crate::Result;
use crate::error::ParseError;
use crate::limits::MAX_CANVAS_DIM;
use crate::model::{ColorDepth, Header};
use crate::parse::{FILE_MAGIC, HEADER_SIZE};
use crate::read::Reader;

/// Parses the 128-byte header and leaves the reader positioned at offset 128
/// (start of the first frame), regardless of reserved trailing bytes.
pub fn parse_header(r: &mut Reader) -> Result<Header> {
    let _file_size = r.u32()?; // advisory; never trusted over actual buffer length

    let magic_offset = r.pos();
    let magic = r.u16()?;
    if magic != FILE_MAGIC {
        return Err(ParseError::BadMagic {
            offset: magic_offset,
            expected: FILE_MAGIC,
            found: magic,
        });
    }

    let frames = r.u16()?;
    if frames == 0 {
        return Err(ParseError::Invalid {
            offset: 6,
            what: "frame count (zero)",
        });
    }
    let width = r.u16()?;
    let height = r.u16()?;
    if width == 0 || height == 0 {
        return Err(ParseError::Invalid {
            offset: 8,
            what: "canvas size (zero dimension)",
        });
    }
    if u32::from(width) > MAX_CANVAS_DIM || u32::from(height) > MAX_CANVAS_DIM {
        return Err(ParseError::LimitExceeded {
            offset: 8,
            what: "canvas size",
        });
    }

    let depth_offset = r.pos();
    let bpp = r.u16()?;
    let color_depth = ColorDepth::from_bpp(bpp).ok_or(ParseError::Invalid {
        offset: depth_offset,
        what: "color depth (expected 8/16/32 bpp)",
    })?;

    let flags = r.u32()?;
    let default_frame_duration_ms = r.u16()?;
    r.skip(8)?; // two DWORDs, set to 0

    let mut transparent_index = r.u8()?;
    if color_depth != ColorDepth::Indexed {
        // Aseprite forces this to 0 for non-indexed sprites (gotcha #7).
        transparent_index = 0;
    }
    r.skip(3)?; // ignored bytes

    let mut num_colors = r.u16()?;
    if num_colors == 0 {
        num_colors = 256; // old files (gotcha #8)
    }

    let pixel_width = r.u8()?;
    let pixel_height = r.u8()?;
    let pixel_ratio = if pixel_width == 0 || pixel_height == 0 {
        (1, 1) // gotcha #8
    } else {
        (pixel_width, pixel_height)
    };

    let grid_x = r.i16()?;
    let grid_y = r.i16()?;
    let grid_width = r.u16()?;
    let grid_height = r.u16()?;

    // Reserved tail: always seek, never assume consumption (§3, gotcha #1).
    r.seek(HEADER_SIZE)?;

    Ok(Header {
        frames,
        width,
        height,
        color_depth,
        flags,
        default_frame_duration_ms,
        transparent_index,
        num_colors,
        pixel_ratio,
        grid_x,
        grid_y,
        grid_width,
        grid_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal valid 128-byte header. Field offsets per §3.
    fn header_bytes() -> Vec<u8> {
        let mut h = vec![0u8; HEADER_SIZE];
        h[0..4].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes()); // file size
        h[4..6].copy_from_slice(&FILE_MAGIC.to_le_bytes());
        h[6..8].copy_from_slice(&3u16.to_le_bytes()); // frames
        h[8..10].copy_from_slice(&64u16.to_le_bytes()); // width
        h[10..12].copy_from_slice(&32u16.to_le_bytes()); // height
        h[12..14].copy_from_slice(&32u16.to_le_bytes()); // bpp RGBA
        h[14..18].copy_from_slice(&1u32.to_le_bytes()); // flags: layer opacity valid
        h[18..20].copy_from_slice(&100u16.to_le_bytes()); // speed
        h[28] = 7; // transparent index (should be zeroed: not indexed)
        h[32..34].copy_from_slice(&0u16.to_le_bytes()); // ncolors 0 => 256
        // pixel width/height left 0 => ratio 1:1
        h
    }

    #[test]
    fn parses_and_normalizes() {
        let bytes = header_bytes();
        let mut r = Reader::new(&bytes);
        let h = parse_header(&mut r).unwrap();
        assert_eq!(h.frames, 3);
        assert_eq!((h.width, h.height), (64, 32));
        assert_eq!(h.color_depth, ColorDepth::Rgba);
        assert!(h.layer_opacity_valid());
        assert!(!h.group_blend_valid());
        assert_eq!(
            h.transparent_index, 0,
            "transparent index zeroed for non-indexed"
        );
        assert_eq!(h.num_colors, 256, "0 colors means 256");
        assert_eq!(h.pixel_ratio, (1, 1));
        assert_eq!(r.pos(), HEADER_SIZE, "reader left at first frame");
    }

    #[test]
    fn keeps_transparent_index_for_indexed() {
        let mut bytes = header_bytes();
        bytes[12..14].copy_from_slice(&8u16.to_le_bytes()); // indexed
        let h = parse_header(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(h.color_depth, ColorDepth::Indexed);
        assert_eq!(h.transparent_index, 7);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = header_bytes();
        bytes[4..6].copy_from_slice(&0xA5E1u16.to_le_bytes());
        let err = parse_header(&mut Reader::new(&bytes)).unwrap_err();
        assert_eq!(
            err,
            ParseError::BadMagic {
                offset: 4,
                expected: FILE_MAGIC,
                found: 0xA5E1
            }
        );
    }

    #[test]
    fn rejects_unknown_depth() {
        let mut bytes = header_bytes();
        bytes[12..14].copy_from_slice(&24u16.to_le_bytes());
        assert!(matches!(
            parse_header(&mut Reader::new(&bytes)),
            Err(ParseError::Invalid { offset: 12, .. })
        ));
    }

    #[test]
    fn rejects_truncated_header() {
        let bytes = header_bytes();
        let mut r = Reader::new(&bytes[..100]);
        assert!(matches!(
            parse_header(&mut r),
            Err(ParseError::UnexpectedEof { .. })
        ));
    }
}
