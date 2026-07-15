//! EditorPlugin registration. Import plugins, docks, and inspector plugins get
//! wired up here as they land (see docs/architecture.md "gdext layer").
//!
//! Editor classes must be `#[class(tool)]`; gdext auto-registers EditorPlugin
//! subclasses with the editor. Note: wiring that needs `Gd<Self>` belongs in
//! `enter_tree`, not `init` (gdext#997).

use godot::classes::{EditorPlugin, IEditorPlugin};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(tool, init, base=EditorPlugin)]
pub struct AsepriteImporterPlugin {
    base: Base<EditorPlugin>,
}

#[godot_api]
impl IEditorPlugin for AsepriteImporterPlugin {
    fn enter_tree(&mut self) {
        godot_print!("aseprite-gd: editor plugin loaded (importers not yet registered)");
        // TODO: add_import_plugin() calls for texture / SpriteFrames /
        // AnimationLibrary / TileSet importers as they are implemented.
    }

    fn exit_tree(&mut self) {
        // TODO: remove_import_plugin() in reverse order.
    }
}
