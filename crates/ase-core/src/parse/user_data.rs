//! User data chunk 0x2020 (§6.11): text, color, and the v1.3 typed
//! properties maps. Association to objects happens in `file.rs` (gotcha #17).

use crate::Result;
use crate::error::ParseError;
use crate::limits::MAX_PROPERTY_DEPTH;
use crate::model::{Properties, PropertiesMap, PropertyValue, UserData};
use crate::read::Reader;

pub fn parse_user_data(r: &mut Reader) -> Result<UserData> {
    let flags = r.u32()?;
    let text = if flags & 1 != 0 {
        Some(r.string()?)
    } else {
        None
    };
    let color = if flags & 2 != 0 {
        Some([r.u8()?, r.u8()?, r.u8()?, r.u8()?])
    } else {
        None
    };

    let mut maps = Vec::new();
    if flags & 4 != 0 {
        let props_start = r.pos();
        let total = r.u32()? as usize;
        if total < 8 {
            return Err(ParseError::Invalid {
                offset: props_start,
                what: "properties size",
            });
        }
        let end = props_start + total;
        // The blob size is stored precisely so readers can survive unknown
        // property types: parse what we can, then always seek to the end
        // (§6.11, gotcha #22).
        maps = parse_maps(r, end).unwrap_or_default();
        r.seek(end)?;
    }

    Ok(UserData { text, color, maps })
}

fn parse_maps(r: &mut Reader, end: usize) -> Result<Vec<PropertiesMap>> {
    let n_maps = r.u32()?;
    let mut maps = Vec::new();
    for _ in 0..n_maps {
        if r.pos() >= end {
            return Err(ParseError::Invalid {
                offset: r.pos(),
                what: "properties map count",
            });
        }
        let key = r.u32()?;
        let properties = parse_properties(r, 0)?;
        maps.push(PropertiesMap { key, properties });
    }
    Ok(maps)
}

fn parse_properties(r: &mut Reader, depth: u32) -> Result<Properties> {
    if depth > MAX_PROPERTY_DEPTH {
        return Err(ParseError::LimitExceeded {
            offset: r.pos(),
            what: "property nesting",
        });
    }
    let n = r.u32()?;
    // Each property is at least name(2) + type(2) + 1 byte of value.
    if n as usize > r.remaining() / 5 {
        return Err(ParseError::Invalid {
            offset: r.pos(),
            what: "property count",
        });
    }
    let mut props = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let name = r.string()?;
        let ty = r.u16()?;
        props.push((name, parse_value(r, ty, depth)?));
    }
    Ok(props)
}

