//! Script-facing import helpers behind the editor UI (and usable directly
//! from EditorScripts). The inspector dock in `plugin.gd` is a thin veneer
//! over these; keeping the logic here makes it testable headlessly.

use godot::builtin::{GString, NodePath, StringName, VarDictionary};
use godot::classes::{Animation, AnimationLibrary, AnimationPlayer, RefCounted};
use godot::prelude::*;

use crate::convert::{self, AtlasParams, ConvertOptions};

/// Copies every track of `src` into `dst` (types, paths, update modes, keys).
fn copy_tracks(src: &Gd<Animation>, dst: &mut Gd<Animation>) {
    use godot::classes::animation::TrackType;
    for t in 0..src.get_track_count() {
        let ty = src.track_get_type(t);
        let nt = dst.add_track(ty);
        dst.track_set_path(nt, &src.track_get_path(t));
        if ty == TrackType::VALUE {
            dst.value_track_set_update_mode(nt, src.value_track_get_update_mode(t));
        }
        for k in 0..src.track_get_key_count(t) {
            dst.track_insert_key_ex(
                nt,
                src.track_get_key_time(t, k),
                &src.track_get_key_value(t, k),
            )
            .transition(src.track_get_key_transition(t, k))
            .done();
        }
    }
}

/// Removes from `anim` every track whose path is claimed by `generated` —
/// i.e. the tracks a previous import created — leaving hand-made tracks on
/// other paths untouched.
fn remove_claimed_tracks(anim: &mut Gd<Animation>, generated: &Gd<Animation>) {
    let claimed: Vec<NodePath> = (0..generated.get_track_count())
        .map(|t| generated.track_get_path(t))
        .collect();
    for t in (0..anim.get_track_count()).rev() {
        let path = anim.track_get_path(t);
        if claimed.contains(&path) {
            anim.remove_track(t);
        }
    }
}

/// Non-destructive AnimationPlayer import: builds animations from an
/// Aseprite file and merges them into the player's library.
#[derive(GodotClass)]
#[class(tool, init, base=RefCounted)]
pub struct AseAnimationImport {
    base: Base<RefCounted>,
}

