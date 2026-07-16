//! Whole-file parsing: assembles the document model by walking frames and
//! dispatching chunks (§2). Unknown chunk types are skipped (gotcha #23).

use crate::Result;
use crate::model::{
    ColorProfile, ExternalFile, Frame, Header, Layer, Palette, Slice, Tag, Tileset, UserData,
};
use crate::parse::cel::InflateBudget;
use crate::parse::chunk::{parse_chunk_header, types};
use crate::parse::layer::resolve_parent;
use crate::parse::{self, HEADER_SIZE};
use crate::read::Reader;

/// A fully parsed Aseprite document.
#[derive(Debug, Clone)]
pub struct AseFile {
    pub header: Header,
    /// Flat, in file order — the index is the layer index cels reference.
    pub layers: Vec<Layer>,
    pub frames: Vec<Frame>,
    pub tags: Vec<Tag>,
    pub tilesets: Vec<Tileset>,
    pub slices: Vec<Slice>,
    /// Per-frame palette snapshots — a palette chunk in frame N changes the
    /// palette from frame N onward (§6.10). Same length as `frames`.
    pub palettes: Vec<Palette>,
    /// Sprite-level user data (§6.11): a user data chunk read while no other
    /// object has appeared yet.
    pub user_data: UserData,
    pub color_profile: Option<ColorProfile>,
    pub external_files: Vec<ExternalFile>,
}

/// What the next user data chunk attaches to (§6.11, gotcha #17). Palette,
/// old-palette, color-profile, and unknown chunks do NOT change this state.
enum UdTarget {
    Sprite,
    Layer(usize),
    /// Index into the current frame's cel vec.
    Cel(usize),
    Slice(usize),
    /// Next tag index to receive user data, and the end of the tag run.
    Tags {
        next: usize,
        end: usize,
    },
    /// Tileset itself first, then per-tile user data in tile order.
    Tileset {
        index: usize,
        next_tile: Option<usize>,
    },
    /// Preceding object was skipped/invalid: drop user data.
    None,
}

