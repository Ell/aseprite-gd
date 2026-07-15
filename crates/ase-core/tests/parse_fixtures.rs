//! Corpus smoke test: every fixture must header-parse and frame-walk cleanly.
//! Grows with the parser — once chunk decoding lands, this walks chunks too;
//! once compositing lands, `generated/*.png` goldens get compared pixel-exact.

use std::path::PathBuf;

use ase_core::parse::{parse_frame_header, parse_header, HEADER_SIZE};
use ase_core::read::Reader;

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
fn all_fixtures_header_parse_and_frame_walk() {
    let files = fixture_files();
    assert!(files.len() >= 60, "expected the full corpus, found {} files", files.len());

    for path in files {
        let data = std::fs::read(&path).unwrap();
        let mut r = Reader::new(&data);
        let header = parse_header(&mut r)
            .unwrap_or_else(|e| panic!("{}: header: {e}", path.display()));
        assert!(header.frames > 0, "{}: zero frames", path.display());

        let mut offset = HEADER_SIZE;
        for i in 0..header.frames {
            r.seek(offset)
                .unwrap_or_else(|e| panic!("{}: frame {i} offset: {e}", path.display()));
            let fh = parse_frame_header(&mut r)
                .unwrap_or_else(|e| panic!("{}: frame {i}: {e}", path.display()));
            offset += fh.frame_bytes as usize;
        }
        assert!(
            offset <= data.len(),
            "{}: frames overrun file ({offset} > {})",
            path.display(),
            data.len()
        );
    }
}
