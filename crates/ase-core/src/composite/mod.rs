//! Frame compositing: flattening layers/cels into RGBA images the same way
//! Aseprite renders them (§8).

pub mod blend;
pub mod render;

pub use render::{render_frame, RenderError, RgbaImage};
