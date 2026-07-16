//! PackedScene importer: a ready-to-instance scene — Node2D root with an
//! AnimatedSprite2D playing the file's animations. The post-import hook runs
//! on the live node tree BEFORE ownership finalization and packing, so hooks
//! can add nodes (hitboxes from slices, markers from user data), reparent, or
//! replace the root entirely.

use godot::builtin::{AnyDictionary, GString, PackedStringArray, StringName, VarDictionary};
use godot::classes::{
    AnimatedSprite2D, EditorImportPlugin, IEditorImportPlugin, Node, Node2D, PackedScene,
    ResourceSaver,
};
use godot::global::Error;
use godot::prelude::*;

use crate::convert::{self, ConvertOptions};
use crate::{hooks, import};

/// Everything without an owner becomes owned by the root, so hook-added
/// nodes survive packing.
fn own_recursive(node: &Gd<Node>, root: &Gd<Node>) {
    for i in 0..node.get_child_count() {
        let Some(mut child) = node.get_child(i) else {
            continue;
        };
        if child.get_owner().is_none() && child != *root {
            child.set_owner(root);
        }
        own_recursive(&child, root);
    }
}

#[derive(GodotClass)]
#[class(tool, init, base=EditorImportPlugin)]
pub struct AseSceneImporter {
    base: Base<EditorImportPlugin>,
}

#[godot_api]
impl IEditorImportPlugin for AseSceneImporter {
    fn get_importer_name(&self) -> GString {
        "aseprite_gd.scene".into()
    }

    fn get_visible_name(&self) -> GString {
        "PackedScene (Aseprite)".into()
    }

    fn get_recognized_extensions(&self) -> PackedStringArray {
        import::recognized_extensions()
    }

    fn get_save_extension(&self) -> GString {
        "scn".into()
    }

    fn get_resource_type(&self) -> GString {
        "PackedScene".into()
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
        let mut autoplay = VarDictionary::new();
        autoplay.set(&"name".to_variant(), &"autoplay".to_variant());
        autoplay.set(&"default_value".to_variant(), &true.to_variant());
        opts.push(autoplay.upcast_any_dictionary());
        let mut pad = VarDictionary::new();
        pad.set(&"name".to_variant(), &"atlas_padding".to_variant());
        pad.set(&"default_value".to_variant(), &1.to_variant());
        opts.push(pad.upcast_any_dictionary());
        let mut extrude = VarDictionary::new();
        extrude.set(&"name".to_variant(), &"atlas_extrude".to_variant());
        extrude.set(&"default_value".to_variant(), &false.to_variant());
        opts.push(extrude.upcast_any_dictionary());
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

        let frames =
            match convert::build_sprite_frames(&file, convert::AtlasParams::from_dict(&options)) {
                Ok(f) => f,
                Err(e) => {
                    godot_error!("aseprite-gd: {source_file}: {e}");
                    return Error::ERR_CANT_CREATE;
                }
            };

        // Root named after the file, sprite child playing the first animation.
        let mut root = Node2D::new_alloc();
        let stem = source_file
            .to_string()
            .rsplit('/')
            .next()
            .unwrap_or("AseScene")
            .split('.')
            .next()
            .unwrap_or("AseScene")
            .to_string();
        root.set_name(&GString::from(stem.as_str()));

        let mut sprite = AnimatedSprite2D::new_alloc();
        sprite.set_name("AnimatedSprite2D");
        sprite.set_sprite_frames(&frames);
        let autoplay = options
            .get(&"autoplay".to_variant())
            .map(|v| v.booleanize())
            .unwrap_or(true);
        if let Some(anim) = convert::animations(&file).first() {
            if autoplay {
                sprite.set_autoplay(&GString::from(anim.name.as_str()));
            }
            sprite.set_animation(&StringName::from(anim.name.as_str()));
        }
        root.add_child(&sprite);
        sprite.set_owner(&root);

        // Hook runs on live nodes, pre-finalization.
        let mut root: Gd<Node> = root.upcast();
        let hook = hooks::hook_path(&options);
        if !hook.is_empty() {
            match hooks::run_post_import(&hook, root.to_variant(), &file, &options, &source_file)
                .and_then(|v| {
                    v.try_to::<Gd<Node>>()
                        .map_err(|_| "post_import_script must return a Node (or null)".to_string())
                }) {
                Ok(new_root) => {
                    if new_root != root {
                        root.queue_free();
                        root = new_root;
                    }
                }
                Err(e) => {
                    godot_error!("aseprite-gd: {source_file}: {e}");
                    root.free();
                    return Error::ERR_CANT_CREATE;
                }
            }
        }

        // Finalize: adopt hook-added nodes, pack, save.
        own_recursive(&root.clone(), &root.clone());
        let mut packed = PackedScene::new_gd();
        if packed.pack(&root) != Error::OK {
            godot_error!("aseprite-gd: {source_file}: PackedScene.pack failed");
            root.free();
            return Error::ERR_CANT_CREATE;
        }
        root.free();

        let out = format!("{save_path}.scn");
        ResourceSaver::singleton()
            .save_ex(&packed)
            .path(&GString::from(out.as_str()))
            .done()
    }
}