fn parse_value(r: &mut Reader, ty: u16, depth: u32) -> Result<PropertyValue> {
    use PropertyValue::*;
    Ok(match ty {
        0x0001 => Bool(r.u8()? != 0),
        0x0002 => I8(r.u8()? as i8),
        0x0003 => U8(r.u8()?),
        0x0004 => I16(r.i16()?),
        0x0005 => U16(r.u16()?),
        0x0006 => I32(r.i32()?),
        0x0007 => U32(r.u32()?),
        0x0008 => I64(r.u64()? as i64),
        0x0009 => U64(r.u64()?),
        0x000A => Fixed(r.fixed()?),
        0x000B => F32(r.f32()?),
        0x000C => F64(r.f64()?),
        0x000D => Str(r.string()?),
        0x000E => Point(r.i32()?, r.i32()?),
        0x000F => Size(r.i32()?, r.i32()?),
        0x0010 => Rect(r.i32()?, r.i32()?, r.i32()?, r.i32()?),
        0x0011 => {
            let n = r.u32()?;
            let elems_type = r.u16()?;
            if n as usize > r.remaining() {
                return Err(ParseError::Invalid {
                    offset: r.pos(),
                    what: "vector length",
                });
            }
            let mut v = Vec::with_capacity(n as usize);
            for _ in 0..n {
                // elemsType 0 = heterogeneous: each element carries its type.
                let ty = if elems_type == 0 {
                    r.u16()?
                } else {
                    elems_type
                };
                v.push(parse_value(r, ty, depth + 1)?);
            }
            Vector(v)
        }
        0x0012 => Map(parse_properties(r, depth + 1)?),
        0x0013 => {
            let mut u = [0u8; 16];
            u.copy_from_slice(r.bytes(16)?);
            Uuid(u)
        }
        // 0x0000 (nullptr) must not appear in files; anything else is a
        // future type we can't skip (unknown width) — abort the blob (§6.11).
        _ => {
            return Err(ParseError::Invalid {
                offset: r.pos(),
                what: "property type",
            });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn str_bytes(s: &str) -> Vec<u8> {
        let mut b = (s.len() as u16).to_le_bytes().to_vec();
        b.extend_from_slice(s.as_bytes());
        b
    }

    #[test]
    fn parses_text_and_color() {
        let mut b = 3u32.to_le_bytes().to_vec(); // flags: text + color
        b.extend_from_slice(&str_bytes("hit"));
        b.extend_from_slice(&[255, 0, 0, 255]);
        let ud = parse_user_data(&mut Reader::new(&b)).unwrap();
        assert_eq!(ud.text.as_deref(), Some("hit"));
        assert_eq!(ud.color, Some([255, 0, 0, 255]));
        assert!(ud.maps.is_empty());
    }

    #[test]
    fn parses_nested_properties() {
        // flags=4; one map (key 0) with: "n" = i32 7, "v" = vector[u8 1, u8 2],
        // "m" = map{"s" = "x"}
        let mut body = vec![];
        body.extend_from_slice(&1u32.to_le_bytes()); // nMaps
        body.extend_from_slice(&0u32.to_le_bytes()); // map key
        body.extend_from_slice(&3u32.to_le_bytes()); // nProps
        body.extend_from_slice(&str_bytes("n"));
        body.extend_from_slice(&0x0006u16.to_le_bytes());
        body.extend_from_slice(&7i32.to_le_bytes());
        body.extend_from_slice(&str_bytes("v"));
        body.extend_from_slice(&0x0011u16.to_le_bytes());
        body.extend_from_slice(&2u32.to_le_bytes()); // nElems
        body.extend_from_slice(&0x0003u16.to_le_bytes()); // homogeneous u8
        body.extend_from_slice(&[1, 2]);
        body.extend_from_slice(&str_bytes("m"));
        body.extend_from_slice(&0x0012u16.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes()); // nested nProps
        body.extend_from_slice(&str_bytes("s"));
        body.extend_from_slice(&0x000Du16.to_le_bytes());
        body.extend_from_slice(&str_bytes("x"));

        let mut b = 4u32.to_le_bytes().to_vec();
        b.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes()); // totalSize
        b.extend_from_slice(&body);

        let ud = parse_user_data(&mut Reader::new(&b)).unwrap();
        let props = &ud.maps[0].properties;
        assert_eq!(props[0], ("n".into(), PropertyValue::I32(7)));
        assert_eq!(
            props[1],
            (
                "v".into(),
                PropertyValue::Vector(vec![PropertyValue::U8(1), PropertyValue::U8(2)])
            )
        );
        assert_eq!(
            props[2],
            (
                "m".into(),
                PropertyValue::Map(vec![("s".into(), PropertyValue::Str("x".into()))])
            )
        );
    }

    #[test]
    fn unknown_property_type_drops_maps_but_survives() {
        // flags=4, totalSize covers a blob whose property type is bogus.
        let mut body = vec![];
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&str_bytes("x"));
        body.extend_from_slice(&0x00FFu16.to_le_bytes()); // unknown type
        body.extend_from_slice(&[0xAA; 4]);

        let mut b = 4u32.to_le_bytes().to_vec();
        b.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
        b.extend_from_slice(&body);
        b.extend_from_slice(&[0xEE]); // trailing byte after the blob

        let mut r = Reader::new(&b);
        let ud = parse_user_data(&mut r).unwrap();
        assert!(ud.maps.is_empty(), "unparseable maps dropped");
        assert_eq!(r.u8().unwrap(), 0xEE, "reader seeked past the blob");
    }
}
