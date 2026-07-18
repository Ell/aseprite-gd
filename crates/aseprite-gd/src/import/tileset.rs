//! TileSet importer: Aseprite tilesets become TileSetAtlasSources (source id
//! = tileset id) with per-tile user-data text in an "aseprite_text" custom
//! data layer. Collision/terrain authored in Godot's TileSet editor should be
//! layered on via a separate resource until reimport-safe merging lands.

use godot::builtin::{AnyDictionary, GString, PackedStringArray, StringName, VarDictionary};
use godot::classes::{EditorImportPlugin, IEditorImportPlugin, ResourceSaver};
use godot::global::Error;
use godot::prelude::*;

use crate::convert::{self, ConvertOptions};
use crate::import;

#[derive(GodotClass)]
#[class(tool, init, base=EditorImportPlugin)]
pub struct AseTilesetImporter {
    base: Base<EditorImportPlugin>,
}

#[godot_api]
impl IEditorImportPlugin for AseTilesetImporter {
    fn get_importer_name(&self) -> GString {
        "aseprite_gd.tileset".into()
    }

    fn get_visible_name(&self) -> GString {
        "TileSet (Aseprite)".into()
    }

    fn get_recognized_extensions(&self) -> PackedStringArray {
        import::recognized_extensions()
    }

    fn get_save_extension(&self) -> GString {
        "res".into()
    }

    fn get_resource_type(&self) -> GString {
        "TileSet".into()
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
        let mut extract = VarDictionary::new();
        extract.set(&"name".to_variant(), &"extract_dir".to_variant());
        extract.set(&"default_value".to_variant(), &"".to_variant());
        opts.push(extract.upcast_any_dictionary());
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

        let extract_dir = options
            .get(&"extract_dir".to_variant())
            .map(|v| v.to_string())
            .unwrap_or_default();
        if !extract_dir.trim().is_empty()
            && let Err(e) = convert::extract_named_tiles(&file, extract_dir.trim())
        {
            godot_error!("aseprite-gd: {source_file}: {e}");
            return Error::ERR_CANT_CREATE;
        }
        let library = match convert::build_tileset(&file) {
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
