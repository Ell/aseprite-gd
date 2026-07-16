//! ase-core model → Godot resources. Everything here is pure conversion;
//! importer plumbing lives in `import/`.

use ase_core::AseFile;
use ase_core::composite::render_frame;
use ase_core::model::AniDir;
use godot::builtin::{
    GString, PackedByteArray, Rect2, StringName, VarDictionary, Vector2, Vector2i,
};
use godot::classes::image::Format;
use godot::classes::{
    Animation, AnimationLibrary, AtlasTexture, Image, ImageTexture, SpriteFrames, Texture2D,
    TileSet,
};
use godot::prelude::*;

/// Options shared by the importers.
pub struct ConvertOptions {
    /// Comma-separated, case-sensitive substrings; layers whose names contain
    /// any of them are hidden.
    pub exclude_layers: String,
    /// Render layers that are hidden in Aseprite too.
    pub include_hidden_layers: bool,
}

impl ConvertOptions {
    pub fn from_dict(options: &VarDictionary) -> Self {
        ConvertOptions {
            exclude_layers: options
                .get(&"exclude_layers".to_variant())
                .map(|v| v.to_string())
                .unwrap_or_default(),
            include_hidden_layers: options
                .get(&"include_hidden_layers".to_variant())
                .map(|v| v.booleanize())
                .unwrap_or(false),
        }
    }

    /// Returns a copy of the file with layer visibility adjusted per options.
    pub fn apply(&self, file: &AseFile) -> AseFile {
        let mut file = file.clone();
        for layer in &mut file.layers {
            if self.include_hidden_layers {
                layer.flags |= 1;
            }
            let excluded = self
                .exclude_layers
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .any(|p| layer.name.contains(p));
            if excluded {
                layer.flags &= !1;
            }
        }
        file
    }
}

/// Flattens one frame into a Godot Image (RGBA8, straight alpha).
pub fn frame_to_image(file: &AseFile, frame: usize) -> Result<Gd<Image>, String> {
    let img = render_frame(file, frame).map_err(|e| e.to_string())?;
    let data = PackedByteArray::from(img.pixels.as_slice());
    Image::create_from_data(
        img.width as i32,
        img.height as i32,
        false,
        Format::RGBA8,
        &data,
    )
    .ok_or_else(|| "Image::create_from_data failed".to_string())
}

fn frame_texture(file: &AseFile, frame: usize) -> Result<Gd<ImageTexture>, String> {
    let image = frame_to_image(file, frame)?;
    ImageTexture::create_from_image(&image)
        .ok_or_else(|| "ImageTexture::create_from_image failed".to_string())
}

pub fn texture_for_frame(file: &AseFile, frame: usize) -> Result<Gd<ImageTexture>, String> {
    frame_texture(file, frame)
}

/// Frame playback order for a tag range per its direction (§6.9). Ping-pong
/// plays there and back without repeating the endpoints.
fn tag_frame_order(from: usize, to: usize, direction: AniDir) -> Vec<usize> {
    match direction {
        AniDir::Forward => (from..=to).collect(),
        AniDir::Reverse => (from..=to).rev().collect(),
        // 0,1,2,3 -> 0,1,2,3,2,1 (endpoints not doubled when looping)
        AniDir::PingPong => {
            let mut v: Vec<usize> = (from..=to).collect();
            if to > from + 1 {
                v.extend((from + 1..to).rev());
            }
            v
        }
        AniDir::PingPongReverse => {
            let mut v: Vec<usize> = (from..=to).rev().collect();
            if to > from + 1 {
                v.extend(from + 1..to);
            }
            v
        }
    }
}

/// Animation definition shared by SpriteFrames and AnimationLibrary builders.
pub struct Anim {
    pub name: String,
    pub order: Vec<usize>,
    pub looped: bool,
}

/// Tags become animations; an untagged file becomes one "default" animation
/// over all frames.
pub fn animations(file: &AseFile) -> Vec<Anim> {
    if file.tags.is_empty() {
        return vec![Anim {
            name: "default".to_string(),
            order: (0..file.frames.len()).collect(),
            looped: true,
        }];
    }
    file.tags
        .iter()
        .map(|t| {
            let from = t.from_frame as usize;
            let to = (t.to_frame as usize).min(file.frames.len().saturating_sub(1));
            Anim {
                name: t.name.clone(),
                order: tag_frame_order(from, to, t.direction),
                // repeat 0 = unspecified = loop forever in Aseprite's UI (§6.9)
                looped: t.repeat == 0,
            }
        })
        .collect()
}

