//! Fuzz target: parse then composite every frame. Must never panic; render
//! errors are fine. Canvas area is capped so iterations stay fast — huge
//! canvases just exercise the allocator, not the compositor logic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(file) = ase_core::AseFile::parse(data) else {
        return;
    };
    let area = u32::from(file.header.width) * u32::from(file.header.height);
    if area > 65536 {
        return;
    }
    for i in 0..file.frames.len() {
        let _ = ase_core::composite::render_frame(&file, i);
    }
});
