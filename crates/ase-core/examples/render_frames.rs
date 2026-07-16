//! Renders every frame of an .aseprite file and prints basic info.
//!
//!     cargo run -p ase-core --example render_frames -- file.aseprite

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: render_frames <file.aseprite>");
    let data = std::fs::read(&path).expect("read file");
    let file = ase_core::AseFile::parse(&data).expect("parse");

    println!(
        "{}x{}, {} frames, {} layers, {} tags",
        file.header.width,
        file.header.height,
        file.frames.len(),
        file.layers.len(),
        file.tags.len()
    );
    for i in 0..file.frames.len() {
        let img = ase_core::composite::render_frame(&file, i).expect("render");
        let opaque = img.pixels.chunks_exact(4).filter(|px| px[3] != 0).count();
        println!(
            "frame {i}: {} ms, {opaque} visible pixels",
            file.frames[i].duration_ms
        );
    }
}