/// Renders every frame, packs a trimmed/deduped atlas, and returns one
/// canvas-sized AtlasTexture per frame (trim offsets restored via margins).
pub fn frame_atlas_textures(file: &AseFile) -> Result<Vec<Gd<AtlasTexture>>, String> {
    let rendered: Vec<_> = (0..file.frames.len())
        .map(|i| render_frame(file, i).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    // Godot rejects textures above 16384px on a side; the packer splits
    // pages under that cap.
    let atlas = crate::atlas::pack(&rendered, 1, 16384);

    let sheets: Vec<Gd<ImageTexture>> = atlas
        .pages
        .iter()
        .map(|page| {
            let data = PackedByteArray::from(page.pixels.as_slice());
            let image = Image::create_from_data(
                page.width as i32,
                page.height as i32,
                false,
                Format::RGBA8,
                &data,
            )
            .ok_or("atlas Image::create_from_data failed")?;
            ImageTexture::create_from_image(&image)
                .ok_or("atlas ImageTexture::create_from_image failed".to_string())
        })
        .collect::<Result<_, _>>()?;

    let (cw, ch) = (file.header.width as f32, file.header.height as f32);
    let textures = file
        .frames
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let p = &atlas.placements[atlas.frame_to_placement[i]];
            let mut tex = AtlasTexture::new_gd();
            tex.set_atlas(&sheets[p.page]);
            tex.set_region(Rect2::new(
                Vector2::new(p.x as f32, p.y as f32),
                Vector2::new(p.width as f32, p.height as f32),
            ));
            // Margin restores the trimmed frame to canvas size: position is
            // the trim offset, size is the total trimmed-away extent.
            tex.set_margin(Rect2::new(
                Vector2::new(p.offset_x as f32, p.offset_y as f32),
                Vector2::new(cw - p.width as f32, ch - p.height as f32),
            ));
            tex
        })
        .collect();
    Ok(textures)
}

/// Builds a SpriteFrames resource: one animation per tag (whole file becomes
/// "default" when untagged). Animation fps is fixed at 1000 so per-frame
/// durations in ms map exactly onto SpriteFrames' relative durations. All
/// frames share one packed atlas.
pub fn build_sprite_frames(file: &AseFile) -> Result<Gd<SpriteFrames>, String> {
    let mut frames = SpriteFrames::new_gd();
    let textures = frame_atlas_textures(file)?;

    let default_name = StringName::from("default");
    if !file.tags.is_empty() {
        frames.remove_animation(&default_name);
    }

    for anim in animations(file) {
        let name = StringName::from(anim.name.as_str());
        if !frames.has_animation(&name) {
            frames.add_animation(&name);
        }
        frames.set_animation_speed(&name, 1000.0);
        frames.set_animation_loop(&name, anim.looped);
        for &frame_index in &anim.order {
            let duration = file.frames[frame_index].duration_ms as f32;
            frames
                .add_frame_ex(&name, &textures[frame_index].clone().upcast::<Texture2D>())
                .duration(duration)
                .done();
        }
    }

    Ok(frames)
}

