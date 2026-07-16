//! Chunk envelope (§5). Traversal always seeks to `offset + size` afterward —
//! unknown chunk types and unread trailing fields are skipped by construction
//! (gotchas #1, #23).

use crate::Result;
use crate::error::ParseError;
use crate::read::Reader;

/// Known chunk type IDs (§5).
pub mod types {
    pub const OLD_PALETTE_8: u16 = 0x0004;
    pub const OLD_PALETTE_6: u16 = 0x0011;
    pub const LAYER: u16 = 0x2004;
    pub const CEL: u16 = 0x2005;
    pub const CEL_EXTRA: u16 = 0x2006;
    pub const COLOR_PROFILE: u16 = 0x2007;
    pub const EXTERNAL_FILES: u16 = 0x2008;
    pub const MASK: u16 = 0x2016; // deprecated
    pub const PATH: u16 = 0x2017; // never used
    pub const TAGS: u16 = 0x2018;
    pub const PALETTE: u16 = 0x2019;
    pub const USER_DATA: u16 = 0x2020;
    pub const SLICE: u16 = 0x2022;
    pub const TILESET: u16 = 0x2023;
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkHeader {
    /// Absolute offset of the chunk (start of its size field).
    pub offset: usize,
    /// Total chunk size including the 6 header bytes.
    pub size: u32,
    pub kind: u16,
}

impl ChunkHeader {
    /// Absolute offset one past the chunk's last byte.
    pub fn end(&self) -> usize {
        self.offset + self.size as usize
    }
}

/// Reads a chunk header and validates it fits inside `frame_end` (absolute).
/// Leaves the reader at the chunk payload.
pub fn parse_chunk_header(r: &mut Reader, frame_end: usize) -> Result<ChunkHeader> {
    let offset = r.pos();
    let size = r.u32()?;
    let kind = r.u16()?;
    if size < 6 {
        return Err(ParseError::Invalid {
            offset,
            what: "chunk size (< 6 bytes)",
        });
    }
    let end = offset
        .checked_add(size as usize)
        .ok_or(ParseError::Invalid {
            offset,
            what: "chunk size (overflow)",
        })?;
    if end > frame_end {
        return Err(ParseError::Invalid {
            offset,
            what: "chunk size (overruns frame)",
        });
    }
    Ok(ChunkHeader { offset, size, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_bounds_checks() {
        let mut data = vec![];
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&types::LAYER.to_le_bytes());
        data.extend_from_slice(&[0u8; 4]);
        let mut r = Reader::new(&data);
        let h = parse_chunk_header(&mut r, data.len()).unwrap();
        assert_eq!((h.offset, h.size, h.kind), (0, 10, types::LAYER));
        assert_eq!(h.end(), 10);
    }

    #[test]
    fn rejects_overrun_and_undersize() {
        let mut data = vec![];
        data.extend_from_slice(&100u32.to_le_bytes());
        data.extend_from_slice(&types::CEL.to_le_bytes());
        let mut r = Reader::new(&data);
        assert!(parse_chunk_header(&mut r, 6).is_err());

        let mut data = vec![];
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(&types::CEL.to_le_bytes());
        let mut r = Reader::new(&data);
        assert!(parse_chunk_header(&mut r, 6).is_err());
    }
}
