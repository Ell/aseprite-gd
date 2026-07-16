//! Hard caps applied while parsing untrusted files (mods, downloads, runtime
//! loading). These are deliberately generous for legitimate art and deliberately
//! fatal for decompression bombs. See "Hostile-file safety" in docs/architecture.md.

/// Maximum canvas dimension (width or height) we accept. Aseprite's own UI
/// caps canvases well below this; Godot cannot use textures above 16384 anyway.
pub const MAX_CANVAS_DIM: u32 = 65_535;

/// Maximum bytes a single cel/tileset image may decompress to.
/// A 4096×4096 RGBA cel is 64 MiB; nothing legitimate exceeds this.
pub const MAX_IMAGE_BYTES: usize = 256 * 1024 * 1024;

/// Maximum total bytes decompressed across an entire file.
pub const MAX_TOTAL_DECOMPRESSED_BYTES: usize = 1024 * 1024 * 1024;

/// Maximum nesting depth of user-data property maps (§6.11).
/// Aseprite itself throws beyond 128 levels.
pub const MAX_PROPERTY_DEPTH: u32 = 128;

/// Maximum palette entries we accept (the format allows >256; §6.10).
pub const MAX_PALETTE_ENTRIES: u32 = 65_536;

/// Maximum tiles in one tileset (§6.13). Bounds both the decompressed strip
/// size and the per-tile user data allocation — external tilesets carry no
/// pixel data, so `num_tiles` would otherwise be an unchecked allocation size.
pub const MAX_TILESET_TILES: u32 = 1 << 20;