/// Builds an AnimationLibrary: per tag, a texture value track on
/// `<sprite_path>:texture` keyed with atlas frames at exact times, plus a
/// method track when cels carry user-data text (the text is the method name,
/// keyed at the frame's start — footsteps, impacts, spawn points).
pub fn build_animation_library(
    file: &AseFile,
    sprite_path: &str,
    slice_tracks: bool,
    split_layers: bool,
) -> Result<Gd<AnimationLibrary>, String> {
    use godot::classes::animation::{LoopMode, TrackType, UpdateMode};

    // Split mode: one texture track per layer, targeting
    // "<sprite_path>/<layer>:texture" — sprite_path is the container node
    // holding one sprite child per layer. Playing one animation drives all
    // layers in sync.
    let split_units: Vec<(String, Vec<Gd<AtlasTexture>>)> = if split_layers {
        split_atlas_textures(file)?
    } else {
        Vec::new()
    };
    let textures = if split_layers {
        Vec::new()
    } else {
        frame_atlas_textures(file)?
    };
    let mut library = AnimationLibrary::new_gd();

    for anim_def in animations(file) {
        let mut anim = Animation::new_gd();
        let total_ms: u32 = anim_def
            .order
            .iter()
            .map(|&i| file.frames[i].duration_ms as u32)
            .sum();
        anim.set_length(total_ms as f32 / 1000.0);
        anim.set_loop_mode(if anim_def.looped {
            LoopMode::LINEAR
        } else {
            LoopMode::NONE
        });

        let mut tex_tracks: Vec<(i32, &Vec<Gd<AtlasTexture>>)> = Vec::new();
        if split_layers {
            for (unit, unit_textures) in &split_units {
                let track = anim.add_track(TrackType::VALUE);
                anim.track_set_path(
                    track,
                    &NodePath::from(format!("{sprite_path}/{unit}:texture").as_str()),
                );
                anim.value_track_set_update_mode(track, UpdateMode::DISCRETE);
                tex_tracks.push((track, unit_textures));
            }
        } else {
            let track = anim.add_track(TrackType::VALUE);
            anim.track_set_path(
                track,
                &NodePath::from(format!("{sprite_path}:texture").as_str()),
            );
            anim.value_track_set_update_mode(track, UpdateMode::DISCRETE);
            tex_tracks.push((track, &textures));
        }

        let mut method_track: Option<i32> = None;
        let mut t_ms: u32 = 0;
        for &frame_index in &anim_def.order {
            let t = t_ms as f64 / 1000.0;
            for (track, unit_textures) in &tex_tracks {
                anim.track_insert_key(*track, t, &unit_textures[frame_index].clone().to_variant());
            }

            // Any cel in this frame with user-data text triggers a method call.
            for cel in &file.frames[frame_index].cels {
                if let Some(text) = &cel.user_data.text {
                    if text.is_empty() {
                        continue;
                    }
                    let track = *method_track.get_or_insert_with(|| {
                        let tr = anim.add_track(TrackType::METHOD);
                        anim.track_set_path(tr, &NodePath::from(sprite_path));
                        tr
                    });
                    let mut key = VarDictionary::new();
                    key.set(
                        &"method".to_variant(),
                        &StringName::from(text.as_str()).to_variant(),
                    );
                    key.set(
                        &"args".to_variant(),
                        &godot::builtin::VarArray::new().to_variant(),
                    );
                    anim.track_insert_key(track, t, &key.to_variant());
                }
            }
            t_ms += file.frames[frame_index].duration_ms as u32;
        }

        // Opt-in: animate one child node per slice ("<slice name>:position"
        // / ":size") — hitboxes/hurtboxes keyed from per-frame slice keys.
        if slice_tracks {
            for slice in &file.slices {
                if slice.keys.is_empty() {
                    continue;
                }
                let pos_track = anim.add_track(TrackType::VALUE);
                anim.track_set_path(
                    pos_track,
                    &NodePath::from(format!("{}:position", slice.name).as_str()),
                );
                anim.value_track_set_update_mode(pos_track, UpdateMode::DISCRETE);
                let size_track = anim.add_track(TrackType::VALUE);
                anim.track_set_path(
                    size_track,
                    &NodePath::from(format!("{}:size", slice.name).as_str()),
                );
                anim.value_track_set_update_mode(size_track, UpdateMode::DISCRETE);

                let mut t_ms: u32 = 0;
                for &frame_index in &anim_def.order {
                    let t = t_ms as f64 / 1000.0;
                    if let Some(key) = slice.key_for(frame_index as u32)
                        && key.width > 0
                        && key.height > 0
                    {
                        anim.track_insert_key(
                            pos_track,
                            t,
                            &Vector2::new(key.x as f32, key.y as f32).to_variant(),
                        );
                        anim.track_insert_key(
                            size_track,
                            t,
                            &Vector2::new(key.width as f32, key.height as f32).to_variant(),
                        );
                    }
                    t_ms += file.frames[frame_index].duration_ms as u32;
                }
            }
        }

        library.add_animation(&StringName::from(anim_def.name.as_str()), &anim);
    }

    Ok(library)
}

/// Loads and parses a `res://` (or absolute) path.
pub fn load_ase(path: &GString) -> Result<AseFile, String> {
    let bytes = godot::classes::FileAccess::get_file_as_bytes(path);
    if bytes.is_empty() {
        return Err(format!("cannot read {path}"));
    }
    AseFile::parse(bytes.as_slice()).map_err(|e| e.to_string())
}

