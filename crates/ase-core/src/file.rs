//! Whole-file parsing: assembles the document model by walking frames and
//! dispatching chunks (§2). Unknown chunk types are skipped (gotcha #23).

use crate::model::{Frame, Header, Layer, Palette, Tag, Tileset};
use crate::parse::cel::InflateBudget;
use crate::parse::chunk::{parse_chunk_header, types};
use crate::parse::layer::resolve_parent;
use crate::parse::{self, HEADER_SIZE};
use crate::read::Reader;
use crate::Result;

/// A fully parsed Aseprite document.
#[derive(Debug, Clone)]
pub struct AseFile {
    pub header: Header,
    /// Flat, in file order — the index is the layer index cels reference.
    pub layers: Vec<Layer>,
    pub frames: Vec<Frame>,
    pub tags: Vec<Tag>,
    pub tilesets: Vec<Tileset>,
    /// Palette state after all palette chunks (per-frame snapshots TODO).
    pub palette: Palette,
}

impl AseFile {
    pub fn parse(data: &[u8]) -> Result<AseFile> {
        let mut r = Reader::new(data);
        let header = parse::parse_header(&mut r)?;

        let mut layers: Vec<Layer> = Vec::new();
        let mut frames: Vec<Frame> = Vec::with_capacity(header.frames as usize);
        let mut tags: Vec<Tag> = Vec::new();
        let mut tilesets: Vec<Tileset> = Vec::new();
        let mut palette = Palette::default();
        let mut budget = InflateBudget::default();
        // Old palette chunks are ignored once a 0x2019 has been seen (gotcha #10).
        let mut seen_new_palette = false;

        let mut frame_start = HEADER_SIZE;
        for _ in 0..header.frames {
            r.seek(frame_start)?;
            let fh = parse::parse_frame_header(&mut r)?;
            let frame_end = frame_start + fh.frame_bytes as usize;
            let mut cels = Vec::new();

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
                    }
                    types::CEL => {
                        if let Some(cel) = parse::parse_cel(
                            &mut r,
                            ch.end(),
                            header.color_depth,
                            layers.len(),
                            &mut budget,
                        )? {
                            cels.push(cel);
                        }
                    }
                    types::TAGS => tags.extend(parse::parse_tags(&mut r)?),
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
                        tilesets.push(parse::parse_tileset(&mut r, header.color_depth, &mut budget)?);
                    }
                    // Not yet modeled: cel extra, color profile, external
                    // files, user data, slices. Deprecated: mask, path.
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
            frame_start = frame_end;
        }

        Ok(AseFile { header, layers, frames, tags, tilesets, palette })
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
