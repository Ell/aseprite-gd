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

func _post_import(resource: Resource, _doc: AseDocument,
        _options: Dictionary, source_file: String) -> Resource:
    var ts: TileSet = load(TILESET_PATH) if ResourceLoader.exists(TILESET_PATH) else TileSet.new()
    if AseTilesetSync.sync(ts, source_file) > 0:
        ResourceSaver.save(ts, TILESET_PATH)
    return resource
```

Collision and terrain authored on that TileSet survive, per the usual sync
guarantees.

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
