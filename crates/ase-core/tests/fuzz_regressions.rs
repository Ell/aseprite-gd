//! Regression tests for bugs found by the fuzz targets in
//! `crates/ase-core/fuzz`. Each input is a `cargo fuzz tmin`-minimized crash
//! artifact inlined as bytes — parsing must return a structured error (or
//! succeed), never panic.

use ase_core::AseFile;

/// Minimal valid 128-byte header (§3): RGBA, 1x1 canvas, `frames` frames.
fn header(frames: u16) -> Vec<u8> {
    let mut h = Vec::new();
    h.extend_from_slice(&0u32.to_le_bytes()); // file size (advisory)
    h.extend_from_slice(&0xA5E0u16.to_le_bytes()); // magic
    h.extend_from_slice(&frames.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // width
    h.extend_from_slice(&1u16.to_le_bytes()); // height
    h.extend_from_slice(&32u16.to_le_bytes()); // color depth
    h.resize(128, 0);
    h
}

/// Frame envelope (§4) wrapping pre-assembled chunk bytes.
fn frame(num_chunks: u16, chunks: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&(16 + chunks.len() as u32).to_le_bytes()); // frame bytes
    f.extend_from_slice(&0xF1FAu16.to_le_bytes()); // magic
    f.extend_from_slice(&num_chunks.to_le_bytes()); // old chunk count
    f.extend_from_slice(&100u16.to_le_bytes()); // duration
    f.extend_from_slice(&[0; 2]); // reserved
    f.extend_from_slice(&0u32.to_le_bytes()); // new chunk count
    f.extend_from_slice(chunks);
    f
}

/// Chunk envelope (§5).
fn chunk(kind: u16, payload: &[u8]) -> Vec<u8> {
    let mut c = Vec::new();
    c.extend_from_slice(&(6 + payload.len() as u32).to_le_bytes());
    c.extend_from_slice(&kind.to_le_bytes());
    c.extend_from_slice(payload);
    c
}

/// User data chunk 0x2020 with just a text field (§6.11).
fn user_data_chunk() -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&1u32.to_le_bytes()); // flags: has text
    p.extend_from_slice(&1u16.to_le_bytes()); // string length
    p.push(b'x');
    chunk(0x2020, &p)
}

/// Tileset chunk 0x2023 fixed fields (§6.13) with an empty name.
fn tileset_payload(flags: u32, num_tiles: u32, tile_w: u16, tile_h: u16) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&1u32.to_le_bytes()); // id
    p.extend_from_slice(&flags.to_le_bytes());
    p.extend_from_slice(&num_tiles.to_le_bytes());
    p.extend_from_slice(&tile_w.to_le_bytes());
    p.extend_from_slice(&tile_h.to_le_bytes());
    p.extend_from_slice(&0u16.to_le_bytes()); // base index
    p.extend_from_slice(&[0; 14]); // reserved
    p.extend_from_slice(&0u16.to_le_bytes()); // name length (empty)
    p
}

/// A tileset (§6.13) with embedded pixels and a huge `num_tiles` overflowed
/// the `tile_w * tile_h * num_tiles * bpp` expected-size product (panic in
/// overflow-checked builds). Fixed by the `MAX_TILESET_TILES` limit plus
/// `checked_mul` in `parse::tileset`.
#[test]
fn tileset_strip_size_overflow_does_not_panic() {
    let mut p = tileset_payload(2, u32::MAX, u16::MAX, u16::MAX);
    p.extend_from_slice(&0u32.to_le_bytes()); // compressed data length

    let mut data = header(1);
    data.extend_from_slice(&frame(1, &chunk(0x2023, &p)));

    assert!(AseFile::parse(&data).is_err());
}

/// An external tileset (§6.13) carries no pixel data, leaving `num_tiles`
/// unchecked; a following user data chunk then allocated
/// `vec![UserData; num_tiles]` — multi-GiB for a ~200 byte file (OOM).
/// Fixed by rejecting `num_tiles > MAX_TILESET_TILES` at parse time.
#[test]
fn huge_external_tileset_tile_count_is_rejected() {
    let mut chunks = chunk(0x2023, &tileset_payload(0, u32::MAX, 1, 1));
    chunks.extend_from_slice(&user_data_chunk()); // tileset user data
    chunks.extend_from_slice(&user_data_chunk()); // would size the tile vec

    let mut data = header(1);
    data.extend_from_slice(&frame(3, &chunks));

    assert!(AseFile::parse(&data).is_err());
}

