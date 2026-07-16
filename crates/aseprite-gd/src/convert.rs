//! ase-core model → Godot resources. Everything here is pure conversion;
//! importer plumbing lives in `import/`.

use ase_core::AseFile;
use ase_core::composite::render_frame;
use ase_core::model::AniDir;
use godot::builtin::{GString, PackedByteArray, StringName, VarDictionary};
use godot::classes::image::Format;
use godot::classes::{Image, ImageTexture, SpriteFrames};
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

/// Builds a SpriteFrames resource: one animation per tag (whole file becomes
/// "default" when untagged). Animation fps is fixed at 1000 so per-frame
/// durations in ms map exactly onto SpriteFrames' relative durations.
pub fn build_sprite_frames(file: &AseFile) -> Result<Gd<SpriteFrames>, String> {
    let mut frames = SpriteFrames::new_gd();

    // Rendered frames are shared between animations; render lazily.
    let mut cache: Vec<Option<Gd<ImageTexture>>> = vec![None; file.frames.len()];
    let mut texture = |i: usize| -> Result<Gd<ImageTexture>, String> {
        if cache[i].is_none() {
            cache[i] = Some(frame_texture(file, i)?);
        }
        Ok(cache[i].clone().unwrap())
    };

    struct Anim {
        name: String,
        order: Vec<usize>,
        looped: bool,
    }
    let anims: Vec<Anim> = if file.tags.is_empty() {
        vec![Anim {
            name: "default".to_string(),
            order: (0..file.frames.len()).collect(),
            looped: true,
        }]
    } else {
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
    };

    let default_name = StringName::from("default");
    if !file.tags.is_empty() {
        frames.remove_animation(&default_name);
    }

    for anim in anims {
        let name = StringName::from(anim.name.as_str());
        if !frames.has_animation(&name) {
            frames.add_animation(&name);
        }
        frames.set_animation_speed(&name, 1000.0);
        frames.set_animation_loop(&name, anim.looped);
        for &frame_index in &anim.order {
            let tex = texture(frame_index)?;
            let duration = file.frames[frame_index].duration_ms as f32;
            frames
                .add_frame_ex(&name, &tex.upcast::<godot::classes::Texture2D>())
                .duration(duration)
                .done();
        }
    }

    Ok(frames)
}

/// Loads and parses a `res://` (or absolute) path.
pub fn load_ase(path: &GString) -> Result<AseFile, String> {
    let bytes = godot::classes::FileAccess::get_file_as_bytes(path);
    if bytes.is_empty() {
        return Err(format!("cannot read {path}"));
    }
    AseFile::parse(bytes.as_slice()).map_err(|e| e.to_string())
}
