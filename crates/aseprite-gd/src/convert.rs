//! ase-core model → Godot resources. Everything here is pure conversion;
//! importer plumbing lives in `import/`.

use ase_core::AseFile;
use ase_core::composite::render_frame;
use ase_core::model::AniDir;
use godot::builtin::{GString, PackedByteArray, Rect2, StringName, VarDictionary, Vector2};
use godot::classes::image::Format;
use godot::classes::{
    Animation, AnimationLibrary, AtlasTexture, Image, ImageTexture, SpriteFrames, Texture2D,
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
    let atlas = crate::atlas::pack(&rendered, 1);

    let data = PackedByteArray::from(atlas.pixels.as_slice());
    let image = Image::create_from_data(
        atlas.width as i32,
        atlas.height as i32,
        false,
        Format::RGBA8,
        &data,
    )
    .ok_or("atlas Image::create_from_data failed")?;
    let sheet = ImageTexture::create_from_image(&image)
        .ok_or("atlas ImageTexture::create_from_image failed")?;

    let (cw, ch) = (file.header.width as f32, file.header.height as f32);
    let textures = file
        .frames
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let p = &atlas.placements[atlas.frame_to_placement[i]];
            let mut tex = AtlasTexture::new_gd();
            tex.set_atlas(&sheet);
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
