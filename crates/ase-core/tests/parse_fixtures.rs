//! Corpus test: every fixture must fully parse — all chunks decoded, all cel
//! zlib streams inflated. This is the parser's reality check against real
//! Aseprite output across format eras.

use std::path::PathBuf;

use ase_core::AseFile;

fn fixture_files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut out = Vec::new();
    for dir in ["aseprite-tests", "asefile", "generated"] {
        let dir = root.join(dir);
        for entry in std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("missing fixture dir {}: {e}", dir.display()))
        {
            let path = entry.unwrap().path();
            match path.extension().and_then(|e| e.to_str()) {
                Some("aseprite") | Some("ase") => out.push(path),
                _ => {}
            }
        }
    }
    out.sort();
    out
}

#[test]
fn all_fixtures_fully_parse() {
    let files = fixture_files();
    assert!(
        files.len() >= 60,
        "expected the full corpus, found {} files",
        files.len()
    );

    for path in files {
        let data = std::fs::read(&path).unwrap();
        let file = AseFile::parse(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

        assert_eq!(
            file.frames.len(),
            file.header.frames as usize,
            "{}: frame count mismatch",
            path.display()
        );
        assert!(!file.layers.is_empty(), "{}: no layers", path.display());
        for frame in &file.frames {
            assert!(
                frame.duration_ms > 0,
                "{}: zero frame duration",
                path.display()
            );
            for cel in &frame.cels {
                assert!(
                    cel.layer_index < file.layers.len(),
                    "{}: cel layer index out of range",
                    path.display()
                );
            }
        }
    }
}
