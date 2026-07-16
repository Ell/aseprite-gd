//! Godot GDExtension entry point. This crate is a thin adapter: all parsing
//! and compositing lives in `ase-core`; modules here only translate to Godot
//! types and register editor/runtime integration.

use godot::prelude::*;

pub mod atlas;
pub mod convert;
pub mod editor_tools;
pub mod hooks;
pub mod import;
pub mod runtime;

struct AsepriteGdExtension;

// The runtime loader Gd handle lives here between init and deinit; extension
// init/deinit run on the main thread.
thread_local! {
    static RUNTIME_LOADER: std::cell::RefCell<Option<Gd<runtime::AseResourceLoader>>> =
        const { std::cell::RefCell::new(None) };
}

#[gdextension]
unsafe impl ExtensionLibrary for AsepriteGdExtension {
    fn on_stage_init(stage: InitStage) {
        // Outside the editor, register a ResourceFormatLoader so plain
        // load()/preload() of .aseprite paths works; in the editor the
        // import pipeline owns these files.
        if stage == InitStage::Scene && !godot::classes::Engine::singleton().is_editor_hint() {
            let loader = runtime::AseResourceLoader::new_gd();
            godot::classes::ResourceLoader::singleton().add_resource_format_loader(
                &loader
                    .clone()
                    .upcast::<godot::classes::ResourceFormatLoader>(),
            );
            RUNTIME_LOADER.with(|l| *l.borrow_mut() = Some(loader));
        }
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Scene
            && let Some(loader) = RUNTIME_LOADER.with(|l| l.borrow_mut().take())
        {
            godot::classes::ResourceLoader::singleton().remove_resource_format_loader(
                &loader.upcast::<godot::classes::ResourceFormatLoader>(),
            );
        }
    }
}
