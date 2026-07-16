//! AnimationLibrary importer: per-tag Animations with a texture value track
//! (atlas frames at exact times) and method tracks from cel user-data text.
//! Add the library to an AnimationPlayer whose root sits above the sprite
//! node named by the `sprite_path` option.

use godot::builtin::{AnyDictionary, GString, PackedStringArray, StringName, VarDictionary};
use godot::classes::{EditorImportPlugin, IEditorImportPlugin, ResourceSaver};
use godot::global::Error;
use godot::prelude::*;

use crate::convert::{self, ConvertOptions};
use crate::import;

#[derive(GodotClass)]
#[class(tool, init, base=EditorImportPlugin)]
pub struct AseAnimationLibraryImporter {
    base: Base<EditorImportPlugin>,
}

#[godot_api]
impl IEditorImportPlugin for AseAnimationLibraryImporter {
    fn get_importer_name(&self) -> GString {
        "aseprite_gd.animation_library".into()
    }

    fn get_visible_name(&self) -> GString {
        "AnimationLibrary (Aseprite)".into()
    }

    fn get_recognized_extensions(&self) -> PackedStringArray {
        import::recognized_extensions()
    }

    fn get_save_extension(&self) -> GString {
        "res".into()
    }

    fn get_resource_type(&self) -> GString {
        "AnimationLibrary".into()
    }

    fn get_preset_count(&self) -> i32 {
        1
    }

    fn get_preset_name(&self, _preset_index: i32) -> GString {
        "Default".into()
    }

    fn get_priority(&self) -> f32 {
        0.5
    }

    fn get_import_order(&self) -> i32 {
        0
    }

    fn get_option_visibility(
        &self,
        _path: GString,
        _option_name: StringName,
        _options: VarDictionary,
    ) -> bool {
        true
    }

    fn get_import_options(&self, _path: GString, _preset_index: i32) -> Array<AnyDictionary> {
        let mut opts = import::common_options();
        let mut sprite_path = VarDictionary::new();
        sprite_path.set(&"name".to_variant(), &"sprite_path".to_variant());
        sprite_path.set(&"default_value".to_variant(), &"Sprite2D".to_variant());
        opts.push(sprite_path.upcast_any_dictionary());
        let mut slice_tracks = VarDictionary::new();
        slice_tracks.set(&"name".to_variant(), &"slice_tracks".to_variant());
        slice_tracks.set(&"default_value".to_variant(), &false.to_variant());
        opts.push(slice_tracks.upcast_any_dictionary());
        opts
    }

    fn import(
        &mut self,
        source_file: GString,
        save_path: GString,
        options: VarDictionary,
        _platform_variants: Array<GString>,
        _gen_files: Array<GString>,
    ) -> Error {
        let file = match convert::load_ase(&source_file) {
            Ok(f) => f,
            Err(e) => {
                godot_error!("aseprite-gd: {source_file}: {e}");
                return Error::ERR_FILE_CORRUPT;
            }
        };
        let file = ConvertOptions::from_dict(&options).apply(&file);
        let sprite_path = options
            .get(&"sprite_path".to_variant())
            .map(|v| v.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Sprite2D".to_string());

        let slice_tracks = options
            .get(&"slice_tracks".to_variant())
            .map(|v| v.booleanize())
            .unwrap_or(false);
        let library = match convert::build_animation_library(&file, &sprite_path, slice_tracks) {
            Ok(l) => l,
            Err(e) => {
                godot_error!("aseprite-gd: {source_file}: {e}");
                return Error::ERR_CANT_CREATE;
            }
        };

        let resource = match import::apply_resource_hook(
            library.upcast::<godot::classes::Resource>(),
            &file,
            &options,
            &source_file,
        ) {
            Ok(r) => r,
            Err(e) => {
                godot_error!("aseprite-gd: {source_file}: {e}");
                return Error::ERR_CANT_CREATE;
            }
        };
        let out = format!("{save_path}.res");
        ResourceSaver::singleton()
            .save_ex(&resource)
            .path(&GString::from(out.as_str()))
            .done()
    }
}
