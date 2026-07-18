# Cookbook

Practical recipes. Every `.aseprite`/`.ase` file in the project defaults to
the *Texture2D (Aseprite)* importer; switch importers per file in the Import
dock.

## Animated character (AnimatedSprite2D)

Switch the file to *SpriteFrames (Aseprite)* and assign the imported resource
to an AnimatedSprite2D. Tags become animations with exact per-frame timings;
tag loop settings and ping-pong directions carry over. All frames share one
trimmed, deduplicated atlas, and every frame still renders at canvas size, so
nothing shifts.

## AnimationPlayer-driven sprite

Switch to *AnimationLibrary (Aseprite)*, add the imported library to an
AnimationPlayer, and set the `sprite_path` import option to the path from the
player's root node to your Sprite2D (default `Sprite2D`). Each tag becomes an
Animation with a texture track.

Frame-accurate gameplay events: put text in a cel's user data in Aseprite
(right-click cel → Properties) — the text becomes a Call Method track keyed at
that frame's start, calling that method on the sprite node. Footsteps,
impacts, spawn points.

Animated hitboxes: enable the `slice_tracks` import option and create child
nodes named after your slices. Each animation then keys
`<slice name>:position` and `<slice name>:size` from the slice's per-frame
keys — draw your hurtboxes in Aseprite, move them per frame, done.

## Sprite sheets (grid of individual sprites)

For a sheet that is one big image (an asset-pack grid rather than Aseprite
frames), set the SpriteFrames importer's `split_grid` option to the cell
size, e.g. `16x16`. Every cell becomes an indexable frame:

```gdscript
var sheet: SpriteFrames = load("res://characters.aseprite")
var hero: Texture2D = sheet.get_frame_texture("default", 9) # row-major index
```

On a multi-frame file, `split_grid` produces one animation per cell playing
that cell across the frames (each canvas tile is its own animation set,
e.g. walk-down / walk-side / walk-up stacked vertically). Add tags and the
combination becomes `<tag>_<cell>` animation sets — directions in the grid,
actions as tags, loop behavior from each tag's repeat — which fits a whole
character in one file.

Cells share one trimmed atlas, so blank and duplicate cells are free. Sheets
whose regions are not a uniform grid should use slices instead (see the
runtime API's `get_slices`).

## Named sprites out of one file (items, icons)

Name regions in the art — slices for irregular shapes, or per-tile user data
for tiles (use the bundled `name_tiles.lua` Aseprite dialog) — and set
`extract_dir` on the texture or TileSet import. The folder fills with
`sword.tres`, `potion.tres`, ... AtlasTextures sharing one sheet: drag them
into any texture slot, and they refresh when the art changes. One GPU
texture behind all of them, so drawing several batches cleanly. If a hook on
the same file also syncs a TileSet, `AseTilesetSync.sync_with_sheet` pointed
at the extraction's `sheet.res` makes the tilemap share that texture too
(see [tileset-workflow.md](tileset-workflow.md)).

## Multi-layer characters (outfits, equipment)

Enable `split_layers` on the SpriteFrames import: each visible layer becomes
its own set of animations named `<layer>/<tag>`, all sharing one atlas. Stack
one AnimatedSprite2D per layer and play the same tag on each. With the
AnimationLibrary importer instead, `split_layers` puts one texture track per
layer into each animation (targeting `<sprite_path>/<layer>:texture`), so a
single AnimationPlayer animation drives every layer in sync.

## Importing into an existing AnimationPlayer

Select the AnimationPlayer and use the Inspector's "Aseprite Import" section
(see [editor-tools.md](editor-tools.md)): animations merge into its library
without disturbing hand-made tracks or animations, and "Reimport last"
repeats the import after the art changes.

## Tilemaps

Quick path: switch the file to *TileSet (Aseprite)* and use the imported
TileSet directly.

With collision/terrain: keep your own TileSet resource and sync into it —
see [tileset-workflow.md](tileset-workflow.md). Per-tile user data text shows
up in the `aseprite_text` custom data layer either way.

## One file as both animation and tileset

Godot allows one importer per file, but the sync API reads files directly,
so a mixed file (character layers plus a tilemap layer) can feed both:

1. Import the file as SpriteFrames, with `exclude_layers` hiding the tilemap
   layer so it stays out of the animation frames.
2. Set a post-import hook that refreshes a TileSet resource on every
   reimport (the demo project ships this as `hooks/sync_tileset.gd`):

```gdscript
@tool
extends RefCounted

const TILESET_PATH := "res://world/tiles.tres"
const SHEET_PATH := "res://world/tiles.sheet.res"

func _post_import(resource: Resource, _doc: AseDocument,
        _options: Dictionary, source_file: String) -> Resource:
    var ts: TileSet = load(TILESET_PATH) if ResourceLoader.exists(TILESET_PATH) else TileSet.new()
    if AseTilesetSync.sync_with_sheet(ts, source_file, SHEET_PATH) > 0:
        ResourceSaver.save(ts, TILESET_PATH)
    return resource
```

Collision and terrain authored on that TileSet survive, per the usual sync
guarantees. `sync_with_sheet` keeps the sheet in its own file instead of
embedding it in the `.tres`; if the import also uses `extract_dir`, pass
`options["extract_dir"].path_join("sheet.res")` as the sheet path and the
TileSet shares one texture with the extracted AtlasTextures.

## 9-patch UI panels

Give the slice a center rect in Aseprite (slice properties → 9-slices),
switch the file to *StyleBoxTexture (Aseprite)*, and use the resource in any
Control theme override. `slice_name` picks a specific slice; the default is
the first 9-patch slice.

## Lit pixel art

Name a layer `normal` (or suffix a layer `_normal`) and paint your normal
map; `specular`/`emission` layers work the same. Switch the file to
*CanvasTexture (Aseprite)* — the color layers become the diffuse map and the
convention-named layers become the corresponding textures, all excluded from
each other.

## Layer filtering

All importers support `exclude_layers` (comma-separated substrings; matching layers are
hidden) and `include_hidden_layers`. Useful for guide/reference layers or
alternate outfits.

## Runtime loading (mods, user content)

In a running game, plain `load("user://mods/enemy.aseprite")` returns the
composited first frame as an ImageTexture. For full access:

```gdscript
var doc := AseDocument.open("user://mods/enemy.aseprite")
var frames := doc.get_frame_count()
var image := doc.render_frame(0)          # exact Aseprite compositing
var tags := doc.get_tag_names()
var slices := doc.get_slices(0)           # rects, pivots, 9-patch, user data
```

Parsing is hardened against malformed files (bounded allocations, fuzz
tested), so pointing it at downloaded content is fine.