#[godot_api]
impl AseAnimationImport {
    /// Merges the file's animations into `player`'s library named
    /// `options.library` (default the global "" library), creating it when
    /// missing. Options mirror the AnimationLibrary importer:
    /// exclude_layers, exclude_tags, include_hidden_layers, snap_to_fps,
    /// sprite_path, slice_tracks, split_layers, create_reset_animation,
    /// scale, atlas_padding, atlas_extrude, compress_mode.
    ///
    /// Merge rules, per generated animation name:
    /// - animation missing → added as-is;
    /// - animation exists → only tracks on paths the import owns are
    ///   replaced; tracks on other paths (hand-made) are kept, and the
    ///   animation's length/loop come from the file;
    /// - animations the file doesn't produce are never touched.
    ///
    /// The file path and options are stored as metadata on the player so
    /// [`Self::reimport`] can repeat the import later. Returns the number of
    /// animations merged (0 on failure; details are logged).
    #[func]
    fn merge_into_player(
        player: Option<Gd<AnimationPlayer>>,
        path: GString,
        options: VarDictionary,
    ) -> i64 {
        let Some(mut player) = player else {
            godot_error!("AseAnimationImport.merge_into_player: player is null");
            return 0;
        };
        let file = match convert::load_ase(&path) {
            Ok(f) => f,
            Err(e) => {
                godot_error!("AseAnimationImport: {path}: {e}");
                return 0;
            }
        };
        let file = ConvertOptions::from_dict(&options).apply(&file);
        let sprite_path = options
            .get(&"sprite_path".to_variant())
            .map(|v| v.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Sprite2D".to_string());
        let get_bool = |k: &str| {
            options
                .get(&k.to_variant())
                .map(|v| v.booleanize())
                .unwrap_or(false)
        };

        let generated = match convert::build_animation_library(
            &file,
            &sprite_path,
            get_bool("slice_tracks"),
            get_bool("split_layers"),
            get_bool("create_reset_animation"),
            AtlasParams::from_dict(&options),
        ) {
            Ok(l) => l,
            Err(e) => {
                godot_error!("AseAnimationImport: {path}: {e}");
                return 0;
            }
        };

        let lib_name = options
            .get(&"library".to_variant())
            .map(|v| StringName::from(v.to_string().as_str()))
            .unwrap_or_default();
        let mut target: Gd<AnimationLibrary> = match player.get_animation_library(&lib_name) {
            Some(l) => l,
            None => {
                let lib = AnimationLibrary::new_gd();
                player.add_animation_library(&lib_name, &lib);
                lib
            }
        };

        let mut merged = 0;
        let names = generated.get_animation_list();
        for name in names.iter_shared() {
            let name = &name;
            let Some(gen_anim) = generated.get_animation(name) else {
                continue;
            };
            // (block continues below)
            match target.get_animation(name) {
                Some(mut existing) => {
                    remove_claimed_tracks(&mut existing, &gen_anim);
                    copy_tracks(&gen_anim, &mut existing);
                    existing.set_length(gen_anim.get_length());
                    existing.set_loop_mode(gen_anim.get_loop_mode());
                }
                None => {
                    target.add_animation(name, &gen_anim);
                }
            }
            merged += 1;
        }

        // Remember the import so it can be repeated with one call/click.
        let mut meta = VarDictionary::new();
        meta.set(&"file".to_variant(), &path.to_variant());
        meta.set(&"options".to_variant(), &options.to_variant());
        player.set_meta("aseprite_gd_import", &meta.to_variant());

        merged
    }

    /// Repeats the last [`Self::merge_into_player`] using the metadata it
    /// stored on the player. Returns the number of animations merged.
    #[func]
    fn reimport(player: Option<Gd<AnimationPlayer>>) -> i64 {
        let Some(p) = player.clone() else {
            godot_error!("AseAnimationImport.reimport: player is null");
            return 0;
        };
        let meta = p.get_meta("aseprite_gd_import");
        let Ok(meta) = meta.try_to::<VarDictionary>() else {
            godot_error!("AseAnimationImport.reimport: node has no aseprite_gd_import metadata");
            return 0;
        };
        let path = meta
            .get(&"file".to_variant())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let options = meta
            .get(&"options".to_variant())
            .and_then(|v| v.try_to::<VarDictionary>().ok())
            .unwrap_or_default();
        Self::merge_into_player(player, GString::from(path.as_str()), options)
    }

    /// Builds SpriteFrames from the file and assigns it to an
    /// AnimatedSprite2D/AnimatedSprite3D (any object with a `sprite_frames`
    /// property). Stores reimport metadata like `merge_into_player`.
    #[func]
    fn assign_sprite_frames(
        sprite: Option<Gd<godot::classes::Object>>,
        path: GString,
        options: VarDictionary,
    ) -> bool {
        let Some(mut sprite) = sprite else {
            godot_error!("AseAnimationImport.assign_sprite_frames: sprite is null");
            return false;
        };
        let file = match convert::load_ase(&path) {
            Ok(f) => f,
            Err(e) => {
                godot_error!("AseAnimationImport: {path}: {e}");
                return false;
            }
        };
        let file = ConvertOptions::from_dict(&options).apply(&file);
        let split = options
            .get(&"split_layers".to_variant())
            .map(|v| v.booleanize())
            .unwrap_or(false);
        let frames = match if split {
            convert::build_sprite_frames_split(&file, AtlasParams::from_dict(&options))
        } else {
            convert::build_sprite_frames(&file, AtlasParams::from_dict(&options))
        } {
            Ok(f) => f,
            Err(e) => {
                godot_error!("AseAnimationImport: {path}: {e}");
                return false;
            }
        };
        sprite.set("sprite_frames", &frames.to_variant());

        let mut meta = VarDictionary::new();
        meta.set(&"file".to_variant(), &path.to_variant());
        meta.set(&"options".to_variant(), &options.to_variant());
        sprite.set_meta("aseprite_gd_import", &meta.to_variant());
        true
    }
}
