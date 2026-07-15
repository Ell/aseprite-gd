# aseprite-gd

Native Aseprite importer for Godot 4 — a pure-Rust GDExtension that parses
`.aseprite`/`.ase` files directly. No Aseprite installation, no CLI wrapper,
no per-machine configuration; imports work identically in the editor, on
teammates' machines, and in headless CI.

**Status: early development.** Parser core is being built first; see
[docs/feature-list.md](docs/feature-list.md) for the roadmap.

## Planned highlights

- Drop a `.aseprite` file in your project → Texture2D, SpriteFrames,
  AnimationLibrary, or TileSet, pixel-identical to what Aseprite renders
  (all 19 blend modes, layer/cel opacity, groups, z-index)
- Real `TileSet` import from Aseprite tilesets, with reimport-safe merging of
  collision/terrain you author in Godot
- Slices → AtlasTextures, 9-patch StyleBoxes, pivots, animated hitboxes
- Layer-convention normal maps → CanvasTexture
- Cel user data → Call Method tracks / animation markers
- Runtime loading API for mods and user content

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