/// Fixed sheet column count. A constant layout keeps tile atlas coords
/// stable as the artist adds tiles, so per-tile data on synced TileSets
/// survives growth (index i -> coords (i % COLS, i / COLS) forever).
pub const TILESET_COLS: usize = 16;

/// Atlas coords of the i-th non-empty tile.
pub fn tile_coords(i: usize) -> Vector2i {
    Vector2i::new((i % TILESET_COLS) as i32, (i / TILESET_COLS) as i32)
}

/// Sheet texture for one Aseprite tileset: the vertical strip re-arranged
/// into a TILESET_COLS-wide grid, empty tile skipped.
fn tileset_sheet(
    file: &AseFile,
    ts: &ase_core::model::Tileset,
) -> Result<Option<(Gd<ImageTexture>, usize, usize)>, String> {
    use ase_core::composite::tileset_strip_rgba;

    let Some(rgba) = tileset_strip_rgba(file, ts) else {
        return Ok(None); // external tileset
    };
    let (tw, th) = (ts.tile_width as usize, ts.tile_height as usize);
    let start = if ts.zero_is_empty() { 1 } else { 0 };
    let count = (ts.num_tiles as usize).saturating_sub(start);
    if count == 0 {
        return Ok(None);
    }
    let cols = count.min(TILESET_COLS);
    let rows = count.div_ceil(TILESET_COLS);
    let (sheet_w, sheet_h) = (cols * tw, rows * th);
    let mut sheet = vec![0u8; sheet_w * sheet_h * 4];
    for i in 0..count {
        let src_tile = start + i;
        let (cx, cy) = (i % TILESET_COLS, i / TILESET_COLS);
        for row in 0..th {
            let src = ((src_tile * th + row) * tw) * 4;
            let dst = ((cy * th + row) * sheet_w + cx * tw) * 4;
            sheet[dst..dst + tw * 4].copy_from_slice(&rgba[src..src + tw * 4]);
        }
    }
    let data = PackedByteArray::from(sheet.as_slice());
    let image =
        Image::create_from_data(sheet_w as i32, sheet_h as i32, false, Format::RGBA8, &data)
            .ok_or("tileset Image::create_from_data failed")?;
    let texture = ImageTexture::create_from_image(&image)
        .ok_or("tileset ImageTexture::create_from_image failed")?;
    Ok(Some((texture, count, start)))
}

/// Ensures the "aseprite_text" custom data layer exists; returns its index.
fn ensure_text_layer(tile_set: &mut Gd<TileSet>) -> i32 {
    for i in 0..tile_set.get_custom_data_layers_count() {
        if tile_set.get_custom_data_layer_name(i) == "aseprite_text" {
            return i;
        }
    }
    tile_set.add_custom_data_layer();
    let idx = tile_set.get_custom_data_layers_count() - 1;
    tile_set.set_custom_data_layer_name(idx, "aseprite_text");
    tile_set.set_custom_data_layer_type(idx, VariantType::STRING);
    idx
}

/// Syncs the file's tilesets into an existing TileSet, preserving everything
/// the user authored (see docs/tileset-workflow.md): sources are matched by
/// Aseprite tileset id and created when missing; surviving tiles keep their
/// TileData; tiles no longer in the file are removed. Returns the number of
/// sources synced.
pub fn sync_tileset_into(file: &AseFile, tile_set: &mut Gd<TileSet>) -> Result<u32, String> {
    use godot::classes::{TileSetAtlasSource, TileSetSource};

    let has_text = file
        .tilesets
        .iter()
        .any(|t| t.tile_user_data.iter().any(|u| u.text.is_some()));
    if has_text {
        ensure_text_layer(tile_set);
    }

    let mut synced = 0;
    for ts in &file.tilesets {
        let Some((texture, count, start)) = tileset_sheet(file, ts)? else {
            continue;
        };
        let (tw, th) = (ts.tile_width as i32, ts.tile_height as i32);
        let id = ts.id as i32;

        let mut source = if tile_set.has_source(id) {
            tile_set
                .get_source(id)
                .and_then(|s| s.try_cast::<TileSetAtlasSource>().ok())
                .ok_or_else(|| format!("TileSet source {id} exists but is not an atlas source"))?
        } else {
            let source = TileSetAtlasSource::new_gd();
            tile_set
                .add_source_ex(&source.clone().upcast::<TileSetSource>())
                .atlas_source_id_override(id)
                .done();
            source
        };
        source.set_texture(&texture);
        source.set_texture_region_size(Vector2i::new(tw, th));

        // Drop tiles that no longer exist in the file (their coords lie past
        // the current tile count in the fixed-column layout).
        let stale: Vec<Vector2i> = (0..source.get_tiles_count())
            .map(|n| source.get_tile_id(n))
            .filter(|c| {
                let i = c.y as usize * TILESET_COLS + c.x as usize;
                c.x as usize >= TILESET_COLS || i >= count
            })
            .collect();
        for coords in stale {
            source.remove_tile(coords);
        }

        for i in 0..count {
            let coords = tile_coords(i);
            if !source.has_tile(coords) {
                source.create_tile(coords);
            }
            if has_text
                && let Some(ud) = ts.tile_user_data.get(start + i)
                && let Some(text) = &ud.text
                && let Some(mut td) = source.get_tile_data(coords, 0)
            {
                td.set_custom_data("aseprite_text", &text.as_str().to_variant());
            }
        }
        synced += 1;
    }

    if synced == 0 {
        return Err("no embedded tilesets in file".to_string());
    }
    Ok(synced)
}

