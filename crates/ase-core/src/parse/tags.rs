//! Tags chunk 0x2018 (§6.9).

use crate::model::{AniDir, Tag};
use crate::read::Reader;
use crate::Result;

pub fn parse_tags(r: &mut Reader) -> Result<Vec<Tag>> {
    let n = r.u16()?;
    r.skip(8)?;
    let mut tags = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let from_frame = r.u16()?;
        let to_frame = r.u16()?;
        let direction = match r.u8()? {
            1 => AniDir::Reverse,
            2 => AniDir::PingPong,
            3 => AniDir::PingPongReverse,
            _ => AniDir::Forward, // out-of-range decodes as Forward (gotcha #18)
        };
        let repeat = r.u16()?;
        r.skip(6)?;
        let color = [r.u8()?, r.u8()?, r.u8()?];
        r.skip(1)?; // extra byte
        let name = r.string()?;
        tags.push(Tag { from_frame, to_frame, direction, repeat, color, name });
    }
    Ok(tags)
}
