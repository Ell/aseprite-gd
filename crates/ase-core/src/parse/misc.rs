//! Small chunks: cel extra 0x2006 (§6.4), color profile 0x2007 (§6.5),
//! external files 0x2008 (§6.6).

use crate::Result;
use crate::error::ParseError;
use crate::model::{CelExtra, ColorProfile, ExternalFile};
use crate::read::Reader;

/// Returns None when the "precise bounds" flag is unset or bounds are
/// degenerate (Aseprite ignores those).
pub fn parse_cel_extra(r: &mut Reader) -> Result<Option<CelExtra>> {
    let flags = r.u32()?;
    let x = r.fixed()?;
    let y = r.fixed()?;
    let width = r.fixed()?;
    let height = r.fixed()?;
    if flags & 1 == 0 || width == 0.0 || height == 0.0 {
        return Ok(None);
    }
    Ok(Some(CelExtra {
        x,
        y,
        width,
        height,
    }))
}

pub fn parse_color_profile(r: &mut Reader) -> Result<ColorProfile> {
    let start = r.pos();
    let kind = r.u16()?;
    let flags = r.u16()?;
    let gamma = r.fixed()?;
    r.skip(8)?;
    let fixed_gamma = (flags & 1 != 0).then_some(gamma);
    Ok(match kind {
        0 => ColorProfile::None { fixed_gamma },
        1 => ColorProfile::Srgb { fixed_gamma },
        2 => {
            let len = r.u32()? as usize;
            ColorProfile::Icc {
                fixed_gamma,
                data: r.bytes(len)?.to_vec(),
            }
        }
        _ => {
            return Err(ParseError::Invalid {
                offset: start,
                what: "color profile type",
            });
        }
    })
}

pub fn parse_external_files(r: &mut Reader) -> Result<Vec<ExternalFile>> {
    let n = r.u32()?;
    r.skip(8)?;
    if n as usize > r.remaining() / 12 {
        return Err(ParseError::Invalid {
            offset: r.pos(),
            what: "external file count",
        });
    }
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = r.u32()?;
        let kind = r.u8()?;
        r.skip(7)?;
        let name = r.string()?;
        out.push(ExternalFile { id, kind, name });
    }
    Ok(out)
}
