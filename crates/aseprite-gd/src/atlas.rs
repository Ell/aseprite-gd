//! Frame atlas packing: trim transparent borders, dedup identical frames
//! (linked cels collapse naturally), shelf-pack into one sheet. Pure data —
//! no Godot types — so it stays unit-testable.
//!
//! Offsets survive trimming: each placement records where the trimmed rect
//! sat in the original canvas, which the Godot layer turns into AtlasTexture
//! margins so frames render at full canvas size.

use std::collections::HashMap;

use ase_core::composite::RgbaImage;

/// A trimmed frame's pixels plus its position in the source canvas.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Trimmed {
    pub width: u32,
    pub height: u32,
    /// Offset of the trimmed rect inside the original canvas.
    pub offset_x: u32,
    pub offset_y: u32,
    pub pixels: Vec<u8>,
}

/// Where a unique image landed in the atlas.
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    /// Which page of the atlas holds this image.
    pub page: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

/// One texture page. Pages stay under the dimension cap passed to [`pack`]
/// (Godot rejects textures above 16384px).
pub struct AtlasPage {
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, row-major.
    pub pixels: Vec<u8>,
}

pub struct Atlas {
    pub pages: Vec<AtlasPage>,
    /// One placement per unique image.
    pub placements: Vec<Placement>,
    /// Maps each input frame to its placement index.
    pub frame_to_placement: Vec<usize>,
}

/// Tight bounding box of non-transparent pixels. Fully-transparent frames
/// become a 1x1 transparent rect so downstream regions stay valid.
fn trim(img: &RgbaImage) -> Trimmed {
    let (w, h) = (img.width as usize, img.height as usize);
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if img.pixels[(y * w + x) * 4 + 3] != 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x {
        return Trimmed {
            width: 1,
            height: 1,
            offset_x: 0,
            offset_y: 0,
            pixels: vec![0; 4],
        };
    }
    let (tw, th) = (max_x - min_x + 1, max_y - min_y + 1);
    let mut pixels = Vec::with_capacity(tw * th * 4);
    for y in min_y..=max_y {
        let row = &img.pixels[(y * w + min_x) * 4..(y * w + max_x + 1) * 4];
        pixels.extend_from_slice(row);
    }
    Trimmed {
        width: tw as u32,
        height: th as u32,
        offset_x: min_x as u32,
        offset_y: min_y as u32,
        pixels,
    }
}

