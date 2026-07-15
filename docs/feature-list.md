# Feature list

Scope for aseprite-gd: a Godot 4 GDExtension that reads `.aseprite`/`.ase`
files directly. No Aseprite installation is needed at import time, imports
behave the same on every machine and in headless CI, and output matches what
Aseprite renders.

## Why a native parser

- Importing through Aseprite's CLI ties every machine (and every CI container)
  to an installed, licensed, correctly-pathed Aseprite binary. Parsing the
  format directly removes that dependency.
- The format is fully specified and stable; a parser plus compositor can
  reproduce Aseprite's output exactly rather than approximating it through
  export settings.
- `godot --headless --import` works in a bare container with no setup.

## Why Rust

- The parser's input is untrusted (downloaded files, mod content, runtime
  loading). Rust removes the memory-safety bug class outright, and cargo-fuzz
  makes fuzzing routine.
- The code splits into a pure `ase-core` crate (parser + compositor, no Godot
  types — independently testable, fuzzable, usable outside Godot) and a thin
  gdext layer that assembles Godot resources.
- Compositing math is transliterated from Aseprite's own source
  (`MUL_UN8`/`DIV_UN8` integer blending, the newer blend variants) and held to
  it by a golden-image suite that diffs our renders against Aseprite's.
- Known tradeoff: gdext is community-maintained and can lag new Godot minors
  or unusual editor APIs, so the `EditorImportPlugin`/`ResourceFormatLoader`
  surface was verified before anything was built on it.

## Tier 0 — Parsing & compositing core

- [ ] Full spec coverage: header, frames, every current chunk type (layer, cel, cel-extra, color profile, external files, tags, palette, user data, slice, tileset), graceful skip of deprecated/unknown chunks (forward compatible)
- [ ] All three color modes: RGBA, Grayscale, Indexed (transparent index semantics, per-frame palette deltas, >256-entry palettes, background-layer opacity rules)
- [ ] All cel types: raw, linked (chain-resolved, honoring link-local x/y/opacity), zlib-compressed image, compressed tilemap (mask-driven bit decoding, x/y/d flips)
- [ ] Pixel-exact compositing: all 19 blend modes with Aseprite's exact integer math, layer × cel opacity, new-blend variants for parity with modern Aseprite, group compositing with group blend/opacity, cel z-index ordering, visibility inheritance
- [ ] Layer tree fidelity: groups/child levels, background/reference-layer flags, layer UUIDs
- [ ] Tags with loop directions (forward/reverse/ping-pong/ping-pong-reverse) and repeat counts; tag colors from user data with in-chunk fallback for older files
- [ ] User data everywhere it attaches (sprite, layers, cels, tags, slices, tilesets, per-tile), including typed properties maps
- [ ] Old-file compatibility: pre-1.2 .ase (header speed fallback, old palette chunks, raw cels)
- [ ] Hostile-file safety: hard caps on decompressed sizes, bounds-checked reads everywhere, fuzz-tested
- [ ] Golden-image test suite: composited output compared against Aseprite's own renders per blend mode/opacity/color mode

## Tier 1 — Core import products

- [ ] `Texture2D` import (composited frame, or chosen frame/tag) so `.aseprite` files work as plain images out of the box
- [ ] `SpriteFrames` import for AnimatedSprite2D/3D: tags → animations, per-frame durations (relative frame duration, not lossy FPS conversion), loop from repeat count or name convention, direction handling including ping-pong unrolling
- [ ] `AnimationLibrary` / AnimationPlayer import: region-based tracks on a packed atlas; merges into existing players without touching hand-made tracks
- [ ] Optional `PackedScene` generation (AnimatedSprite2D/3D, Sprite2D+AnimationPlayer, TextureRect)
- [ ] Layer control: include/exclude by pattern, visible-only toggle, name-suffix conventions, group-aware selection, split-by-layer to separate resources/tracks (multi-layer characters: gear/clothing overlays)
- [ ] Atlas packing: tight packing with trim + offset preservation, padding/extrusion options, deduplication of identical/linked frames, automatic sheet splitting under the 16384px texture limit
- [ ] Import dock integration: per-file options + presets via EditorImportPlugin, automatic reimport on save, clean filesystem-scan behavior
- [ ] Headless/CI-clean: `godot --headless --import` produces identical output with zero external dependencies — deterministic, byte-stable imports

## Tier 2 — Deep engine integration

- [ ] TileSet import: Aseprite tilesets → `TileSet` with a configured `TileSetAtlasSource`; flip/transpose tile flags → alternative tiles; per-tile user data → custom data layers; empty-tile handling. Authoring a tileset in Aseprite is painting a tilemap layer, so every tileset file has tilemap cels — the deliverable is the `TileSet` resource. The hard part is reimport-safe merging: collision polygons, physics layers, and terrain bits are authored in Godot's TileSet editor (Aseprite can't express them) and must survive when the artist adds tiles and saves.
- [ ] Slices: slices → `AtlasTexture`s; 9-patch slices → `StyleBoxTexture`/`NinePatchRect`; pivots → sprite offset/centering; named slices → collision shapes via convention or user data; per-frame slice keys → animated hitboxes/hurtboxes on AnimationPlayer tracks
- [ ] Normal map / emission pipeline: layer-name or group convention (`normal`, `emission`, `specular`) → `CanvasTexture` with matching atlas layouts, for lit pixel art with no manual steps
- [ ] User data → gameplay: cel/frame user data → Call Method tracks or animation markers (frame-accurate footsteps/impacts/spawns); tag user data → animation metadata; sprite/layer user data → node metadata; typed properties surfaced as `Dictionary`
- [ ] Runtime loading: `ResourceFormatLoader` plus an exposed `AseFile` class for loading `.aseprite` at runtime (mods, user content) with per-layer/per-frame image access from script
- [ ] Palette import: Aseprite palette → `ColorPalette` resource / `Gradient` / color constants

## Tier 3 — Workflow polish

- [ ] Editor preview: animation preview for `.aseprite` files in the editor (scrub tags, watch loops)
- [ ] Post-import script hook (user callback receiving generated resources)
- [ ] Compression control: VRAM compression opt-in per file (with normal-map-friendly modes), lossless default for pixel art, `PortableCompressedTexture2D` option
- [ ] Nearest-neighbor filter defaults appropriate for pixel art
- [ ] Diagnostics: precise, actionable import errors (offset + chunk context on corrupt files), warnings for unsupported constructs instead of silent drops
- [ ] Cookbook docs: multi-layer characters, tilemap levels, 9-patch UI, lit sprites, hitboxes
- [ ] Reference layers, color profiles (sRGB assumption documented), pixel-aspect handling

## Tier 4 — Later

- [ ] Incremental reimport: hash frames/layers and only regenerate changed sub-resources on save
- [ ] Multi-file atlas sharing: pack many `.aseprite` files into shared atlases project-wide
- [ ] Write support: save composited exports (PNG strips, JSON metadata) for interop
- [ ] Terrain/autotile inference from tile user data or slice conventions
- [ ] TileMapLayer node generation from tilemap cels: the tile grid is decoded anyway (required to composite frames), so optionally emit it as a `TileMapLayer` scene for room prototyping
- [ ] Idiomatic C# access to the exposed classes

## Non-goals (v1)

- Wrapping the Aseprite CLI in any form
- Godot 3.x support
- Importing formats other than `.aseprite`/`.ase`
