//! Oracle cross-check: every fixture is parsed by our parser AND by two
//! independent Rust implementations (`asefile`, `aseprite-loader`); the model
//! data both sides expose must agree.
//!
//! Skip lists are exact. A file goes on an oracle's skip list only when that
//! oracle itself fails to parse it, and the test asserts the skip list equals
//! the observed failure set — so a new oracle failure fails the test, and a
//! fixture the oracle *starts* handling fails it too (stale entry).

use std::collections::BTreeSet;
use std::path::PathBuf;

use ase_core::AseFile;
use ase_core::model::{AniDir, ColorDepth};

/// Fixtures `asefile` 0.3.8 fails to parse. These are asefile limitations or
/// bugs, not format violations — our parser and aseprite-loader handle them.
const ASEFILE_SKIP: &[&str] = &[
    // asefile rejects embedded ICC color profiles outright ("Unsupported
    // Aseprite feature"); we parse the 0x2007 chunk and keep the ICC blob.
    "asefile/color-curve.aseprite",
];

/// Fixtures `aseprite-loader` 0.4.2 (`binary::file::parse_file`) fails to
/// parse.
const LOADER_SKIP: &[&str] = &[];

const FIXTURE_DIRS: &[&str] = &["aseprite-tests", "asefile", "generated"];

/// All corpus files as `(key, path)` where the key is `"<dir>/<file name>"`.
fn fixture_files() -> Vec<(String, PathBuf)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut out = Vec::new();
    for dir in FIXTURE_DIRS {
        for entry in std::fs::read_dir(root.join(dir))
            .unwrap_or_else(|e| panic!("missing fixture dir {dir}: {e}"))
        {
            let path = entry.unwrap().path();
            if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("aseprite" | "ase")
            ) {
                let key = format!("{dir}/{}", path.file_name().unwrap().to_str().unwrap());
                out.push((key, path));
            }
        }
    }
    out.sort();
    assert!(
        out.len() >= 60,
        "expected the full corpus, found {} files",
        out.len()
    );
    out
}

/// Assert the observed oracle failure set matches the declared skip list
/// exactly, in both directions.
fn assert_skip_list_exact(oracle: &str, skip: &[&str], failed: &BTreeSet<String>) {
    let declared: BTreeSet<String> = skip.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        declared.len(),
        skip.len(),
        "{oracle}: duplicate entries in skip list"
    );
    let new_failures: Vec<_> = failed.difference(&declared).collect();
    let stale: Vec<_> = declared.difference(failed).collect();
    assert!(
        new_failures.is_empty() && stale.is_empty(),
        "{oracle} skip list out of date.\n  new oracle failures (add with reason): {new_failures:?}\n  now parsing fine (remove): {stale:?}"
    );
}

// ---------------------------------------------------------------------------
// asefile
// ---------------------------------------------------------------------------

#[test]
fn oracle_asefile() {
    let mut failed = BTreeSet::new();
    for (key, path) in fixture_files() {
        let data = std::fs::read(&path).unwrap();
        let ours = AseFile::parse(&data).unwrap_or_else(|e| panic!("{key}: our parser: {e}"));
        match asefile::AsepriteFile::read(&data[..]) {
            Ok(oracle) => compare_asefile(&key, &ours, &oracle),
            Err(e) => {
                eprintln!("asefile failed on {key}: {e}");
                failed.insert(key);
            }
        }
    }
    assert_skip_list_exact("asefile", ASEFILE_SKIP, &failed);
}

