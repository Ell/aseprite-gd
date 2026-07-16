//! Palette chunks: new 0x2019 (§6.10) and old 0x0004/0x0011 (§6.1).

use crate::Result;
use crate::error::ParseError;
use crate::limits::MAX_PALETTE_ENTRIES;
use crate::model::Palette;
use crate::read::Reader;

/// Applies a new palette chunk (0x2019) as a delta onto `pal`.
pub fn apply_new_palette(r: &mut Reader, pal: &mut Palette) -> Result<()> {
    let start = r.pos();
    let new_size = r.u32()?;
    let from = r.u32()?;
    let to = r.u32()?;
    if new_size > MAX_PALETTE_ENTRIES {
        return Err(ParseError::LimitExceeded {
            offset: start,
            what: "palette size",
        });
    }
    if to < from || to >= new_size {
        return Err(ParseError::Invalid {
            offset: start,
            what: "palette range",
        });
    }
    r.skip(8)?;
    pal.entries.resize(new_size as usize, [0, 0, 0, 255]);
    for i in from..=to {
        let flags = r.u16()?;
        let rgba = [r.u8()?, r.u8()?, r.u8()?, r.u8()?];
        pal.entries[i as usize] = rgba;
        if flags & 1 != 0 {
            let _name = r.string()?; // Aseprite ignores names on load; so do we
        }
    }
    Ok(())
}

/// Applies an old palette chunk (0x0004 or 0x0011) onto `pal`.
/// `six_bit` selects 0x0011's 0-63 component range (scaled up per gotcha #10).
pub fn apply_old_palette(r: &mut Reader, pal: &mut Palette, six_bit: bool) -> Result<()> {
    let n_packets = r.u16()?;
    let mut index: usize = 0;
    for _ in 0..n_packets {
        index += r.u8()? as usize; // skip count, cumulative
        let count = match r.u8()? {
            0 => 256usize, // 0 means 256 (§6.1)
            n => n as usize,
        };
        let end = index + count;
        if end > MAX_PALETTE_ENTRIES as usize {
            return Err(ParseError::LimitExceeded {
                offset: r.pos(),
                what: "palette size",
            });
        }
        if pal.entries.len() < end {
            pal.entries.resize(end, [0, 0, 0, 255]);
        }
        for _ in 0..count {
            let scale = |v: u8| if six_bit { (v << 2) | (v >> 4) } else { v };
            let (rr, g, b) = (scale(r.u8()?), scale(r.u8()?), scale(r.u8()?));
            pal.entries[index] = [rr, g, b, 255];
            index += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_palette_applies_delta_range() {
        // size 4, replace entries 1..=2
        let mut data = vec![];
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        data.extend_from_slice(&[0, 0, 10, 20, 30, 40]); // entry 1, no name
        data.extend_from_slice(&[0, 0, 50, 60, 70, 80]); // entry 2
        let mut pal = Palette {
            entries: vec![[1, 1, 1, 255]; 3],
        };
        apply_new_palette(&mut Reader::new(&data), &mut pal).unwrap();
        assert_eq!(pal.entries.len(), 4);
        assert_eq!(
            pal.entries[0],
            [1, 1, 1, 255],
            "outside delta range untouched"
        );
        assert_eq!(pal.entries[1], [10, 20, 30, 40]);
        assert_eq!(pal.entries[2], [50, 60, 70, 80]);
    }

    #[test]
    fn old_palette_scales_six_bit_components() {
        // 1 packet: skip 0, count 1, color (63, 0, 32)
        let data = [1u8, 0, 0, 1, 63, 0, 32];
        let mut pal = Palette::default();
        apply_old_palette(&mut Reader::new(&data[..2]), &mut pal, true).unwrap_err();
        let mut pal = Palette::default();
        apply_old_palette(&mut Reader::new(&data), &mut pal, true).unwrap();
        assert_eq!(pal.entries[0], [255, 0, 130, 255]); // (63<<2)|(63>>4)=255, (32<<2)|(32>>4)=130
    }
}
