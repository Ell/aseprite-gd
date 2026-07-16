//! Default importer: one composited frame as an ImageTexture, so `.aseprite`
//! files drop into a project and work as plain images.

use godot::builtin::{AnyDictionary, GString, PackedStringArray, StringName, VarDictionary};
use godot::classes::{EditorImportPlugin, IEditorImportPlugin, ResourceSaver};
use godot::global::Error;
use godot::meta::ToGodot;
use godot::prelude::*;

use crate::convert::{self, ConvertOptions};
use crate::import;

#[derive(GodotClass)]
#[class(tool, init, base=EditorImportPlugin)]
pub struct AseTextureImporter {
    base: Base<EditorImportPlugin>,
}

#[godot_api]
impl IEditorImportPlugin for AseTextureImporter {
    fn get_importer_name(&self) -> GString {
        "aseprite_gd.texture".into()
    }

    fn get_visible_name(&self) -> GString {
        "Texture2D (Aseprite)".into()
    }

    fn get_recognized_extensions(&self) -> PackedStringArray {
        import::recognized_extensions()
    }

    fn get_save_extension(&self) -> GString {
        "res".into()
    }

    fn get_resource_type(&self) -> GString {
        "ImageTexture".into()
    }

    fn get_preset_count(&self) -> i32 {
        1
    }

    fn get_preset_name(&self, _preset_index: i32) -> GString {
        "Default".into()
    }

    fn get_priority(&self) -> f32 {
        1.0
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
        let mut frame = VarDictionary::new();
        frame.set(&"name".to_variant(), &"frame".to_variant());
        frame.set(&"default_value".to_variant(), &0.to_variant());
        opts.push(frame.upcast_any_dictionary());
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
        let frame = options
            .get(&"frame".to_variant())
            .map(|v| v.to::<i64>().max(0) as usize)
            .unwrap_or(0)
            .min(file.frames.len() - 1);

        let texture = match convert::texture_for_frame(&file, frame) {
            Ok(t) => t,
            Err(e) => {
                godot_error!("aseprite-gd: {source_file}: {e}");
                return Error::ERR_CANT_CREATE;
            }
        };

        let out = format!("{save_path}.res");
        ResourceSaver::singleton()
            .save_ex(&texture)
            .path(&GString::from(out.as_str()))
            .done()
    }
}
