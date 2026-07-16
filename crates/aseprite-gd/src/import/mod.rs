//! EditorImportPlugin implementations. Each importer is a thin shell over
//! `convert`; shared option schema and boilerplate live here.
//!
//! Note (godot#104519): `get_priority`/`get_import_order` must be implemented
//! explicitly — engine defaults are unreliable for GDExtension importers.

pub mod animation_library;
pub mod sprite_frames;
pub mod texture;

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
    a
}
