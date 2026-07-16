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
    /// Case-sensitive substring match against layer names; matches are hidden.
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
            if !self.exclude_layers.is_empty() && layer.name.contains(&self.exclude_layers) {
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
) -> Result<Gd<AnimationLibrary>, String> {
    use godot::classes::animation::{LoopMode, TrackType, UpdateMode};

    let textures = frame_atlas_textures(file)?;
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

        let tex_track = anim.add_track(TrackType::VALUE);
        anim.track_set_path(
            tex_track,
            &NodePath::from(format!("{sprite_path}:texture").as_str()),
        );
        anim.value_track_set_update_mode(tex_track, UpdateMode::DISCRETE);

        let mut method_track: Option<i32> = None;
        let mut t_ms: u32 = 0;
        for &frame_index in &anim_def.order {
            let t = t_ms as f64 / 1000.0;
            anim.track_insert_key(tex_track, t, &textures[frame_index].clone().to_variant());

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

/// Builds a Godot TileSet from the file's tilesets: one TileSetAtlasSource
/// per Aseprite tileset (source id = tileset id), tiles laid out in a
/// near-square grid. The empty tile (id 0 in release-format files) is not
/// emitted. Per-tile user-data text lands in an "aseprite_text" custom data
/// layer. Cell flips need no alternative tiles: Godot cells carry
/// flip/transpose bits natively.
pub fn build_tileset(file: &AseFile) -> Result<Gd<TileSet>, String> {
    use ase_core::composite::tileset_strip_rgba;
    use godot::classes::{TileSetAtlasSource, TileSetSource};

    let embedded: Vec<_> = file
        .tilesets
        .iter()
        .filter(|t| t.pixels.is_some())
        .collect();
    if embedded.is_empty() {
        return Err("no embedded tilesets in file".to_string());
    }

    let mut tile_set = TileSet::new_gd();
    let first = embedded[0];
    tile_set.set_tile_size(Vector2i::new(
        first.tile_width as i32,
        first.tile_height as i32,
    ));

    let has_text = embedded
        .iter()
        .any(|t| t.tile_user_data.iter().any(|u| u.text.is_some()));
    if has_text {
        tile_set.add_custom_data_layer();
        tile_set.set_custom_data_layer_name(0, "aseprite_text");
        tile_set.set_custom_data_layer_type(0, VariantType::STRING);
    }

    for ts in embedded {
        let rgba = tileset_strip_rgba(file, ts).expect("filtered to embedded");
        let (tw, th) = (ts.tile_width as usize, ts.tile_height as usize);
        let start = if ts.zero_is_empty() { 1 } else { 0 };
        let count = (ts.num_tiles as usize).saturating_sub(start);
        if count == 0 {
            continue;
        }
        let cols = (count as f64).sqrt().ceil() as usize;
        let rows = count.div_ceil(cols);

        // Re-arrange the vertical strip into a cols x rows sheet.
        let (sheet_w, sheet_h) = (cols * tw, rows * th);
        let mut sheet = vec![0u8; sheet_w * sheet_h * 4];
        for i in 0..count {
            let src_tile = start + i;
            let (cx, cy) = (i % cols, i / cols);
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

        let mut source = TileSetAtlasSource::new_gd();
        source.set_texture(&texture);
        source.set_texture_region_size(Vector2i::new(tw as i32, th as i32));
        // The source must live inside the TileSet before TileData can accept
        // custom data (the layer definitions live on the TileSet).
        tile_set
            .add_source_ex(&source.clone().upcast::<TileSetSource>())
            .atlas_source_id_override(ts.id as i32)
            .done();
        for i in 0..count {
            let coords = Vector2i::new((i % cols) as i32, (i / cols) as i32);
            source.create_tile(coords);
            if has_text
                && let Some(ud) = ts.tile_user_data.get(start + i)
                && let Some(text) = &ud.text
                && let Some(mut td) = source.get_tile_data(coords, 0)
            {
                td.set_custom_data("aseprite_text", &text.as_str().to_variant());
            }
        }
    }

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
