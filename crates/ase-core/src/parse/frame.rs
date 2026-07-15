//! Frame header (§4 of docs/ase-format-reference.md).

use crate::error::ParseError;
use crate::model::FrameHeader;
use crate::parse::FRAME_MAGIC;
use crate::read::Reader;
use crate::Result;

/// Parses a 16-byte frame header, resolving the old/new chunk-count rule:
/// the old WORD saturates at 0xFFFF, in which case the newer DWORD field holds
/// the real count (gotcha #3). Leaves the reader at the first chunk.
pub fn parse_frame_header(r: &mut Reader) -> Result<FrameHeader> {
    let frame_start = r.pos();
    let frame_bytes = r.u32()?;

    let magic_offset = r.pos();
    let magic = r.u16()?;
    if magic != FRAME_MAGIC {
        return Err(ParseError::BadMagic { offset: magic_offset, expected: FRAME_MAGIC, found: magic });
    }

    let old_chunks = r.u16()?;
    let duration_ms = r.u16()?;
    r.skip(2)?; // reserved
    let new_chunks = r.u32()?;

    let num_chunks = if old_chunks == 0xFFFF && new_chunks != 0 {
        new_chunks
    } else {
        u32::from(old_chunks)
    };

    if (frame_bytes as usize) < r.pos() - frame_start {
        return Err(ParseError::Invalid { offset: frame_start, what: "frame size (smaller than frame header)" });
    }

    Ok(FrameHeader { frame_bytes, num_chunks, duration_ms })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::FRAME_HEADER_SIZE;

    fn frame_bytes(old: u16, new: u32, duration: u16) -> Vec<u8> {
        let mut f = vec![0u8; FRAME_HEADER_SIZE];
        f[0..4].copy_from_slice(&(FRAME_HEADER_SIZE as u32).to_le_bytes());
        f[4..6].copy_from_slice(&FRAME_MAGIC.to_le_bytes());
        f[6..8].copy_from_slice(&old.to_le_bytes());
        f[8..10].copy_from_slice(&duration.to_le_bytes());
        f[12..16].copy_from_slice(&new.to_le_bytes());
        f
    }

    #[test]
    fn uses_old_count_normally() {
        let bytes = frame_bytes(5, 5, 100);
        let fh = parse_frame_header(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(fh.num_chunks, 5);
        assert_eq!(fh.duration_ms, 100);
    }

    #[test]
    fn overflow_defers_to_new_count() {
        let bytes = frame_bytes(0xFFFF, 70_000, 100);
        let fh = parse_frame_header(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(fh.num_chunks, 70_000);
    }

    #[test]
    fn overflow_with_zero_new_count_keeps_old() {
        // Degenerate old-file case: 0xFFFF chunks written before the new field existed.
        let bytes = frame_bytes(0xFFFF, 0, 100);
        let fh = parse_frame_header(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(fh.num_chunks, 0xFFFF);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = frame_bytes(1, 1, 50);
        bytes[4..6].copy_from_slice(&0xBEEFu16.to_le_bytes());
        assert!(matches!(
            parse_frame_header(&mut Reader::new(&bytes)),
            Err(ParseError::BadMagic { offset: 4, .. })
        ));
    }
}
