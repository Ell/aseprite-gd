//! Golden-image tests: our compositor's output must be byte-identical to
//! Aseprite's own flattened render of the same file (the .png next to each
//! generated fixture, produced by tools/corpus/generate.sh).

use std::path::PathBuf;

use ase_core::composite::render_frame;
use ase_core::AseFile;

fn load_png_rgba(path: &PathBuf) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        other => panic!("{}: unexpected png color type {other:?}", path.display()),
    };
    (info.width, info.height, rgba)
}

#[test]
fn generated_fixtures_render_pixel_identical_to_aseprite() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/generated");
    let mut checked = 0;
    let mut entries: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();

    for path in entries {
        if path.extension().and_then(|e| e.to_str()) != Some("aseprite") {
            continue;
        }
        let golden_path = path.with_extension("png");
        let data = std::fs::read(&path).unwrap();
        let file = AseFile::parse(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let ours = render_frame(&file, 0).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let (gw, gh, golden) = load_png_rgba(&golden_path);

        assert_eq!((ours.width, ours.height), (gw, gh), "{}: size", path.display());
        if ours.pixels != golden {
            let (mut worst, mut worst_i) = (0i32, 0usize);
            let mut diffs = 0usize;
            for (i, (a, b)) in ours.pixels.iter().zip(&golden).enumerate() {
                let d = (*a as i32 - *b as i32).abs();
                if d > 0 {
                    diffs += 1;
                }
                if d > worst {
                    (worst, worst_i) = (d, i);
                }
            }
            let px = worst_i / 4;
            panic!(
                "{}: {diffs} byte diffs, worst Δ{worst} at pixel ({},{}) chan {} — ours {:?} vs golden {:?}",
                path.display(),
                px % gw as usize,
                px / gw as usize,
                worst_i % 4,
                &ours.pixels[px * 4..px * 4 + 4],
                &golden[px * 4..px * 4 + 4],
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 19, "expected all 19 blend-mode goldens");
}
