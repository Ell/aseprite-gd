# aseprite-gd — Architecture (pure Rust)

Decision: **pure-Rust GDExtension** via [godot-rust/gdext](https://github.com/godot-rust/gdext), with a Godot-free core crate. No C++, no Aseprite CLI at import time.

## Ecosystem facts this design rests on (verified 2026-07-15)

**gdext (`godot` crate):**
- Latest release 0.5.4 (2026-06-23), MPL-2.0. Supports Godot 4.2–4.7 via `api-4-x` feature levels; compile against the minimum you support and it runs on anything newer.
- `IEditorImportPlugin` is fully implementable (all virtuals: `get_import_options`, `import`, `get_recognized_extensions`, …). Importer classes must be `#[class(tool)]`. Engine quirk: implement `get_priority`/`get_import_order` explicitly ([godot#104519](https://github.com/godotengine/godot/issues/104519)).
- `IResourceFormatLoader`/`IResourceFormatSaver` implementable; **the editor calls loaders from background threads → the `experimental-threads` cargo feature is mandatory** ([gdext#597](https://github.com/godot-rust/gdext/issues/597), also nested resources [#610](https://github.com/godot-rust/gdext/issues/610)). Enable from day one; keep loader state `Send`-safe.
- `EditorPlugin` subclasses auto-register (docks, inspector plugins, `add_import_plugin`, export plugins all exposed). Keep the whole plugin in Rust — GDScript can't subclass a GDExtension EditorPlugin ([godot#85268](https://github.com/godotengine/godot/issues/85268)).
- Custom `Resource` subclasses are first-class (`#[class(tool, init, base=Resource)]`, `#[export]` fields, `.tres`/`.res` round-trip).
- Panics are caught at the FFI boundary and surface as Godot errors, not editor crashes — good fit for editor tooling.
- Hot reload (`reloadable = true`): works on Linux/Windows, roughest on macOS; treat editor restart as the fallback when class layouts change. Tool classes get re-`ready()`-ed on reload — guard side effects.
- 0.5.3 dropped bindgen/LLVM (JSON codegen), so cross-compiling is plain Rust practice: `cargo-xwin`/mingw for Windows from Linux; macOS builds want a mac CI runner (`lipo` universal binary + `.framework`).

**Existing Rust parsers — why we still write our own core:**
- [`asefile`](https://github.com/alpine-alpaca/asefile) 0.3.8 (MIT): best compositing fidelity (all 19 blend modes, bug-compatible with Aseprite), but **dormant since mid-2024**, and missing exactly what we need: no 1.3 user-data properties maps, no cel z-index, no per-frame palettes, no color profiles, and it hard-errors on chunk orderings newer Aseprite emits (issues #26, #29).
- [`aseprite-loader`](https://github.com/bikeshedder/aseprite-loader) 0.4.2 (2026-02): actively maintained zero-copy parser, but parsing only — no compositor.
- [`aseprite-io`](https://github.com/spebern/aseprite-io) 0.2.0 (2026-06): the only read+write crate, byte-perfect round-trip; very young.
- Verdict: none covers the full spec we documented in [ase-format-reference.md](ase-format-reference.md). We build `ase-core` ourselves and use asefile/aseprite-loader as **cross-check oracles in tests**, not dependencies.

## Workspace layout

```
aseprite-gd/
├── Cargo.toml                 # workspace
├── crates/
│   ├── ase-core/              # pure parser + compositor, zero Godot deps
│   │   ├── src/
│   │   │   ├── read.rs        # bounded little-endian reader (all access bounds-checked)
│   │   │   ├── parse/         # header, frame, one module per chunk type
│   │   │   ├── model/         # Sprite, LayerTree, Cel, Tag, Slice, Tileset, Palette, UserData
│   │   │   ├── composite/     # blend.rs (19 modes, MUL_UN8/DIV_UN8), render.rs (frame flatten,
│   │   │   │                  #   z-index ordering, group buffers, indexed/gray→RGBA)
│   │   │   └── limits.rs      # decompression caps, recursion caps, allocation budgets
│   │   └── fuzz/              # cargo-fuzz targets: parse, composite
│   ├── ase-cli/               # tiny dev binary: dump chunks, render frames to PNG
│   │                          #   (drives the golden-image suite; useful for bug reports)
│   └── aseprite-gd/           # gdext cdylib (crate-type = ["cdylib"])
│       └── src/
│           ├── lib.rs         # ExtensionLibrary entry
│           ├── import/        # EditorImportPlugin impls (one per product):
│           │                  #   texture, sprite_frames, animation_library, tileset, scene
│           ├── runtime/       # ResourceFormatLoader (+ exposed AseFile class for scripts)
│           ├── resources/     # custom Resource subclasses (e.g. AseImportMetadata)
│           ├── convert/       # ase-core model → Godot types (Image, SpriteFrames, TileSet…),
│           │                  #   atlas packing, trim/offset, 16384px splitting
│           └── plugin.rs      # EditorPlugin: registers importers, docks, inspector bits
└── godot/                     # test/demo Godot project + addons/aseprite_gd/*.gdextension
```

Dependency rules: `ase-core` depends on ~nothing (`miniz_oxide` or `flate2` for zlib; no `image`, no `godot`). `aseprite-gd` depends on `ase-core` + `godot`. Anything usable outside Godot lives in `ase-core`; the gdext crate only adapts.

`ase-core` is publishable to crates.io on its own — non-Godot adopters become free correctness QA.

## ase-core design notes

- **Two-phase API:** `AseFile::parse(&[u8]) -> Result<AseFile>` builds the full document model (cheap: chunk index + metadata; cel pixel data stays as zlib slices), then `file.frame(i).render()` / `file.render_layer(...)` inflate + composite on demand. Big files don't pay for frames nobody imports.
- **Compositor** transliterates Aseprite's `blend_funcs.cpp` integer math exactly (the `_n` new-blend variants), per §9 of the format reference. Golden-image tests diff our output against Aseprite's own renders per blend mode × color mode × opacity.
- **Hostile input:** every read bounds-checked (no `unsafe` in parse paths), hard caps: decompressed cel ≤ `w*h*bpp` exactly, total decompression budget per file, property-map recursion ≤ 128, palette size caps. `cargo-fuzz` targets run in CI.
- **Errors:** structured (`chunk offset + kind + context`), so importer diagnostics can say *what* is corrupt where. Unknown chunks/fields skipped per spec (forward compatibility).
- **Determinism:** identical input bytes → identical output bytes (stable atlas packing order), so Godot's `.godot/imported` artifacts are reproducible across machines/CI.

## gdext layer design notes

- One `EditorImportPlugin` per import product (Texture2D, SpriteFrames, AnimationLibrary, TileSet, PackedScene), all thin shells over `convert/`. Shared option schema (layer filters, packing, trim, etc.) defined once.
- `can_import_threaded() -> true` is the goal (parsing is `Send`); requires the same `experimental-threads` discipline as the runtime loader.
- Runtime loading: `ResourceFormatLoader` registered for `.aseprite`/`.ase` (opt-in project setting, since editor imports normally shadow it), plus an `AseFile` RefCounted class exposing frames/layers/tags/slices to GDScript/C# for modding use cases.
- TileSet reimport-safe merging: generated `TileSet` carries provenance metadata (which atlas source/tiles we own); on reimport we rebuild only owned parts and preserve user-added physics/terrain/custom-data. This is its own design doc eventually.

## Build, CI, distribution

- **CI matrix:** Linux x86_64 (host), Windows x86_64 via `cargo-xwin`, macOS universal (aarch64+x86_64, `lipo`) on a mac runner. Artifacts assembled into `addons/aseprite_gd/` with the `.gdextension` (`compatibility_minimum = 4.2` — pending a check of which api level the needed editor APIs require; `reloadable = true`).
- **Test tiers in CI:** (1) `ase-core` unit + golden-image tests — pure cargo, fast; (2) fuzz smoke (short budget per PR, long nightly); (3) integration: `godot --headless --import` on the demo project, assert generated resources — this is the product promise, so it's tested directly; (4) cross-check oracle tests comparing parses against `asefile`/`aseprite-loader` on the corpus.
- **Golden corpus:** generated once by a dev-only script that drives real Aseprite (CLI is fine *here* — it never ships); fixtures committed. Covers: every blend mode, indexed/gray/RGBA, linked cels, z-index, groups w/ blend, tilemaps + flips, slices w/ 9-patch+pivot, per-frame palettes, 1.3 properties, old pre-1.2 files.

## Known risks & mitigations

| Risk | Mitigation |
|---|---|
| gdext lags a future Godot minor | api-level pinning (forward-compatible); we depend on stable editor APIs, not bleeding-edge ones |
| `experimental-threads` soundness holes | keep shared state minimal/immutable after init; loader is stateless over `ase-core` |
| Hot-reload flakiness (esp. macOS) | document "restart editor" fallback; avoid state that must survive reload |
| Blend-math drift vs future Aseprite | golden corpus regenerated per Aseprite release; upstream `blend_funcs.cpp` watched |
| TileSet merge complexity | ship TileSet import in phases: atlas-source only → custom data → merge-preserving reimport |
