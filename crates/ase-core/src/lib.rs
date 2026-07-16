//! Parser and compositor for Aseprite (`.aseprite` / `.ase`) files.
//!
//! Parses the binary format directly — every chunk type, all three color
//! modes, tilemaps, slices, and typed user data — and composites frames
//! pixel-identically to Aseprite itself (all 19 blend modes, exact integer
//! math, verified against Aseprite's own renders).
//!
//! ```no_run
//! let data = std::fs::read("sprite.aseprite").unwrap();
//! let file = ase_core::AseFile::parse(&data).unwrap();
//! let frame0 = ase_core::composite::render_frame(&file, 0).unwrap();
//! assert_eq!(frame0.pixels.len(), frame0.width as usize * frame0.height as usize * 4);
//! ```
//!
//! Input is treated as untrusted: reads are bounds-checked, decompression and
//! recursion are capped (see [`limits`]), and the parser is fuzz-tested.
//! Errors carry absolute file offsets.
//!
//! This crate has no engine dependencies. The implementation spec is
//! `docs/ase-format-reference.md` in the repository; section references in
//! comments (e.g. "§6.3") point into that document.

pub mod composite;
pub mod error;
pub mod file;
pub mod limits;
pub mod model;
pub mod parse;
pub mod read;

pub use error::ParseError;
pub use file::AseFile;

/// Result alias used throughout the parsing code.
pub type Result<T> = std::result::Result<T, ParseError>;
