//! Parser and compositor for Aseprite (`.aseprite` / `.ase`) files.
//!
//! This crate has no engine dependencies. The Godot integration lives in the
//! `aseprite-gd` crate; anything reusable outside Godot belongs here.
//!
//! The implementation spec is `docs/ase-format-reference.md` at the repo root.
//! Section references in comments (e.g. "§6.3") point into that document.

pub mod error;
pub mod limits;
pub mod model;
pub mod parse;
pub mod read;

pub use error::ParseError;

/// Result alias used throughout the parsing code.
pub type Result<T> = std::result::Result<T, ParseError>;
