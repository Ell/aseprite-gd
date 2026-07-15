//! Pixel blenders, transliterated from Aseprite's blend_funcs.cpp so output is
//! bit-identical (§9 of the format reference). All arithmetic is the same
//! integer math (MUL_UN8/DIV_UN8 from pixman); HSL modes and soft light use
//! doubles exactly like upstream, including their truncation behavior.

use crate::model::BlendMode;

pub type Rgba = [u8; 4];

/// round(a*b/255) via pixman's shift trick (§9.1). Callers may pass negative
/// `a` (the merge blender does); i32 arithmetic shifts match C's behavior on
/// gcc/clang.
#[inline]
pub fn mul_un8(a: i32, b: i32) -> i32 {
    let t = a * b + 0x80;
    ((t >> 8) + t) >> 8
}

/// round(a*255/b), b > 0 (§9.1).
#[inline]
fn div_un8(a: i32, b: i32) -> i32 {
    (a * 0xFF + b / 2) / b
}

/// Straight-alpha "over" — the final step of every mode (§9.2).
pub fn blender_normal(back: Rgba, src: Rgba, opacity: i32) -> Rgba {
    if back[3] == 0 {
        return [src[0], src[1], src[2], mul_un8(src[3] as i32, opacity) as u8];
    }
    if src[3] == 0 {
        return back;
    }
    let ba = back[3] as i32;
    let sa = mul_un8(src[3] as i32, opacity);
    let ra = sa + ba - mul_un8(ba, sa);
    let ch = |b: u8, s: u8| (b as i32 + (s as i32 - b as i32) * sa / ra) as u8;
    [ch(back[0], src[0]), ch(back[1], src[1]), ch(back[2], src[2]), ra as u8]
}

/// Per-channel lerp by opacity, alpha lerped too (used by the `_n` blenders;
/// §9.4).
fn blender_merge(back: Rgba, src: Rgba, opacity: i32) -> Rgba {
    let (ba, sa) = (back[3] as i32, src[3] as i32);
    let (rr, rg, rb) = if ba == 0 {
        (src[0] as i32, src[1] as i32, src[2] as i32)
    } else if sa == 0 {
        (back[0] as i32, back[1] as i32, back[2] as i32)
    } else {
        let ch = |b: u8, s: u8| b as i32 + mul_un8(s as i32 - b as i32, opacity);
        (ch(back[0], src[0]), ch(back[1], src[1]), ch(back[2], src[2]))
    };
    let ra = ba + mul_un8(sa - ba, opacity);
    if ra == 0 {
        [0, 0, 0, 0]
    } else {
        [rr as u8, rg as u8, rb as u8, ra as u8]
    }
}

// --- separable per-channel blend functions (§9.3); b/s in 0..=255 ---

fn b_multiply(b: i32, s: i32) -> i32 {
    mul_un8(b, s)
}
fn b_screen(b: i32, s: i32) -> i32 {
    b + s - mul_un8(b, s)
}
fn b_hard_light(b: i32, s: i32) -> i32 {
    if s < 128 { b_multiply(b, s << 1) } else { b_screen(b, (s << 1) - 255) }
}
fn b_overlay(b: i32, s: i32) -> i32 {
    b_hard_light(s, b)
}
fn b_darken(b: i32, s: i32) -> i32 {
    b.min(s)
}
fn b_lighten(b: i32, s: i32) -> i32 {
    b.max(s)
}
fn b_color_dodge(b: i32, s: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    let s = 255 - s;
    if b >= s { 255 } else { div_un8(b, s) }
}
fn b_color_burn(b: i32, s: i32) -> i32 {
    if b == 255 {
        return 255;
    }
    let b = 255 - b;
    if b >= s { 0 } else { 255 - div_un8(b, s) }
}
fn b_soft_light(b: i32, s: i32) -> i32 {
    // W3C soft light in doubles, rounding as upstream (§9.3).
    let b = b as f64 / 255.0;
    let s = s as f64 / 255.0;
    let d = if b <= 0.25 { ((16.0 * b - 12.0) * b + 4.0) * b } else { b.sqrt() };
    let r = if s <= 0.5 { b - (1.0 - 2.0 * s) * b * (1.0 - b) } else { b + (2.0 * s - 1.0) * (d - b) };
    (r * 255.0 + 0.5) as i32
}
fn b_difference(b: i32, s: i32) -> i32 {
    (b - s).abs()
}
fn b_exclusion(b: i32, s: i32) -> i32 {
    b + s - 2 * mul_un8(b, s)
}
fn b_addition(b: i32, s: i32) -> i32 {
    (b + s).min(255)
}
fn b_subtract(b: i32, s: i32) -> i32 {
    (b - s).max(0)
}
fn b_divide(b: i32, s: i32) -> i32 {
    if b == 0 {
        0
    } else if b >= s {
        255
    } else {
        div_un8(b, s)
    }
}