fn compare_asefile(key: &str, ours: &AseFile, oracle: &asefile::AsepriteFile) {
    assert_eq!(oracle.width(), ours.header.width as usize, "{key}: width");
    assert_eq!(
        oracle.height(),
        ours.header.height as usize,
        "{key}: height"
    );

    let depth = match oracle.pixel_format() {
        asefile::PixelFormat::Rgba => ColorDepth::Rgba,
        asefile::PixelFormat::Grayscale => ColorDepth::Grayscale,
        asefile::PixelFormat::Indexed {
            transparent_color_index,
        } => {
            assert_eq!(
                transparent_color_index, ours.header.transparent_index,
                "{key}: transparent index"
            );
            ColorDepth::Indexed
        }
    };
    assert_eq!(depth, ours.header.color_depth, "{key}: color depth");

    assert_eq!(
        oracle.num_frames() as usize,
        ours.frames.len(),
        "{key}: frame count"
    );
    for i in 0..oracle.num_frames() {
        let d = oracle.frame(i).duration();
        // asefile returns the raw per-frame duration word; ours substitutes the
        // header default when the stored value is 0 (gotcha #19), so a raw 0 is
        // not comparable.
        if d != 0 {
            assert_eq!(
                ours.frames[i as usize].duration_ms as u32, d,
                "{key}: frame {i} duration"
            );
        }
    }

    assert_eq!(
        oracle.num_layers() as usize,
        ours.layers.len(),
        "{key}: layer count"
    );
    for i in 0..oracle.num_layers() {
        assert_eq!(
            oracle.layer(i).name(),
            ours.layers[i as usize].name,
            "{key}: layer {i} name"
        );
    }

    assert_eq!(
        oracle.num_tags() as usize,
        ours.tags.len(),
        "{key}: tag count"
    );
    for i in 0..oracle.num_tags() {
        let t = oracle.tag(i);
        let o = &ours.tags[i as usize];
        assert_eq!(t.name(), o.name, "{key}: tag {i} name");
        assert_eq!(t.from_frame(), o.from_frame as u32, "{key}: tag {i} from");
        assert_eq!(t.to_frame(), o.to_frame as u32, "{key}: tag {i} to");
        assert_eq!(
            t.repeat().map_or(0, |r| r.get()),
            o.repeat as u32,
            "{key}: tag {i} repeat"
        );
        let dir = match t.animation_direction() {
            asefile::AnimationDirection::Forward => AniDir::Forward,
            asefile::AnimationDirection::Reverse => AniDir::Reverse,
            asefile::AnimationDirection::PingPong => AniDir::PingPong,
            // asefile has no PingPongReverse variant: it hard-errors on
            // direction byte 3, so such files sit on the skip list instead.
        };
        assert_eq!(dir, o.direction, "{key}: tag {i} direction");
    }

    let our_palette = ours.palettes.last().map_or(0, |p| p.entries.len());
    match oracle.palette() {
        Some(p) => assert_eq!(p.num_colors() as usize, our_palette, "{key}: palette size"),
        None => assert_eq!(our_palette, 0, "{key}: oracle saw no palette"),
    }
}

// ---------------------------------------------------------------------------
// aseprite-loader
// ---------------------------------------------------------------------------

#[test]
fn oracle_aseprite_loader() {
    let mut failed = BTreeSet::new();
    for (key, path) in fixture_files() {
        let data = std::fs::read(&path).unwrap();
        let ours = AseFile::parse(&data).unwrap_or_else(|e| panic!("{key}: our parser: {e}"));
        // The `binary` parser is the crate's full-fidelity layer; the `loader`
        // wrapper on top rejects whole feature classes (tilemap cels) by
        // design, which would say nothing about parsing correctness.
        match aseprite_loader::binary::file::parse_file(&data) {
            Ok(oracle) => compare_loader(&key, &ours, &oracle),
            Err(e) => {
                eprintln!("aseprite-loader failed on {key}: {e}");
                failed.insert(key);
            }
        }
    }
    assert_skip_list_exact("aseprite-loader", LOADER_SKIP, &failed);
}

fn compare_loader(key: &str, ours: &AseFile, oracle: &aseprite_loader::binary::file::File) {
    use aseprite_loader::binary::chunks::tags::AnimationDirection as Dir;

    let h = &oracle.header;
    assert_eq!(h.width, ours.header.width, "{key}: width");
    assert_eq!(h.height, ours.header.height, "{key}: height");
    assert_eq!(
        h.color_depth.bpp() as usize,
        ours.header.color_depth.bytes_per_pixel() * 8,
        "{key}: color depth"
    );

    assert_eq!(oracle.frames.len(), ours.frames.len(), "{key}: frame count");
    for (i, f) in oracle.frames.iter().enumerate() {
        // Raw duration; 0 falls back to the header speed field (gotcha #19).
        // `speed` is deprecated in-format, but it IS the fallback source.
        #[allow(deprecated)]
        let effective = if f.duration == 0 { h.speed } else { f.duration };
        assert_eq!(
            ours.frames[i].duration_ms, effective,
            "{key}: frame {i} duration"
        );
    }

    assert_eq!(oracle.layers.len(), ours.layers.len(), "{key}: layer count");
    for (i, l) in oracle.layers.iter().enumerate() {
        assert_eq!(l.name, ours.layers[i].name, "{key}: layer {i} name");
    }

    assert_eq!(oracle.tags.len(), ours.tags.len(), "{key}: tag count");
    for (i, t) in oracle.tags.iter().enumerate() {
        let o = &ours.tags[i];
        assert_eq!(t.name, o.name, "{key}: tag {i} name");
        assert_eq!(*t.frames.start(), o.from_frame, "{key}: tag {i} from");
        assert_eq!(*t.frames.end(), o.to_frame, "{key}: tag {i} to");
        assert_eq!(t.animation_repeat, o.repeat, "{key}: tag {i} repeat");
        let dir = match t.animation_direction {
            Dir::Forward => AniDir::Forward,
            Dir::Reverse => AniDir::Reverse,
            Dir::PingPong => AniDir::PingPong,
            Dir::PingPongReverse => AniDir::PingPongReverse,
            // Out-of-range decodes as Forward in Aseprite (gotcha #18).
            Dir::Unknown(_) => AniDir::Forward,
        };
        assert_eq!(dir, o.direction, "{key}: tag {i} direction");
    }

    // Palette entry count is NOT compared against this oracle by design: it
    // materializes a fixed 256-slot array (indexed sprites only) and exposes
    // no entry count.
}
