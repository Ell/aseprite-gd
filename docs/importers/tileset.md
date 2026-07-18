# TileSet (Aseprite)

Imports the file's embedded Aseprite tilesets as a Godot `TileSet` with one
`TileSetAtlasSource` per tileset, ready for a `TileMapLayer`. For workflows
that add collision or terrain in Godot, use the sync API instead of the
imported resource — see below.

## Quick start

1. Select a `.aseprite` file that contains at least one tilemap layer (and
   therefore an embedded tileset) in the FileSystem dock.
2. In the Import dock, change the importer to *TileSet (Aseprite)* and click
   Reimport.
3. Assign the file to a `TileMapLayer`'s `tile_set` property and paint.

## Options

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Comma-separated, case-sensitive substrings; layers whose names contain any of them are hidden, including layers revealed by `include_hidden_layers`. Empty disables the filter. |
| `include_hidden_layers` | `false` | Also render layers that are hidden in Aseprite. |
| `exclude_tags` | `""` | Comma-separated, case-sensitive substrings; tags whose names contain any of them produce no animations. |
| `post_import_script` | `""` | Path to a hook script whose `_post_import` runs on the built resource before it is saved — see [post-import-hooks.md](../post-import-hooks.md). |
| `extract_dir` | `""` | When set, tiles whose user data carries a name are written to this folder as `<name>.tres` AtlasTextures sharing one `sheet.res` — a drag-and-drop palette that refreshes on reimport. The folder is owned by the import: stale files from renamed tiles are removed. |

## What maps to what

- Each embedded Aseprite tileset becomes a `TileSetAtlasSource` whose source
  id equals the Aseprite tileset id.
- Tiles are laid out in a fixed 16-column grid: tile `i` sits at atlas
  coords `(i % 16, i / 16)`. The layout never changes as the artist adds
  tiles, so tile references stay stable across reimports.
- The empty tile (index 0, when the tileset reserves it) is skipped.
- `TileSet.tile_size` is taken from the file's first embedded tileset.
- Per-tile user data text lands in an `aseprite_text` custom data layer
  (string type), created only when at least one tile has text.

```gdscript
var ts: TileSet = load("res://art/terrain.aseprite")
var src: TileSetAtlasSource = ts.get_source(ts.get_source_id(0))
var data := src.get_tile_data(Vector2i(0, 0), 0)
print(data.get_custom_data("aseprite_text"))   # e.g. "solid"
```

To set that text in Aseprite: enter tilemap mode, select the tile in the
tileset, and fill in the text field in its tile properties (user data).

## Collision and terrain: use the sync workflow

The imported `TileSet` is regenerated from scratch on every reimport.
Anything you author on it in Godot's TileSet editor — collision polygons,
physics layers, terrain sets, navigation — is lost the next time the art
changes. For those workflows, keep a `TileSet` resource you own and sync the
Aseprite-derived parts into it:

```gdscript
@tool # run from an EditorScript or a plugin
var tile_set: TileSet = load("res://world/tiles.tres")
AseTilesetSync.sync(tile_set, "res://art/terrain.aseprite")
ResourceSaver.save(tile_set, "res://world/tiles.tres")
```

`sync` matches atlas sources by Aseprite tileset id, refreshes textures,
adds new tiles, drops removed ones, and preserves every property you set on
surviving tiles (including collision, terrain, and navigation). See
[tileset-workflow.md](../tileset-workflow.md) for the full guarantees and
how to automate re-syncing.

## Mixed tile sizes

A file can contain tilesets with different tile dimensions. Each atlas
source keeps its own texture region size; `TileSet.tile_size` (the grid
cell) comes from the first tileset on fresh import, and smaller tiles anchor
within that cell. The sync workflow never touches `tile_size` on a TileSet
you own.

## Notes

- Hidden layers are excluded by default; `exclude_layers` wins over
  `include_hidden_layers`. Note that the layer options affect frame
  compositing, not which tilesets exist — tilesets are imported from the
  file's tileset data, not from rendered layers.
- External tileset files (Aseprite's linked-tileset feature) are not
  supported; only tilesets embedded in the file import. A file with no
  embedded tilesets fails to import with `no embedded tilesets in file`.
- The `aseprite_text` custom data layer is a plain string; parse it yourself
  if you encode structured data.

Each atlas source is named "<tileset name> (<id>)" in the TileSet panel
("Tileset (<id>)" when unnamed in Aseprite); the name refreshes from the
file on every import or sync.

Names are authored in Aseprite with the bundled tile-naming dialog
(`aseprite_scripts/name_tiles.lua` in the addon; see
[tileset-workflow.md](../tileset-workflow.md)).