/// Builds a fresh Godot TileSet (the import product; regenerated every
/// reimport). For collision/terrain workflows use `sync_tileset_into` — see
/// docs/tileset-workflow.md.
pub fn build_tileset(file: &AseFile) -> Result<Gd<TileSet>, String> {
    let first = file
        .tilesets
        .iter()
        .find(|t| t.pixels.is_some())
        .ok_or("no embedded tilesets in file")?;
    let mut tile_set = TileSet::new_gd();
    tile_set.set_tile_size(Vector2i::new(
        first.tile_width as i32,
        first.tile_height as i32,
    ));
    sync_tileset_into(file, &mut tile_set)?;
    Ok(tile_set)
}

/// Builds a StyleBoxTexture from a 9-patch slice (§6.12): the slice rect is
/// cropped out of the rendered frame, and the center rect becomes the four
/// texture margins. `slice_name` empty = first slice with a center.
pub fn build_stylebox(
    file: &AseFile,
    slice_name: &str,
    frame: usize,
) -> Result<Gd<godot::classes::StyleBoxTexture>, String> {
    use godot::builtin::Side;
    use godot::classes::StyleBoxTexture;

    let slice = if slice_name.is_empty() {
        file.slices
            .iter()
            .find(|s| s.keys.first().is_some_and(|k| k.center.is_some()))
            .ok_or("no 9-patch slice in file")?
    } else {
        file.slices
            .iter()
            .find(|s| s.name == slice_name)
            .ok_or_else(|| format!("no slice named {slice_name:?}"))?
    };
    let key = slice
        .key_for(frame as u32)
        .ok_or("slice has no key at this frame")?;
    if key.width == 0 || key.height == 0 {
        return Err("slice is hidden at this frame".to_string());
    }

    // Crop the slice rect out of the rendered frame, clamped to the canvas.
    let rendered = render_frame(file, frame).map_err(|e| e.to_string())?;
    let (cw, ch) = (rendered.width as i64, rendered.height as i64);
    let x0 = (key.x as i64).clamp(0, cw);
    let y0 = (key.y as i64).clamp(0, ch);
    let x1 = (key.x as i64 + key.width as i64).clamp(0, cw);
    let y1 = (key.y as i64 + key.height as i64).clamp(0, ch);
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if w == 0 || h == 0 {
        return Err("slice lies outside the canvas".to_string());
    }
    let mut pixels = Vec::with_capacity(w * h * 4);
    for y in y0..y1 {
        let row = ((y * cw + x0) * 4) as usize;
        pixels.extend_from_slice(&rendered.pixels[row..row + w * 4]);
    }
    let data = PackedByteArray::from(pixels.as_slice());
    let image = Image::create_from_data(w as i32, h as i32, false, Format::RGBA8, &data)
        .ok_or("slice Image::create_from_data failed")?;
    let texture = ImageTexture::create_from_image(&image).ok_or("slice ImageTexture failed")?;

    let mut sb = StyleBoxTexture::new_gd();
    sb.set_texture(&texture.upcast::<Texture2D>());
    if let Some((cx, cy, cw_, ch_)) = key.center {
        // Center rect is relative to the slice bounds (§6.12).
        sb.set_texture_margin(Side::LEFT, cx as f32);
        sb.set_texture_margin(Side::TOP, cy as f32);
        sb.set_texture_margin(
            Side::RIGHT,
            (key.width as i64 - cx as i64 - cw_ as i64).max(0) as f32,
        );
        sb.set_texture_margin(
            Side::BOTTOM,
            (key.height as i64 - cy as i64 - ch_ as i64).max(0) as f32,
        );
    }
    Ok(sb)
}

