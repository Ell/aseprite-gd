//! `ase` — development inspector for .aseprite files.
//!
//! Usage: ase info <file.aseprite>
//!
//! Grows with the parser: `dump-chunks`, `render`, etc. will land here and
//! drive the golden-image test suite.

use std::process::ExitCode;

use ase_core::parse::{parse_frame_header, parse_header, FRAME_HEADER_SIZE, HEADER_SIZE};
use ase_core::read::Reader;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("info") if args.len() == 3 => info(&args[2]),
        _ => {
            eprintln!("usage: ase info <file.aseprite>");
            ExitCode::from(2)
        }
    }
}

fn info(path: &str) -> ExitCode {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut r = Reader::new(&data);
    let header = match parse_header(&mut r) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!("{path}");
    println!("  canvas:     {}x{} ({:?})", header.width, header.height, header.color_depth);
    println!("  frames:     {}", header.frames);
    println!("  colors:     {}", header.num_colors);
    println!(
        "  flags:      layer_opacity={} group_blend={} layer_uuids={}",
        header.layer_opacity_valid(),
        header.group_blend_valid(),
        header.layers_have_uuid()
    );
    if header.grid_width != 0 {
        println!(
            "  grid:       {}x{} at ({}, {})",
            header.grid_width, header.grid_height, header.grid_x, header.grid_y
        );
    }

    // Walk frames by declared sizes (chunk parsing lands later).
    let mut offset = HEADER_SIZE;
    for i in 0..header.frames {
        if r.seek(offset).is_err() {
            eprintln!("error: frame {i} starts past end of file");
            return ExitCode::FAILURE;
        }
        match parse_frame_header(&mut r) {
            Ok(fh) => {
                let duration = if fh.duration_ms == 0 {
                    header.default_frame_duration_ms
                } else {
                    fh.duration_ms
                };
                println!(
                    "  frame {i:>3}:  {} chunks, {duration} ms, {} bytes",
                    fh.num_chunks, fh.frame_bytes
                );
                if (fh.frame_bytes as usize) < FRAME_HEADER_SIZE {
                    eprintln!("error: frame {i} declares impossible size");
                    return ExitCode::FAILURE;
                }
                offset += fh.frame_bytes as usize;
            }
            Err(e) => {
                eprintln!("error: frame {i}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
