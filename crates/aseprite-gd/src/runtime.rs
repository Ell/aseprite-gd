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

    /// All slices with the key in effect at `frame`. Each entry:
    /// {"name", "rect": Rect2i, "center": Rect2i (9-patch only),
    ///  "pivot": Vector2i (if set), "text": String (if set)}.
    /// Slices hidden at this frame are omitted.
    #[func]
    fn get_slices(&self, frame: i64) -> godot::builtin::VarArray {
        use godot::builtin::{Rect2i, Vector2i};
        let mut out = godot::builtin::VarArray::new();
        for slice in &self.file().slices {
            let Some(key) = slice.key_for(frame.max(0) as u32) else {
                continue;
            };
            if key.width == 0 || key.height == 0 {
                continue; // hidden from this frame on (§6.12)
            }
            let mut d = VarDictionary::new();
            d.set(&"name".to_variant(), &slice.name.as_str().to_variant());
            d.set(
                &"rect".to_variant(),
                &Rect2i::new(
                    Vector2i::new(key.x, key.y),
                    Vector2i::new(key.width as i32, key.height as i32),
                )
                .to_variant(),
            );
            if let Some((cx, cy, cw, ch)) = key.center {
                d.set(
                    &"center".to_variant(),
                    &Rect2i::new(Vector2i::new(cx, cy), Vector2i::new(cw as i32, ch as i32))
                        .to_variant(),
                );
            }
            if let Some((px, py)) = key.pivot {
                d.set(&"pivot".to_variant(), &Vector2i::new(px, py).to_variant());
            }
            if let Some(text) = &slice.user_data.text {
                d.set(&"text".to_variant(), &text.as_str().to_variant());
            }
            out.push(&d.to_variant());
        }
        out
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

/// Runtime loader: makes plain `load("....aseprite")` work in running games
/// (exported or `--script`), returning the composited first frame as an
/// ImageTexture. Registered only outside the editor — in the editor the
/// import pipeline owns these files. Loader methods may be called from
/// background threads (gdext#597); this type is stateless.
#[derive(GodotClass)]
#[class(init, base=ResourceFormatLoader)]
pub struct AseResourceLoader {
    base: Base<godot::classes::ResourceFormatLoader>,
}

#[godot_api]
impl godot::classes::IResourceFormatLoader for AseResourceLoader {
    fn get_recognized_extensions(&self) -> PackedStringArray {
        crate::import::recognized_extensions()
    }

    fn handles_type(&self, ty: StringName) -> bool {
        ty == "Texture2D" || ty == "ImageTexture"
    }

    fn get_resource_type(&self, path: GString) -> GString {
        let p = path.to_string().to_lowercase();
        if p.ends_with(".aseprite") || p.ends_with(".ase") {
            "ImageTexture".into()
        } else {
            "".into()
        }
    }

    fn load(
        &self,
        path: GString,
        _original_path: GString,
        _use_sub_threads: bool,
        _cache_mode: i32,
    ) -> Variant {
        match convert::load_ase(&path).and_then(|f| convert::texture_for_frame(&f, 0)) {
            Ok(texture) => texture.to_variant(),
            Err(e) => {
                godot_error!("aseprite-gd runtime load: {path}: {e}");
                (godot::global::Error::ERR_FILE_CORRUPT.ord() as i64).to_variant()
            }
        }
    }
}
