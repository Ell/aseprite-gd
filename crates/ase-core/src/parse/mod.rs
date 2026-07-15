//! Binary decoding of the file structure. One module per concern; each states
//! the spec section it implements.

pub mod cel;
pub mod chunk;
pub mod frame;
pub mod header;
pub mod layer;
pub mod palette;
pub mod tags;
pub mod tileset;

pub use cel::parse_cel;
pub use frame::parse_frame_header;
pub use header::parse_header;
pub use layer::parse_layer;
pub use palette::{apply_new_palette, apply_old_palette};
pub use tags::parse_tags;
pub use tileset::parse_tileset;

/// File header magic (§3).
pub const FILE_MAGIC: u16 = 0xA5E0;
/// Frame header magic (§4).
pub const FRAME_MAGIC: u16 = 0xF1FA;
/// Size of the file header; parsing always seeks here afterward (§3).
pub const HEADER_SIZE: usize = 128;
/// Size of a frame header (§4).
pub const FRAME_HEADER_SIZE: usize = 16;
