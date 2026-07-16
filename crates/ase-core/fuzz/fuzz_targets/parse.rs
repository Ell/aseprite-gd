//! Fuzz target: `AseFile::parse` on arbitrary bytes must never panic —
//! structured `ParseError`s are the only acceptable failure mode.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = ase_core::AseFile::parse(data);
});
