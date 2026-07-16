//! EditorPlugin registration. gdext auto-instantiates EditorPlugin subclasses
//! in the editor; importers are added here and removed on unload.
//!
//! Wiring that needs `Gd<Self>` belongs in `enter_tree`, not `init`
//! (gdext#997).

use godot::classes::{EditorImportPlugin, EditorPlugin, IEditorPlugin};
use godot::prelude::*;

use crate::import::sprite_frames::AseSpriteFramesImporter;
use crate::import::texture::AseTextureImporter;

#[derive(GodotClass)]
#[class(tool, init, base=EditorPlugin)]
pub struct AsepriteImporterPlugin {
    texture: Option<Gd<EditorImportPlugin>>,
    sprite_frames: Option<Gd<EditorImportPlugin>>,
    base: Base<EditorPlugin>,
}

#[godot_api]
impl IEditorPlugin for AsepriteImporterPlugin {
    fn enter_tree(&mut self) {
        let texture = AseTextureImporter::new_gd().upcast::<EditorImportPlugin>();
        let sprite_frames = AseSpriteFramesImporter::new_gd().upcast::<EditorImportPlugin>();
        self.base_mut().add_import_plugin(&texture);
        self.base_mut().add_import_plugin(&sprite_frames);
        self.texture = Some(texture);
        self.sprite_frames = Some(sprite_frames);
    }

    fn exit_tree(&mut self) {
        if let Some(p) = self.sprite_frames.take() {
            self.base_mut().remove_import_plugin(&p);
        }
        if let Some(p) = self.texture.take() {
            self.base_mut().remove_import_plugin(&p);
        }
    }
}