/// Packs frames into atlas pages no larger than `max_dim` on either side.
/// `padding` pixels of transparent space separate placements (bleed
/// protection when filtering).
pub fn pack(frames: &[RgbaImage], padding: u32, max_dim: u32) -> Atlas {
    // Dedup identical trimmed frames, preserving first-seen order for
    // determinism.
    let mut unique: Vec<Trimmed> = Vec::new();
    let mut seen: HashMap<Trimmed, usize> = HashMap::new();
    let mut frame_to_placement = Vec::with_capacity(frames.len());
    for frame in frames {
        let t = trim(frame);
        let idx = *seen.entry(t.clone()).or_insert_with(|| {
            unique.push(t);
            unique.len() - 1
        });
        frame_to_placement.push(idx);
    }

    // Shelf packing, tallest first (stable order for determinism).
    let mut order: Vec<usize> = (0..unique.len()).collect();
    order.sort_by_key(|&i| {
        (
            std::cmp::Reverse(unique[i].height),
            std::cmp::Reverse(unique[i].width),
            i,
        )
    });

    // Page width: roughly square by total area, rounded up to a multiple of
    // 4, at least as wide as the widest image, capped at max_dim.
    let total_area: u64 = unique
        .iter()
        .map(|t| (t.width + padding) as u64 * (t.height + padding) as u64)
        .sum();
    let max_w = unique.iter().map(|t| t.width).max().unwrap_or(1) + padding;
    let atlas_w = ((total_area as f64).sqrt().ceil() as u32)
        .next_multiple_of(4)
        .max(max_w)
        .min(max_dim);

    let mut placements = vec![
        Placement {
            page: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            offset_x: 0,
            offset_y: 0
        };
        unique.len()
    ];
    // Shelf-fill pages; a shelf that would push past max_dim vertically
    // starts a new page.
    let mut page_dims: Vec<(u32, u32)> = vec![(0, 0)]; // (used_w, used_h) per page
    let (mut page, mut cur_x, mut cur_y, mut shelf_h) = (0usize, 0u32, 0u32, 0u32);
    for &i in &order {
        let t = &unique[i];
        if cur_x + t.width > atlas_w {
            cur_y += shelf_h;
            cur_x = 0;
            shelf_h = 0;
        }
        if cur_y + t.height > max_dim {
            page += 1;
            page_dims.push((0, 0));
            cur_x = 0;
            cur_y = 0;
            shelf_h = 0;
        }
        placements[i] = Placement {
            page,
            x: cur_x,
            y: cur_y,
            width: t.width,
            height: t.height,
            offset_x: t.offset_x,
            offset_y: t.offset_y,
        };
        cur_x += t.width + padding;
        shelf_h = shelf_h.max(t.height + padding);
        let d = &mut page_dims[page];
        d.0 = d.0.max(cur_x);
        d.1 = d.1.max(cur_y + t.height);
    }

    let mut pages: Vec<AtlasPage> = page_dims
        .iter()
        .map(|&(w, h)| {
            let width = w.saturating_sub(padding).max(1);
            let height = h.max(1);
            AtlasPage {
                width,
                height,
                pixels: vec![0u8; width as usize * height as usize * 4],
            }
        })
        .collect();
    for (i, t) in unique.iter().enumerate() {
        let p = &placements[i];
        let pg = &mut pages[p.page];
        for row in 0..t.height as usize {
            let src = &t.pixels[row * t.width as usize * 4..][..t.width as usize * 4];
            let dst = ((p.y as usize + row) * pg.width as usize + p.x as usize) * 4;
            pg.pixels[dst..dst + src.len()].copy_from_slice(src);
        }
    }

    Atlas {
        pages,
        placements,
        frame_to_placement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32, rect: (u32, u32, u32, u32)) -> RgbaImage {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = ((y * w + x) * 4) as usize;
                pixels[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
        RgbaImage {
            width: w,
            height: h,
            pixels,
        }
    }

    #[test]
    fn trims_to_content_and_keeps_offset() {
        let t = trim(&img(16, 16, (3, 5, 4, 2)));
        assert_eq!((t.width, t.height), (4, 2));
        assert_eq!((t.offset_x, t.offset_y), (3, 5));
    }

    #[test]
    fn empty_frame_becomes_1x1() {
        let t = trim(&img(8, 8, (0, 0, 0, 0)));
        assert_eq!((t.width, t.height, t.pixels.len()), (1, 1, 4));
    }

    #[test]
    fn identical_frames_dedup() {
        let frames = vec![
            img(16, 16, (2, 2, 4, 4)),
            img(16, 16, (2, 2, 4, 4)),
            img(16, 16, (8, 8, 2, 2)),
        ];
        let atlas = pack(&frames, 1, 16384);
        assert_eq!(atlas.placements.len(), 2, "two unique images");
        assert_eq!(atlas.frame_to_placement, vec![0, 0, 1]);
    }

    #[test]
    fn packing_is_deterministic_and_lossless() {
        let frames: Vec<RgbaImage> = (0..6).map(|i| img(32, 32, (i, i, 5 + i, 3 + i))).collect();
        let a = pack(&frames, 1, 16384);
        let b = pack(&frames, 1, 16384);
        assert_eq!(a.pages[0].pixels, b.pages[0].pixels);
        assert_eq!(a.pages[0].width, b.pages[0].width);
        // Every placement's pixels must match its trimmed source.
        for (f, &pi) in a.frame_to_placement.iter().enumerate() {
            let p = &a.placements[pi];
            let t = trim(&frames[f]);
            let a = &a.pages[p.page];
            for row in 0..p.height as usize {
                let atlas_row = &a.pixels
                    [((p.y as usize + row) * a.width as usize + p.x as usize) * 4..]
                    [..p.width as usize * 4];
                let src_row = &t.pixels[row * t.width as usize * 4..][..t.width as usize * 4];
                assert_eq!(atlas_row, src_row);
            }
        }
    }

    #[test]
    fn splits_pages_at_dimension_cap() {
        // Six 10x10 solid frames, max_dim 24: pages hold at most 2 columns x
        // 2 shelves = 4 tiles, so 6 distinct frames need 2 pages.
        let frames: Vec<RgbaImage> = (0..6)
            .map(|i| {
                let mut f = img(10, 10, (0, 0, 10, 10));
                f.pixels[0] = i as u8; // make each frame unique
                f
            })
            .collect();
        let atlas = pack(&frames, 1, 24);
        assert!(atlas.pages.len() > 1, "expected a page split");
        for page in &atlas.pages {
            assert!(page.width <= 24 && page.height <= 24);
        }
        // Every frame still maps to a valid placement on some page.
        for &pi in &atlas.frame_to_placement {
            let p = &atlas.placements[pi];
            assert!(p.page < atlas.pages.len());
        }
    }
}
