//! Tileset chunk 0x2023 (§6.13).

use crate::error::ParseError;
use crate::model::{ColorDepth, Tileset};
use crate::parse::cel::{inflate_exact, InflateBudget};
use crate::read::Reader;
use crate::Result;

pub fn parse_tileset(
    r: &mut Reader,
    depth: ColorDepth,
    budget: &mut InflateBudget,
) -> Result<Tileset> {
    let start = r.pos();
    let id = r.u32()?;
    let flags = r.u32()?;
    let num_tiles = r.u32()?;
    let tile_width = r.u16()?;
    let tile_height = r.u16()?;
    if tile_width == 0 || tile_height == 0 {
        return Err(ParseError::Invalid { offset: start, what: "tileset tile size" });
    }
    let base_index = r.i16()?;
    r.skip(14)?;
    let name = r.string()?;

    let external = if flags & 1 != 0 {
        Some((r.u32()?, r.u32()?))
    } else {
        None
    };

    let pixels = if flags & 2 != 0 {
        let data_len = r.u32()? as usize;
        let data_end = r.pos() + data_len;
        // Strip is tile_width x (tile_height * num_tiles) (§6.13).
        let expected = tile_width as usize
            * tile_height as usize
            * num_tiles as usize
            * depth.bytes_per_pixel();
        Some(inflate_exact(r, data_end, expected, budget)?)
    } else {
        None
    };

    Ok(Tileset {
        id,
        flags,
        num_tiles,
        tile_width,
        tile_height,
        base_index,
        name,
        pixels,
        external,
        user_data: Default::default(),
        tile_user_data: Vec::new(),
    })
}