// --- non-separable HSL helpers, doubles, channels in [0,1] (§9.3) ---

fn lum(r: f64, g: f64, b: f64) -> f64 {
    0.3 * r + 0.59 * g + 0.11 * b
}

fn sat(r: f64, g: f64, b: f64) -> f64 {
    r.max(g).max(b) - r.min(g).min(b)
}

fn clip_color(r: &mut f64, g: &mut f64, b: &mut f64) {
    let l = lum(*r, *g, *b);
    let n = r.min(*g).min(*b);
    let x = r.max(*g).max(*b);
    if n < 0.0 {
        *r = l + ((*r - l) * l) / (l - n);
        *g = l + ((*g - l) * l) / (l - n);
        *b = l + ((*b - l) * l) / (l - n);
    }
    if x > 1.0 {
        *r = l + ((*r - l) * (1.0 - l)) / (x - l);
        *g = l + ((*g - l) * (1.0 - l)) / (x - l);
        *b = l + ((*b - l) * (1.0 - l)) / (x - l);
    }
}

fn set_lum(r: &mut f64, g: &mut f64, b: &mut f64, l: f64) {
    let d = l - lum(*r, *g, *b);
    *r += d;
    *g += d;
    *b += d;
    clip_color(r, g, b);
}

fn set_sat(r: &mut f64, g: &mut f64, b: &mut f64, s: f64) {
    // Order the three channels, scale the mid, zero the min, set the max.
    let mut chans: [&mut f64; 3] = [r, g, b];
    chans.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let [min, mid, max] = chans;
    if *max > *min {
        *mid = ((*mid - *min) * s) / (*max - *min);
        *max = s;
    } else {
        *mid = 0.0;
        *max = 0.0;
    }
    *min = 0.0;
}

fn hsl_blend(mode: BlendMode, back: Rgba, src: Rgba) -> [u8; 3] {
    let (br, bg, bb) = (back[0] as f64 / 255.0, back[1] as f64 / 255.0, back[2] as f64 / 255.0);
    let (mut r, mut g, mut b) = (src[0] as f64 / 255.0, src[1] as f64 / 255.0, src[2] as f64 / 255.0);
    match mode {
        BlendMode::HslHue => {
            let (s, l) = (sat(br, bg, bb), lum(br, bg, bb));
            set_sat(&mut r, &mut g, &mut b, s);
            set_lum(&mut r, &mut g, &mut b, l);
        }
        BlendMode::HslSaturation => {
            let s = sat(r, g, b);
            let l = lum(br, bg, bb);
            (r, g, b) = (br, bg, bb);
            set_sat(&mut r, &mut g, &mut b, s);
            set_lum(&mut r, &mut g, &mut b, l);
        }
        BlendMode::HslColor => {
            let l = lum(br, bg, bb);
            set_lum(&mut r, &mut g, &mut b, l);
        }
        BlendMode::HslLuminosity => {
            let l = lum(r, g, b);
            (r, g, b) = (br, bg, bb);
            set_lum(&mut r, &mut g, &mut b, l);
        }
        _ => unreachable!(),
    }
    // Truncation matches upstream's (int)(255.0 * x).
    [(255.0 * r) as u8, (255.0 * g) as u8, (255.0 * b) as u8]
}

