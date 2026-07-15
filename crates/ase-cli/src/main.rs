//! `ase` — development inspector for .aseprite files.
//!
//! Usage:
//!   ase info <file.aseprite>
//!   ase render <file.aseprite> <frame> <out.png>

use std::process::ExitCode;

use ase_core::composite::render_frame;
use ase_core::AseFile;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("info") if args.len() == 3 => info(&args[2]),
        Some("render") if args.len() == 5 => render(&args[2], &args[3], &args[4]),
        _ => {
            eprintln!("usage: ase info <file.aseprite>");
            eprintln!("       ase render <file.aseprite> <frame> <out.png>");
            ExitCode::from(2)
        }
    }
}

fn parse_file(path: &str) -> Result<AseFile, ExitCode> {
    let data = std::fs::read(path).map_err(|e| {
        eprintln!("error: cannot read {path}: {e}");
        ExitCode::FAILURE
    })?;
    AseFile::parse(&data).map_err(|e| {
        eprintln!("error: {path}: {e}");
        ExitCode::FAILURE
    })
}

fn info(path: &str) -> ExitCode {
    let file = match parse_file(path) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let h = &file.header;

    println!("{path}");
    println!("  canvas:     {}x{} ({:?})", h.width, h.height, h.color_depth);
    println!("  frames:     {}", file.frames.len());
    println!("  palette:    {} entries", file.palette.entries.len());
    println!(
        "  flags:      layer_opacity={} group_blend={} layer_uuids={}",
        h.layer_opacity_valid(),
        h.group_blend_valid(),
        h.layers_have_uuid()
    );
    println!("  layers:");
    for (i, l) in file.layers.iter().enumerate() {
        println!(
            "    [{i}] {}{:?} \"{}\" blend={:?} opacity={}{}{}",
            "  ".repeat(l.child_level as usize),
            l.layer_type,
            l.name,
            l.blend_mode,
            l.opacity,
            if l.is_visible() { "" } else { " (hidden)" },
            if l.is_background() { " (background)" } else { "" },
        );
    }
    if !file.tags.is_empty() {
        println!("  tags:");
        for t in &file.tags {
            println!(
                "    \"{}\" frames {}..={} {:?} repeat={}",
                t.name, t.from_frame, t.to_frame, t.direction, t.repeat
            );
        }
    }
    for ts in &file.tilesets {
        println!(
            "  tileset {}: \"{}\" {} tiles of {}x{}",
            ts.id, ts.name, ts.num_tiles, ts.tile_width, ts.tile_height
        );
    }
    for (i, f) in file.frames.iter().enumerate() {
        println!("  frame {i:>3}:  {} cels, {} ms", f.cels.len(), f.duration_ms);
    }
    ExitCode::SUCCESS
}

fn render(path: &str, frame: &str, out: &str) -> ExitCode {
    let file = match parse_file(path) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let frame: usize = match frame.parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("error: bad frame number {frame:?}");
            return ExitCode::from(2);
        }
    };
    let img = match render_frame(&file, frame) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let w = match std::fs::File::create(out) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: cannot create {out}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut enc = png::Encoder::new(w, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().unwrap();
    writer.write_image_data(&img.pixels).unwrap();
    println!("wrote {out} ({}x{})", img.width, img.height);
    ExitCode::SUCCESS
}
