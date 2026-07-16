//! Slice chunk 0x2022 (§6.12).

use crate::Result;
use crate::error::ParseError;
use crate::model::{Slice, SliceKey};
use crate::read::Reader;

pub fn parse_slice(r: &mut Reader) -> Result<Slice> {
    let start = r.pos();
    let n_keys = r.u32()?;
    let flags = r.u32()?;
    r.skip(4)?; // reserved
    let name = r.string()?;

    // Each key is at least 20 bytes; bound n_keys by what could possibly fit.
    if n_keys as usize > r.remaining() / 20 {
        return Err(ParseError::Invalid {
            offset: start,
            what: "slice key count",
        });
    }

    let mut keys = Vec::with_capacity(n_keys as usize);
    for _ in 0..n_keys {
        let frame = r.u32()?;
        let x = r.i32()?;
        let y = r.i32()?;
        let width = r.u32()?;
        let height = r.u32()?;
        let center = if flags & 1 != 0 {
            Some((r.i32()?, r.i32()?, r.u32()?, r.u32()?))
        } else {
            None
        };
        let pivot = if flags & 2 != 0 {
            Some((r.i32()?, r.i32()?))
        } else {
            None
        };
        keys.push(SliceKey {
            frame,
            x,
            y,
            width,
            height,
            center,
            pivot,
        });
    }
    Ok(Slice {
        name,
        keys,
        user_data: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nine_patch_with_pivot() {
        let mut b = vec![];
        b.extend_from_slice(&1u32.to_le_bytes()); // one key
        b.extend_from_slice(&3u32.to_le_bytes()); // 9-patch + pivot
        b.extend_from_slice(&[0u8; 4]);
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(b"ui");
        for v in [0i32, -1, 2] {
            b.extend_from_slice(&v.to_le_bytes()); // frame(=0 via u32), x, y
        }
        b.extend_from_slice(&16u32.to_le_bytes()); // w
        b.extend_from_slice(&8u32.to_le_bytes()); // h
        for v in [4i32, 2] {
            b.extend_from_slice(&v.to_le_bytes()); // center x,y
        }
        b.extend_from_slice(&8u32.to_le_bytes()); // center w
        b.extend_from_slice(&4u32.to_le_bytes()); // center h
        b.extend_from_slice(&3i32.to_le_bytes()); // pivot x
        b.extend_from_slice(&5i32.to_le_bytes()); // pivot y

        let s = parse_slice(&mut Reader::new(&b)).unwrap();
        assert_eq!(s.name, "ui");
        let k = s.key_for(7).unwrap();
        assert_eq!((k.x, k.y, k.width, k.height), (-1, 2, 16, 8));
        assert_eq!(k.center, Some((4, 2, 8, 4)));
        assert_eq!(k.pivot, Some((3, 5)));
        assert!(s.key_for(0).is_some());
    }
}
