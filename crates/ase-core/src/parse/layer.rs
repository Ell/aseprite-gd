//! Layer chunk 0x2004 (§6.2).

use crate::error::ParseError;
use crate::model::{BlendMode, Layer, LayerType};
use crate::read::Reader;
use crate::Result;

pub fn parse_layer(r: &mut Reader, opacity_valid: bool, has_uuid: bool) -> Result<Layer> {
    let flags = r.u16()?;
    let type_offset = r.pos();
    let raw_type = r.u16()?;
    let child_level = r.u16()?;
    r.skip(4)?; // default width/height, ignored
    let blend_mode = BlendMode::from_u16(r.u16()?);
    let opacity = if opacity_valid { r.u8()? } else { r.skip(1).map(|_| 255)? };
    r.skip(3)?; // reserved
    let name = r.string()?;

    let layer_type = match raw_type {
        0 => LayerType::Image,
        1 => LayerType::Group,
        2 => LayerType::Tilemap { tileset_index: r.u32()? },
        _ => return Err(ParseError::Invalid { offset: type_offset, what: "layer type" }),
    };

    let uuid = if has_uuid {
        let mut u = [0u8; 16];
        u.copy_from_slice(r.bytes(16)?);
        Some(u)
    } else {
        None
    };

    Ok(Layer {
        flags,
        layer_type,
        child_level,
        blend_mode,
        opacity,
        name,
        parent: None, // derived by the caller from child levels
        uuid,
    })
}

/// Derives `parent` for a newly appended layer from child levels (§6.2):
/// the parent of a layer at level L is the nearest preceding layer at L-1.
pub fn resolve_parent(layers: &[Layer], child_level: u16) -> Option<usize> {
    if child_level == 0 {
        return None;
    }
    layers
        .iter()
        .rposition(|l| l.child_level == child_level - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer_bytes(flags: u16, ty: u16, level: u16, blend: u16, opacity: u8, name: &str) -> Vec<u8> {
        let mut b = vec![];
        b.extend_from_slice(&flags.to_le_bytes());
        b.extend_from_slice(&ty.to_le_bytes());
        b.extend_from_slice(&level.to_le_bytes());
        b.extend_from_slice(&[0u8; 4]); // default w/h
        b.extend_from_slice(&blend.to_le_bytes());
        b.push(opacity);
        b.extend_from_slice(&[0u8; 3]);
        b.extend_from_slice(&(name.len() as u16).to_le_bytes());
        b.extend_from_slice(name.as_bytes());
        b
    }

    #[test]
    fn parses_image_layer() {
        let bytes = layer_bytes(1 | 8, 0, 0, 1, 128, "bg");
        let l = parse_layer(&mut Reader::new(&bytes), true, false).unwrap();
        assert!(l.is_visible() && l.is_background());
        assert_eq!(l.layer_type, LayerType::Image);
        assert_eq!(l.blend_mode, BlendMode::Multiply);
        assert_eq!(l.opacity, 128);
        assert_eq!(l.name, "bg");
    }

    #[test]
    fn opacity_forced_when_header_flag_clear() {
        let bytes = layer_bytes(1, 0, 0, 0, 42, "x");
        let l = parse_layer(&mut Reader::new(&bytes), false, false).unwrap();
        assert_eq!(l.opacity, 255, "gotcha #4: opacity invalid without header flag");
    }

    #[test]
    fn tilemap_layer_reads_tileset_index() {
        let mut bytes = layer_bytes(1, 2, 0, 0, 255, "tm");
        bytes.extend_from_slice(&3u32.to_le_bytes());
        let l = parse_layer(&mut Reader::new(&bytes), true, false).unwrap();
        assert_eq!(l.layer_type, LayerType::Tilemap { tileset_index: 3 });
    }

    #[test]
    fn parent_resolution_follows_spec_example() {
        // The §6.2 example tree: levels 0,1,0,1,2,1
        let levels = [0u16, 1, 0, 1, 2, 1];
        let mut layers: Vec<Layer> = vec![];
        let mut parents = vec![];
        for &level in &levels {
            parents.push(resolve_parent(&layers, level));
            let bytes = layer_bytes(1, if level < 2 { 1 } else { 0 }, level, 0, 255, "l");
            let mut l = parse_layer(&mut Reader::new(&bytes), true, false).unwrap();
            l.parent = *parents.last().unwrap();
            layers.push(l);
        }
        assert_eq!(parents, vec![None, Some(0), None, Some(2), Some(3), Some(2)]);
    }
}