/// Layer-name convention for lit sprites: layers named (or suffixed)
/// "normal"/"emission"/"specular" — case-insensitive — are map layers, not
/// color art.
fn map_layer_kind(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    for kind in ["normal", "specular", "emission"] {
        if n == kind || n.ends_with(&format!("_{kind}")) || n.ends_with(&format!(" {kind}")) {
            return Some(if kind == "emission" { "specular" } else { kind });
        }
    }
    None
}

/// Renders one frame with only the given predicate's layers visible.
fn render_filtered(
    file: &AseFile,
    frame: usize,
    keep: impl Fn(&str) -> bool,
) -> Result<Option<Gd<ImageTexture>>, String> {
    let mut filtered = file.clone();
    for layer in &mut filtered.layers {
        if !keep(&layer.name) {
            layer.flags &= !1;
        }
    }
    let img = render_frame(&filtered, frame).map_err(|e| e.to_string())?;
    if img.pixels.chunks_exact(4).all(|px| px[3] == 0) {
        return Ok(None); // nothing visible
    }
    let data = PackedByteArray::from(img.pixels.as_slice());
    let image = Image::create_from_data(
        img.width as i32,
        img.height as i32,
        false,
        Format::RGBA8,
        &data,
    )
    .ok_or("Image::create_from_data failed")?;
    Ok(Some(
        ImageTexture::create_from_image(&image).ok_or("ImageTexture failed")?,
    ))
}

/// Builds a CanvasTexture for lit pixel art: diffuse from ordinary layers,
/// normal/specular maps from convention-named layers (all sharing the same
/// canvas-space layout).
pub fn build_canvas_texture(
    file: &AseFile,
    frame: usize,
) -> Result<Gd<godot::classes::CanvasTexture>, String> {
    use godot::classes::CanvasTexture;

    let diffuse = render_filtered(file, frame, |n| map_layer_kind(n).is_none())?
        .ok_or("no visible color layers")?;
    let normal = render_filtered(file, frame, |n| map_layer_kind(n) == Some("normal"))?;
    let specular = render_filtered(file, frame, |n| map_layer_kind(n) == Some("specular"))?;

    let mut ct = CanvasTexture::new_gd();
    ct.set_diffuse_texture(&diffuse.upcast::<Texture2D>());
    if let Some(n) = normal {
        ct.set_normal_texture(&n.upcast::<Texture2D>());
    }
    if let Some(s) = specular {
        ct.set_specular_texture(&s.upcast::<Texture2D>());
    }
    Ok(ct)
}

/// Layers imported separately in split mode: leaf (non-group) layers visible
/// in the tree, in file order. Duplicate names get a numeric suffix so
/// animation names stay unambiguous.
pub fn split_units(file: &AseFile) -> Vec<(usize, String)> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, usize> = HashMap::new();
    file.layers
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            l.layer_type != ase_core::model::LayerType::Group && file.layer_visible_in_tree(*i)
        })
        .map(|(i, l)| {
            let n = seen.entry(l.name.clone()).or_insert(0);
            *n += 1;
            let name = if *n == 1 {
                l.name.clone()
            } else {
                format!("{}_{}", l.name, *n)
            };
            (i, name)
        })
        .collect()
}

/// A copy of the file where only `target` (and its ancestor groups) render.
fn isolate_layer(file: &AseFile, target: usize) -> AseFile {
    let mut f = file.clone();
    let mut keep = std::collections::HashSet::new();
    keep.insert(target);
    let mut cur = f.layers[target].parent;
    while let Some(p) = cur {
        keep.insert(p);
        cur = f.layers[p].parent;
    }
    for (i, layer) in f.layers.iter_mut().enumerate() {
        if keep.contains(&i) {
            layer.flags |= 1;
        } else {
            layer.flags &= !1;
        }
    }
    f
}

/// One split unit: layer name plus its per-frame canvas-sized textures.
pub type UnitTextures = (String, Vec<Gd<AtlasTexture>>);

