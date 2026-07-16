# aseprite-gd

Native Aseprite importer for Godot 4 — a pure-Rust GDExtension that parses
`.aseprite`/`.ase` files directly. No Aseprite installation, no CLI wrapper,
no per-machine configuration; imports work identically in the editor, on
teammates' machines, and in headless CI.

**Status: working, pre-release.** See
[docs/feature-list.md](docs/feature-list.md) for full scope.

## What works today

- Drop a `.aseprite` file in your project and import it as a `Texture2D`,
  `SpriteFrames`, `AnimationLibrary`, `TileSet`, `StyleBoxTexture`, or
  `CanvasTexture` — pixel-identical to what Aseprite renders (all 19 blend
  modes, layer/cel opacity, groups, z-index, tilemaps; verified against
  Aseprite's own output on every fixture)
- Tags become animations with exact per-frame durations, loop modes, and
  ping-pong unrolling; frames share a trimmed, deduped, multi-page atlas
- Aseprite tilesets import as configured `TileSet` atlas sources; per-tile
  user data lands in a custom data layer
- 9-patch slices become `StyleBoxTexture`s; slice rects/pivots/user data are
  queryable at runtime
- Layers named `normal`/`specular`/`emission` become `CanvasTexture` maps for
  lit pixel art
- Cel user data text becomes AnimationLibrary method tracks (frame-accurate
  gameplay events)
- Runtime loading: plain `load()` works on `.aseprite` files in running
  games, plus an `AseDocument` class for parsing, rendering, and slice/tag
  queries from GDScript
- Headless CI imports work with zero external dependencies

## Documentation

- [Getting started](docs/getting-started.md) — install, first import, importer overview
- [Importer walkthroughs](docs/importers/) — one guide per import product
- [Cookbook](docs/cookbook.md) — short task-oriented recipes
- [TileSet workflow](docs/tileset-workflow.md) — collision/terrain that survives reimports
- [Runtime API](docs/runtime-api.md) — `AseDocument`, runtime `load()`, TileSet sync
- [Troubleshooting](docs/troubleshooting.md)

Working example scenes live in [godot/examples/](godot/examples/).

## Repository layout

- `crates/ase-core` — parser + compositor, no Godot dependencies
- `crates/ase-cli` — dev inspector (`ase info file.aseprite`)
- `crates/aseprite-gd` — the GDExtension
- `godot/` — demo/test project
- `docs/` — format reference, architecture, research

## Development

```sh
cargo test                    # core tests
cargo build -p aseprite-gd    # build the extension
```

See [AGENTS.md](AGENTS.md) for contributor/agent guidelines and
[docs/architecture.md](docs/architecture.md) for design.