impl AseFile {
    pub fn parse(data: &[u8]) -> Result<AseFile> {
        let mut r = Reader::new(data);
        let header = parse::parse_header(&mut r)?;

        let mut layers: Vec<Layer> = Vec::new();
        let mut frames: Vec<Frame> = Vec::with_capacity(header.frames as usize);
        let mut tags: Vec<Tag> = Vec::new();
        let mut tilesets: Vec<Tileset> = Vec::new();
        let mut slices: Vec<Slice> = Vec::new();
        let mut palettes: Vec<Palette> = Vec::with_capacity(header.frames as usize);
        let mut palette = Palette::default();
        let mut sprite_user_data = UserData::default();
        let mut color_profile = None;
        let mut external_files = Vec::new();
        let mut budget = InflateBudget::default();
        // Old palette chunks are ignored once a 0x2019 has been seen (gotcha #10).
        let mut seen_new_palette = false;
        // "Last object" starts as the sprite itself (§6.11).
        let mut ud_target = UdTarget::Sprite;

        let mut frame_start = HEADER_SIZE;
        for _ in 0..header.frames {
            r.seek(frame_start)?;
            let fh = parse::parse_frame_header(&mut r)?;
            let frame_end = frame_start + fh.frame_bytes as usize;
            let mut cels = Vec::new();
            let mut last_cel: Option<usize> = None;

            for _ in 0..fh.num_chunks {
                let ch = parse_chunk_header(&mut r, frame_end)?;
                match ch.kind {
                    types::LAYER => {
                        let mut layer = parse::parse_layer(
                            &mut r,
                            header.layer_opacity_valid(),
                            header.layers_have_uuid(),
                        )?;
                        layer.parent = resolve_parent(&layers, layer.child_level);
                        layers.push(layer);
                        ud_target = UdTarget::Layer(layers.len() - 1);
                    }
                    types::CEL => {
                        match parse::parse_cel(
                            &mut r,
                            ch.end(),
                            header.color_depth,
                            layers.len(),
                            &mut budget,
                        )? {
                            Some(cel) => {
                                cels.push(cel);
                                last_cel = Some(cels.len() - 1);
                                ud_target = UdTarget::Cel(cels.len() - 1);
                            }
                            // Skipped cel: orphan its extra/user data (gotcha #5).
                            None => {
                                last_cel = None;
                                ud_target = UdTarget::None;
                            }
                        }
                    }
                    types::CEL_EXTRA => {
                        if let Some(i) = last_cel {
                            cels[i].extra = parse::parse_cel_extra(&mut r)?;
                        }
                    }
                    types::TAGS => {
                        let start = tags.len();
                        tags.extend(parse::parse_tags(&mut r)?);
                        // The next N user data chunks belong to these tags in
                        // order (§6.11).
                        ud_target = UdTarget::Tags {
                            next: start,
                            end: tags.len(),
                        };
                    }
                    types::PALETTE => {
                        seen_new_palette = true;
                        parse::apply_new_palette(&mut r, &mut palette)?;
                    }
                    types::OLD_PALETTE_8 if !seen_new_palette => {
                        parse::apply_old_palette(&mut r, &mut palette, false)?;
                    }
                    types::OLD_PALETTE_6 if !seen_new_palette => {
                        parse::apply_old_palette(&mut r, &mut palette, true)?;
                    }
                    types::TILESET => {
                        tilesets.push(parse::parse_tileset(
                            &mut r,
                            header.color_depth,
                            &mut budget,
                        )?);
                        ud_target = UdTarget::Tileset {
                            index: tilesets.len() - 1,
                            next_tile: None,
                        };
                    }
                    types::SLICE => {
                        slices.push(parse::parse_slice(&mut r)?);
                        ud_target = UdTarget::Slice(slices.len() - 1);
                    }
                    types::COLOR_PROFILE => {
                        color_profile = Some(parse::parse_color_profile(&mut r)?)
                    }
                    types::EXTERNAL_FILES => external_files = parse::parse_external_files(&mut r)?,
                    types::USER_DATA => {
                        let ud = parse::parse_user_data(&mut r)?;
                        match &mut ud_target {
                            UdTarget::Sprite => sprite_user_data = ud,
                            UdTarget::Layer(i) => layers[*i].user_data = ud,
                            UdTarget::Cel(i) => cels[*i].user_data = ud,
                            UdTarget::Slice(i) => slices[*i].user_data = ud,
                            UdTarget::Tags { next, end } => {
                                tags[*next].user_data = ud;
                                *next += 1;
                                if next == end {
                                    ud_target = UdTarget::None;
                                }
                            }
                            UdTarget::Tileset { index, next_tile } => {
                                let ts = &mut tilesets[*index];
                                match next_tile {
                                    None => {
                                        ts.user_data = ud;
                                        ts.tile_user_data =
                                            vec![UserData::default(); ts.num_tiles as usize];
                                        *next_tile = Some(0);
                                    }
                                    Some(k) if *k < ts.tile_user_data.len() => {
                                        ts.tile_user_data[*k] = ud;
                                        *next_tile = Some(*k + 1);
                                    }
                                    Some(_) => ud_target = UdTarget::None,
                                }
                            }
                            UdTarget::None => {}
                        }
                    }
                    // Deprecated (mask, path) and unrecognized chunks: skipped;
                    // they do NOT reset the user-data target (gotcha #17).
                    _ => {}
                }
                r.seek(ch.end())?;
            }

            let duration_ms = if fh.duration_ms == 0 {
                header.default_frame_duration_ms
            } else {
                fh.duration_ms
            };
            frames.push(Frame { duration_ms, cels });
            palettes.push(palette.clone());
            frame_start = frame_end;
        }

        Ok(AseFile {
            header,
            layers,
            frames,
            tags,
            tilesets,
            slices,
            palettes,
            user_data: sprite_user_data,
            color_profile,
            external_files,
        })
    }

    /// The palette in effect for a given frame (§6.10).
    pub fn palette_for(&self, frame: usize) -> &Palette {
        &self.palettes[frame.min(self.palettes.len().saturating_sub(1))]
    }

    /// A layer renders only if it and all ancestors are visible (§6.2).
    pub fn layer_visible_in_tree(&self, mut index: usize) -> bool {
        loop {
            let layer = &self.layers[index];
            if !layer.is_visible() {
                return false;
            }
            match layer.parent {
                Some(p) => index = p,
                None => return true,
            }
        }
    }
}