/// A tags chunk (§6.9) declaring zero tags set the user-data target to an
/// empty `tags[start..start]` run; the next user data chunk then indexed
/// `tags[start]` out of bounds (panic). Fixed in `AseFile::parse` by not
/// creating a target for an empty run.
#[test]
fn user_data_after_empty_tags_chunk_does_not_panic() {
    let mut tags_payload = Vec::new();
    tags_payload.extend_from_slice(&0u16.to_le_bytes()); // zero tags
    tags_payload.extend_from_slice(&[0; 8]); // reserved

    let mut chunks = chunk(0x2018, &tags_payload);
    chunks.extend_from_slice(&user_data_chunk());

    let mut data = header(1);
    data.extend_from_slice(&frame(2, &chunks));

    let file = AseFile::parse(&data).expect("valid file");
    assert!(file.tags.is_empty());
}

/// A cel at the end of frame N left `UdTarget::Cel` pointing into the
/// frame-local cel vec; a user data chunk at the start of frame N+1 then
/// indexed the new frame's empty vec (panic). Fixed in `AseFile::parse` by
/// dropping a stale cel target at the frame boundary.
#[test]
fn user_data_after_frame_boundary_cel_does_not_panic() {
    // Frame 1: a layer and a raw 1x1 RGBA cel on it.
    let mut layer_payload = Vec::new();
    layer_payload.extend_from_slice(&1u16.to_le_bytes()); // flags: visible
    layer_payload.extend_from_slice(&[0; 10]); // type/child/defaults/blend
    layer_payload.push(255); // opacity
    layer_payload.extend_from_slice(&[0; 3]); // reserved
    layer_payload.extend_from_slice(&1u16.to_le_bytes()); // name length
    layer_payload.push(b'L');

    let mut cel_payload = Vec::new();
    cel_payload.extend_from_slice(&0u16.to_le_bytes()); // layer index
    cel_payload.extend_from_slice(&[0; 4]); // x, y
    cel_payload.push(255); // opacity
    cel_payload.extend_from_slice(&0u16.to_le_bytes()); // cel type: raw image
    cel_payload.extend_from_slice(&[0; 7]); // z-index + reserved
    cel_payload.extend_from_slice(&1u16.to_le_bytes()); // width
    cel_payload.extend_from_slice(&1u16.to_le_bytes()); // height
    cel_payload.extend_from_slice(&[1, 2, 3, 4]); // one RGBA pixel

    let mut chunks = chunk(0x2004, &layer_payload);
    chunks.extend_from_slice(&chunk(0x2005, &cel_payload));

    // Frame 2: only a user data chunk — must not attach to frame 1's cel.
    let mut data = header(2);
    data.extend_from_slice(&frame(2, &chunks));
    data.extend_from_slice(&frame(1, &user_data_chunk()));

    let file = AseFile::parse(&data).expect("valid file");
    assert_eq!(file.frames.len(), 2);
    assert!(file.frames[0].cels[0].user_data.text.is_none());
}

/// A compressed cel (§6.3) whose declared chunk size ends before the cel's
/// fixed fields do: `inflate_exact` computed `chunk_end - pos` with the reader
/// already past `chunk_end`, underflowing usize (panic in overflow-checked
/// builds). Fixed with `checked_sub` in `parse::cel::inflate_exact`.
#[test]
fn cel_fields_overrunning_chunk_end_do_not_panic() {
    let data: &[u8] = &[
        252, 8, 0, 0, 224, 165, 6, 0, 64, 0, 64, 0, 8, 0, 1, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 32, 0, 1, 1, 0, 0, 0, 0, 16, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 5, 0, 0, 250, 241, 22, 0, 100, 0, 196, 61, 10, 0, 0,
        0, 37, 0, 0, 0, 4, 32, 3, 0, 2, 0, 0, 0, 0, 0, 0, 240, 110, 16, 0, 0, 1, 199, 0, 0, 7, 0,
        0, 0, 0, 100, 0, 0, 0, 3, 0, 0, 0, 36, 0, 0, 0, 5, 32, 0, 0, 0, 0, 0, 0, 255, 3, 0, 0, 0,
        0, 0, 0, 0, 0, 4, 0, 4, 0, 32, 0, 255, 0, 0, 0, 0, 0, 0, 156, 237, 193, 1, 13, 0, 0, 0,
        194, 160, 247, 79, 109, 15, 7, 20, 0, 0, 0,
    ];
    assert!(AseFile::parse(data).is_err());
}