/// Per-unit frame textures for split-layer imports. All units' frames share
/// one packed atlas (identical/empty renders dedup across units).
pub fn split_atlas_textures(file: &AseFile) -> Result<Vec<UnitTextures>, String> {
    let units = split_units(file);
    if units.is_empty() {
        return Err("no visible layers to split".to_string());
    }
    let frame_count = file.frames.len();

    let mut renders = Vec::with_capacity(units.len() * frame_count);
    for (idx, _) in &units {
        let isolated = isolate_layer(file, *idx);
        for f in 0..frame_count {
            renders.push(render_frame(&isolated, f).map_err(|e| e.to_string())?);
        }
    }
    let atlas = crate::atlas::pack(&renders, 1, 16384);

    let sheets: Vec<Gd<ImageTexture>> = atlas
        .pages
        .iter()
        .map(|page| {
            let data = PackedByteArray::from(page.pixels.as_slice());
            let image = Image::create_from_data(
                page.width as i32,
                page.height as i32,
                false,
                Format::RGBA8,
                &data,
            )
            .ok_or("atlas Image::create_from_data failed")?;
            ImageTexture::create_from_image(&image)
                .ok_or("atlas ImageTexture::create_from_image failed".to_string())
        })
        .collect::<Result<_, _>>()?;

    let (cw, ch) = (file.header.width as f32, file.header.height as f32);
    let mut out = Vec::with_capacity(units.len());
    for (u, (_, name)) in units.iter().enumerate() {
        let textures = (0..frame_count)
            .map(|f| {
                let p = &atlas.placements[atlas.frame_to_placement[u * frame_count + f]];
                let mut tex = AtlasTexture::new_gd();
                tex.set_atlas(&sheets[p.page]);
                tex.set_region(Rect2::new(
                    Vector2::new(p.x as f32, p.y as f32),
                    Vector2::new(p.width as f32, p.height as f32),
                ));
                tex.set_margin(Rect2::new(
                    Vector2::new(p.offset_x as f32, p.offset_y as f32),
                    Vector2::new(cw - p.width as f32, ch - p.height as f32),
                ));
                tex
            })
            .collect();
        out.push((name.clone(), textures));
    }
    Ok(out)
}

/// Split-mode SpriteFrames: one animation per layer per tag, named
/// "<layer>/<tag>". Stack one AnimatedSprite2D per layer and play the same
/// tag on each for multi-layer characters.
pub fn build_sprite_frames_split(file: &AseFile) -> Result<Gd<SpriteFrames>, String> {
    let mut frames = SpriteFrames::new_gd();
    frames.remove_animation(&StringName::from("default"));
    let units = split_atlas_textures(file)?;

    for (unit_name, textures) in &units {
        for anim in animations(file) {
            let name = StringName::from(format!("{unit_name}/{}", anim.name).as_str());
            frames.add_animation(&name);
            frames.set_animation_speed(&name, 1000.0);
            frames.set_animation_loop(&name, anim.looped);
            for &frame_index in &anim.order {
                let duration = file.frames[frame_index].duration_ms as f32;
                frames
                    .add_frame_ex(&name, &textures[frame_index].clone().upcast::<Texture2D>())
                    .duration(duration)
                    .done();
            }
        }
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::ConvertOptions;

    fn fixture() -> ase_core::AseFile {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ase-core/tests/fixtures/generated/group_blend.aseprite"
        );
        ase_core::AseFile::parse(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn exclude_layers_takes_a_comma_separated_list() {
        let file = fixture(); // layers: base, fx (group), inner_normal, inner_addition
        let opts = ConvertOptions {
            exclude_layers: "inner_normal, inner_addition".to_string(),
            include_hidden_layers: false,
        };
        let out = opts.apply(&file);
        let vis: Vec<(&str, bool)> = out
            .layers
            .iter()
            .map(|l| (l.name.as_str(), l.is_visible()))
            .collect();
        assert_eq!(
            vis,
            vec![
                ("base", true),
                ("fx", true),
                ("inner_normal", false),
                ("inner_addition", false)
            ]
        );

        // Single pattern still works; empty string excludes nothing.
        let one = ConvertOptions {
            exclude_layers: "addition".into(),
            include_hidden_layers: false,
        }
        .apply(&file);
        assert!(one.layers[2].is_visible() && !one.layers[3].is_visible());
        let none = ConvertOptions {
            exclude_layers: "".into(),
            include_hidden_layers: false,
        }
        .apply(&file);
        assert!(none.layers.iter().all(|l| l.is_visible()));
    }
}
