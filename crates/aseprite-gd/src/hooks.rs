//! Post-import hook scripts: a user GDScript whose `_post_import` runs after
//! the importer builds its product but before it is saved (for the scene
//! importer: before ownership finalization and packing). The hook receives
//! the built object, the parsed document, the import options, and the source
//! path, and may mutate the object or return a replacement.
//!
//! ```gdscript
//! @tool
//! extends RefCounted
//!
//! func _post_import(resource, doc: AseDocument, options: Dictionary,
//!         source_file: String):
//!     resource.set_meta("tags", doc.get_tag_names())
//!     return resource # or a replacement; null keeps the original
//! ```

use godot::builtin::{GString, VarDictionary, Variant};
use godot::classes::{Object, ResourceLoader, Script};
use godot::prelude::*;

use crate::runtime::AseDocument;

/// Reads the `post_import_script` option; empty means "no hook".
pub fn hook_path(options: &VarDictionary) -> String {
    options
        .get(&"post_import_script".to_variant())
        .map(|v| v.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Runs the hook on `subject` (a Resource or a Node). Returns the object to
/// continue with: the hook's return value when it returns one of the right
/// type, otherwise `subject` (a null/void return means "keep it").
///
/// Errors (missing script, no `_post_import` method, instantiation failure)
/// fail the import — silently skipping a configured hook would be worse.
pub fn run_post_import(
    script_path: &str,
    subject: Variant,
    file: &ase_core::AseFile,
    options: &VarDictionary,
    source_file: &GString,
) -> Result<Variant, String> {
    let script = ResourceLoader::singleton()
        .load(script_path)
        .and_then(|r| r.try_cast::<Script>().ok())
        .ok_or_else(|| format!("post_import_script {script_path:?} is not a script"))?;

    // Script.new() instantiates the script's base type with the script
    // attached (plain RefCounted scripts work; @tool is required so the
    // script runs in the editor).
    let instance = script.clone().upcast::<Object>().call("new", &[]);
    let mut instance = instance
        .try_to::<Gd<Object>>()
        .map_err(|_| format!("post_import_script {script_path:?} could not be instantiated"))?;
    if !instance.has_method("_post_import") {
        return Err(format!(
            "post_import_script {script_path:?} has no _post_import method (is it missing @tool?)"
        ));
    }

    let doc = AseDocument::from_file(file.clone());
    let result = instance.call(
        "_post_import",
        &[
            subject.clone(),
            doc.to_variant(),
            options.to_variant(),
            source_file.to_variant(),
        ],
    );

    if result.is_nil() {
        Ok(subject)
    } else {
        Ok(result)
    }
}
