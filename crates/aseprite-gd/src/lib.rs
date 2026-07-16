//! Godot GDExtension entry point. This crate is a thin adapter: all parsing
//! and compositing lives in `ase-core`; modules here only translate to Godot
//! types and register editor/runtime integration.

use godot::prelude::*;

pub mod convert;
pub mod runtime;
pub mod import;
pub mod plugin;

struct AsepriteGdExtension;

#[gdextension]
unsafe impl ExtensionLibrary for AsepriteGdExtension {}
