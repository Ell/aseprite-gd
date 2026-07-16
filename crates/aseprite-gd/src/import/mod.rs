//! EditorImportPlugin implementations. Each importer is a thin shell over
//! `convert`; shared option schema and boilerplate live here.
//!
//! Note (godot#104519): `get_priority`/`get_import_order` must be implemented
//! explicitly — engine defaults are unreliable for GDExtension importers.

pub mod animation_library;
pub mod canvas_texture;
pub mod scene;
pub mod sprite_frames;
pub mod stylebox;
pub mod texture;
pub mod tileset;

use godot::builtin::{AnyDictionary, GString, PackedStringArray, VarDictionary, Variant};
use godot::meta::ToGodot;
use godot::prelude::Array;

pub fn recognized_extensions() -> PackedStringArray {
    PackedStringArray::from(&[GString::from("aseprite"), GString::from("ase")])
}

fn option(name: &str, default: Variant) -> AnyDictionary {
    let mut d = VarDictionary::new();
    d.set(&"name".to_variant(), &name.to_variant());
    d.set(&"default_value".to_variant(), &default);
    d.upcast_any_dictionary()
}

/// Options common to all aseprite importers (see `convert::ConvertOptions`).
pub fn common_options() -> Array<AnyDictionary> {
    let mut a = Array::new();
    a.push(option("exclude_layers", "".to_variant()));
    a.push(option("include_hidden_layers", false.to_variant()));
    a.push(script_option());
    a
}

/// The `post_import_script` option with a file-picker hint.
fn script_option() -> AnyDictionary {
    let mut d = VarDictionary::new();
    d.set(&"name".to_variant(), &"post_import_script".to_variant());
    d.set(&"default_value".to_variant(), &"".to_variant());
    d.set(
        &"property_hint".to_variant(),
        &13i64.to_variant(), // PropertyHint::FILE (PROPERTY_HINT_FILE),
    );
    d.set(&"hint_string".to_variant(), &"*.gd".to_variant());
    d.upcast_any_dictionary()
}

/// Runs the configured post-import hook (if any) on a built resource and
/// returns the resource to save.
pub fn apply_resource_hook(
    resource: godot::obj::Gd<godot::classes::Resource>,
    file: &ase_core::AseFile,
    options: &VarDictionary,
    source_file: &GString,
) -> Result<godot::obj::Gd<godot::classes::Resource>, String> {
    use godot::meta::ToGodot;
    let path = crate::hooks::hook_path(options);
    if path.is_empty() {
        return Ok(resource);
    }
    let out =
        crate::hooks::run_post_import(&path, resource.to_variant(), file, options, source_file)?;
    out.try_to::<godot::obj::Gd<godot::classes::Resource>>()
        .map_err(|_| "post_import_script must return a Resource (or null)".to_string())
}
