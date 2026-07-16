//! Script-facing access to parsed .aseprite files, for runtime loading
//! (mods, user content) and editor tooling written in GDScript/C#.

use godot::builtin::{GString, PackedStringArray, VarDictionary, Vector2i};
use godot::classes::{Image, ImageTexture, RefCounted};
use godot::prelude::*;

use crate::convert;

/// A parsed Aseprite document. Construct with [`AseDocument::open`].
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct AseDocument {
    inner: Option<ase_core::AseFile>,
    base: Base<RefCounted>,
}

#[godot_api]
impl AseDocument {
    /// Loads and parses a `.aseprite`/`.ase` file. Returns null on failure
    /// (details are logged).
    #[func]
    fn open(path: GString) -> Option<Gd<AseDocument>> {
        match convert::load_ase(&path) {
            Ok(file) => {
                let mut doc = AseDocument::new_gd();
                doc.bind_mut().inner = Some(file);
                Some(doc)
            }
            Err(e) => {
                godot_error!("AseDocument.open: {path}: {e}");
                None
            }
        }
    }

    fn file(&self) -> &ase_core::AseFile {
        self.inner.as_ref().expect("AseDocument used before open()")
    }

    #[func]
    fn get_size(&self) -> Vector2i {
        let h = &self.file().header;
        Vector2i::new(h.width as i32, h.height as i32)
    }

    #[func]
    fn get_frame_count(&self) -> i64 {
        self.file().frames.len() as i64
    }

    #[func]
    fn get_frame_duration_ms(&self, frame: i64) -> i64 {
        self.file()
            .frames
            .get(frame as usize)
            .map_or(0, |f| f.duration_ms as i64)
    }

    #[func]
    fn get_layer_names(&self) -> PackedStringArray {
        self.file()
            .layers
            .iter()
            .map(|l| GString::from(l.name.as_str()))
            .collect()
    }

    #[func]
    fn get_tag_names(&self) -> PackedStringArray {
        self.file()
            .tags
            .iter()
            .map(|t| GString::from(t.name.as_str()))
            .collect()
    }

    /// Tag frame range as (from, to), inclusive. (-1, -1) if unknown.
    #[func]
    fn get_tag_range(&self, name: GString) -> Vector2i {
        let name = name.to_string();
        self.file()
            .tags
            .iter()
            .find(|t| t.name == name)
            .map(|t| Vector2i::new(t.from_frame as i32, t.to_frame as i32))
            .unwrap_or(Vector2i::new(-1, -1))
    }

    /// Sprite-level user data as {"text": ..., "color": ...} (missing keys
    /// omitted).
    #[func]
    fn get_user_data(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let ud = &self.file().user_data;
        if let Some(text) = &ud.text {
            d.set(&"text".to_variant(), &text.as_str().to_variant());
        }
        if let Some([r, g, b, a]) = ud.color {
            let color = godot::builtin::Color::from_rgba8(r, g, b, a);
            d.set(&"color".to_variant(), &color.to_variant());
        }
        d
    }

    /// Flattens one frame to an Image (RGBA8), exactly as Aseprite renders it.
    #[func]
    fn render_frame(&self, frame: i64) -> Option<Gd<Image>> {
        match convert::frame_to_image(self.file(), frame.max(0) as usize) {
            Ok(img) => Some(img),
            Err(e) => {
                godot_error!("AseDocument.render_frame({frame}): {e}");
                None
            }
        }
    }

    /// Convenience: rendered frame as a ready-to-use texture.
    #[func]
    fn render_texture(&self, frame: i64) -> Option<Gd<ImageTexture>> {
        match convert::texture_for_frame(self.file(), frame.max(0) as usize) {
            Ok(t) => Some(t),
            Err(e) => {
                godot_error!("AseDocument.render_texture({frame}): {e}");
                None
            }
        }
    }
}
