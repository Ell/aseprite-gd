//! Golden-image tests: our compositor's output must be byte-identical to
//! Aseprite's own flattened render of the same files.
//!
//! Two golden sets, both produced by tools/corpus/generate.sh from the same
//! local Aseprite: `generated/*.png` (blend-mode matrix, frame 0) and
//! `goldens/<fixture>@<frame>.png` (every frame of every vendored fixture).

use std::path::{Path, PathBuf};

use ase_core::AseFile;
use ase_core::composite::render_frame;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_png_rgba(path: &Path) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        other => panic!("{}: unexpected png color type {other:?}", path.display()),
    };
    (info.width, info.height, rgba)
}

/// Renders `frame` of `ase_path` and diffs against the golden PNG. Returns an
/// error description instead of panicking so callers can aggregate.
fn check(ase_path: &Path, frame: usize, golden_path: &Path) -> Result<(), String> {
    let data = std::fs::read(ase_path).unwrap();
    let file = AseFile::parse(&data).map_err(|e| format!("parse: {e}"))?;
    let ours = render_frame(&file, frame).map_err(|e| format!("render: {e}"))?;
    let (gw, gh, golden) = load_png_rgba(golden_path);

    if (ours.width, ours.height) != (gw, gh) {
        return Err(format!(
            "size {}x{} vs golden {gw}x{gh}",
            ours.width, ours.height
        ));
    }
    // Pixels transparent on both sides count as equal regardless of RGB:
    // straight alpha means Aseprite's exports carry meaningless source RGB
    // under alpha 0 (e.g. the palette color of the transparent index), while
    // our canvas starts zeroed.
    let (mut worst, mut worst_i) = (0i32, 0usize);
    let mut diffs = 0usize;
    for (px, (a, b)) in ours
        .pixels
        .chunks_exact(4)
        .zip(golden.chunks_exact(4))
        .enumerate()
    {
        if a[3] == 0 && b[3] == 0 {
            continue;
        }
        for c in 0..4 {
            let d = (a[c] as i32 - b[c] as i32).abs();
            if d > 0 {
                diffs += 1;
            }
            if d > worst {
                (worst, worst_i) = (d, px * 4 + c);
            }
        }
    }
    if diffs == 0 {
        return Ok(());
    }
    let px = worst_i / 4;
    Err(format!(
        "{diffs} byte diffs, worst Δ{worst} at ({},{}) chan {} — ours {:?} vs golden {:?}",
        px % gw as usize,
        px / gw as usize,
        worst_i % 4,
        &ours.pixels[px * 4..px * 4 + 4],
        &golden[px * 4..px * 4 + 4],
    ))
}

#[test]
fn generated_fixtures_render_pixel_identical_to_aseprite() {
    let dir = fixtures_root().join("generated");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();

    let mut checked = 0;
    for path in entries {
        if path.extension().and_then(|e| e.to_str()) != Some("aseprite") {
            continue;
        }
        if let Err(e) = check(&path, 0, &path.with_extension("png")) {
            panic!("{}: {e}", path.display());
        }
        checked += 1;
    }
    assert_eq!(checked, 23, "expected 19 blend-mode + 4 feature goldens");
}

#[test]
fn vendored_fixtures_render_pixel_identical_to_aseprite() {
    let root = fixtures_root();
    let goldens = root.join("goldens");
    let mut entries: Vec<_> = std::fs::read_dir(&goldens)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    assert!(
        entries.len() >= 100,
        "goldens missing — run tools/corpus/generate.sh"
    );

    let mut failures = Vec::new();
    let mut checked = 0;
    for golden_path in entries {
        let stem = golden_path.file_stem().unwrap().to_str().unwrap();
        let (name, frame) = stem
            .rsplit_once('@')
            .expect("golden name shape <fixture>@<frame>");
        let frame: usize = frame.parse().unwrap();

        let root = &root;
        let ase_path = ["aseprite-tests", "asefile"]
            .iter()
            .flat_map(|d| {
                ["aseprite", "ase"]
                    .iter()
                    .map(move |e| root.join(d).join(format!("{name}.{e}")))
            })
            .find(|p| p.exists())
            .unwrap_or_else(|| panic!("no fixture for golden {stem}"));

        checked += 1;
        if let Err(e) = check(&ase_path, frame, &golden_path) {
            failures.push(format!("{name}@{frame}: {e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{}/{checked} golden mismatches:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
