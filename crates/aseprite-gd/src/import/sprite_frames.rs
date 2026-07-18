//! SpriteFrames importer: tags become animations with exact per-frame
//! durations, ready for AnimatedSprite2D/3D.

use godot::builtin::{AnyDictionary, GString, PackedStringArray, StringName, VarDictionary};
use godot::classes::{EditorImportPlugin, IEditorImportPlugin, ResourceSaver};
use godot::global::Error;
use godot::prelude::*;

use crate::convert::{self, ConvertOptions};
use crate::import;

#[derive(GodotClass)]
#[class(tool, init, base=EditorImportPlugin)]
pub struct AseSpriteFramesImporter {
    base: Base<EditorImportPlugin>,
}

#[godot_api]
impl IEditorImportPlugin for AseSpriteFramesImporter {
    fn get_importer_name(&self) -> GString {
        "aseprite_gd.sprite_frames".into()
    }

    fn get_visible_name(&self) -> GString {
        "SpriteFrames (Aseprite)".into()
    }

    fn get_recognized_extensions(&self) -> PackedStringArray {
        import::recognized_extensions()
    }

    fn get_save_extension(&self) -> GString {
        "res".into()
    }

    fn get_resource_type(&self) -> GString {
        "SpriteFrames".into()
    }

    fn get_preset_count(&self) -> i32 {
        1
    }

    fn get_preset_name(&self, _preset_index: i32) -> GString {
        "Default".into()
    }

    // Below the texture importer: files import as textures unless the user
    // switches importer in the Import dock.
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
        let mut split = VarDictionary::new();
        split.set(&"name".to_variant(), &"split_layers".to_variant());
        split.set(&"default_value".to_variant(), &false.to_variant());
        opts.push(split.upcast_any_dictionary());
        let mut grid = VarDictionary::new();
        grid.set(&"name".to_variant(), &"split_grid".to_variant());
        grid.set(&"default_value".to_variant(), &"".to_variant());
        opts.push(grid.upcast_any_dictionary());
        let mut pad = VarDictionary::new();
        pad.set(&"name".to_variant(), &"atlas_padding".to_variant());
        pad.set(&"default_value".to_variant(), &1.to_variant());
        opts.push(pad.upcast_any_dictionary());
        let mut extrude = VarDictionary::new();
        extrude.set(&"name".to_variant(), &"atlas_extrude".to_variant());
        extrude.set(&"default_value".to_variant(), &false.to_variant());
        opts.push(extrude.upcast_any_dictionary());
        opts.push(import::option_pair("scale", 1i64));
        opts.push(import::enum_option(
            "compress_mode",
            0,
            "Lossless,Portable Lossless,Portable Lossy",
        ));
        opts.push(import::option_pair("snap_to_fps", 0.0f64));
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

        let split_layers = options
            .get(&"split_layers".to_variant())
            .map(|v| v.booleanize())
            .unwrap_or(false);
        // "WxH" (e.g. "16x16") chops each frame's canvas into cells.
        let split_grid = options
            .get(&"split_grid".to_variant())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let grid_cells = split_grid.trim().split_once(['x', 'X']).and_then(|(w, h)| {
            Some((w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?))
        });
        if !split_grid.trim().is_empty() && grid_cells.is_none() {
            godot_error!("aseprite-gd: {source_file}: split_grid must look like \"16x16\"");
            return Error::ERR_INVALID_PARAMETER;
        }
        let frames = match if let Some((cw, ch)) = grid_cells {
            convert::build_sprite_frames_grid(
                &file,
                convert::AtlasParams::from_dict(&options),
                cw,
                ch,
            )
        } else if split_layers {
            convert::build_sprite_frames_split(&file, convert::AtlasParams::from_dict(&options))
        } else {
            convert::build_sprite_frames(&file, convert::AtlasParams::from_dict(&options))
        } {
            Ok(f) => f,
            Err(e) => {
                godot_error!("aseprite-gd: {source_file}: {e}");
                return Error::ERR_CANT_CREATE;
            }
        };

        let resource = match import::apply_resource_hook(
            frames.upcast::<godot::classes::Resource>(),
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