/// The legacy blender for a non-normal mode: substitute blended RGB for the
/// source (keeping source alpha), then normal-composite (§9.2).
fn blender_legacy(mode: BlendMode, back: Rgba, src: Rgba, opacity: i32) -> Rgba {
    let rgb: [u8; 3] = match mode {
        BlendMode::HslHue
        | BlendMode::HslSaturation
        | BlendMode::HslColor
        | BlendMode::HslLuminosity => hsl_blend(mode, back, src),
        _ => {
            let f = match mode {
                BlendMode::Multiply => b_multiply,
                BlendMode::Screen => b_screen,
                BlendMode::Overlay => b_overlay,
                BlendMode::Darken => b_darken,
                BlendMode::Lighten => b_lighten,
                BlendMode::ColorDodge => b_color_dodge,
                BlendMode::ColorBurn => b_color_burn,
                BlendMode::HardLight => b_hard_light,
                BlendMode::SoftLight => b_soft_light,
                BlendMode::Difference => b_difference,
                BlendMode::Exclusion => b_exclusion,
                BlendMode::Addition => b_addition,
                BlendMode::Subtract => b_subtract,
                BlendMode::Divide => b_divide,
                _ => unreachable!(),
            };
            let ch = |b: u8, s: u8| f(b as i32, s as i32) as u8;
            [ch(back[0], src[0]), ch(back[1], src[1]), ch(back[2], src[2])]
        }
    };
    blender_normal(back, [rgb[0], rgb[1], rgb[2], src[3]], opacity)
}

/// Public entry: the "new" (`_n`) blender Aseprite uses since v1.3 (§9.4).
/// Unknown modes degrade to Normal.
pub fn blend(mode: BlendMode, back: Rgba, src: Rgba, opacity: i32) -> Rgba {
    match mode {
        BlendMode::Normal | BlendMode::Unknown(_) => blender_normal(back, src, opacity),
        mode => {
            if back[3] == 0 {
                return blender_normal(back, src, opacity);
            }
            let normal = blender_normal(back, src, opacity);
            let blended = blender_legacy(mode, back, src, opacity);
            let ba = back[3] as i32;
            let m1 = blender_merge(normal, blended, ba);
            let src_total_alpha = mul_un8(src[3] as i32, opacity);
            let composite_alpha = mul_un8(ba, src_total_alpha);
            blender_merge(m1, blended, composite_alpha)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div_match_pixman_rounding() {
        assert_eq!(mul_un8(255, 255), 255);
        assert_eq!(mul_un8(255, 128), 128);
        assert_eq!(mul_un8(128, 128), 64); // round(128*128/255) = 64.25 -> 64
        assert_eq!(mul_un8(0, 200), 0);
        assert_eq!(div_un8(128, 255), 128);
        assert_eq!(div_un8(64, 128), 128); // round(64*255/128) = 127.5 -> 128
    }

    #[test]
    fn normal_over_transparent_backdrop_keeps_src_rgb() {
        let r = blender_normal([0, 0, 0, 0], [200, 100, 50, 255], 128);
        assert_eq!(r, [200, 100, 50, 128]);
    }

    #[test]
    fn normal_full_opacity_replaces() {
        let r = blender_normal([10, 20, 30, 255], [200, 100, 50, 255], 255);
        assert_eq!(r, [200, 100, 50, 255]);
    }

    #[test]
    fn opaque_backdrop_new_blend_equals_legacy() {
        // With Ba=255 the _n merges collapse to the legacy result (§9.4).
        let back = [100, 150, 200, 255];
        let src = [50, 60, 70, 200];
        for mode in [BlendMode::Multiply, BlendMode::Screen, BlendMode::Difference] {
            assert_eq!(
                blend(mode, back, src, 255),
                blender_legacy(mode, back, src, 255),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn multiply_on_opaque_matches_hand_math() {
        let r = blend(BlendMode::Multiply, [100, 100, 100, 255], [128, 255, 0, 255], 255);
        assert_eq!(r, [mul_un8(100, 128) as u8, 100, 0, 255]);
    }
}
