//! Bounded little-endian reader. Every read is bounds-checked; parse code never
//! indexes the input buffer directly. `unsafe` is forbidden in this crate's
//! parse paths (see AGENTS.md).

use crate::error::ParseError;
use crate::Result;

/// A cursor over the raw file bytes. Cheap to clone; offsets are absolute so
/// errors and chunk seeks always refer to real file positions.
#[derive(Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    /// Absolute offset of the next byte to be read.
    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Absolute seek. Seeking to `data.len()` (one past the end) is allowed;
    /// beyond that is an error. Chunk/frame traversal seeks by declared sizes
    /// rather than assuming fields were fully consumed (§2, gotcha #1).
    pub fn seek(&mut self, offset: usize) -> Result<()> {
        if offset > self.data.len() {
            return Err(ParseError::UnexpectedEof { offset: self.data.len(), needed: offset - self.data.len() });
        }
        self.pos = offset;
        Ok(())
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        let target = self.pos.checked_add(n).ok_or(ParseError::UnexpectedEof {
            offset: self.pos,
            needed: n,
        })?;
        self.seek(target)
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.remaining() < n {
            return Err(ParseError::UnexpectedEof { offset: self.pos, needed: n - self.remaining() });
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    pub fn u64(&mut self) -> Result<u64> {
        let b = self.bytes(8)?;
        Ok(u64::from_le_bytes(b.try_into().expect("length checked")))
    }

    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// FIXED: signed 16.16 fixed point (§1).
    pub fn fixed(&mut self) -> Result<f64> {
        Ok(self.i32()? as f64 / 65_536.0)
    }

    /// STRING: WORD byte length + UTF-8 bytes, no NUL terminator (§1, gotcha #11).
    pub fn string(&mut self) -> Result<String> {
        let start = self.pos;
        let len = self.u16()? as usize;
        let bytes = self.bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ParseError::BadString { offset: start })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_le_primitives() {
        let mut r = Reader::new(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(r.u16().unwrap(), 0x0201);
        assert_eq!(r.u16().unwrap(), 0x0403);
        assert_eq!(r.pos(), 4);
        assert_eq!(r.u8().unwrap(), 0x05);
        assert!(matches!(r.u8(), Err(ParseError::UnexpectedEof { offset: 5, needed: 1 })));
    }

    #[test]
    fn string_is_length_prefixed_utf8() {
        // "héllo" — 6 bytes of UTF-8, length prefix counts bytes not chars.
        let mut data = vec![0x06, 0x00];
        data.extend_from_slice("héllo".as_bytes());
        let mut r = Reader::new(&data);
        assert_eq!(r.string().unwrap(), "héllo");
    }

    #[test]
    fn string_rejects_bad_utf8_with_field_offset() {
        let mut r = Reader::new(&[0x02, 0x00, 0xFF, 0xFE]);
        assert_eq!(r.string(), Err(ParseError::BadString { offset: 0 }));
    }

    #[test]
    fn seek_past_end_fails() {
        let mut r = Reader::new(&[0u8; 4]);
        assert!(r.seek(4).is_ok()); // one past the end is fine (empty tail)
        assert!(r.seek(5).is_err());
    }
}
