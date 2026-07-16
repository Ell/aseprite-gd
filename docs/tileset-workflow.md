# TileSet workflow

Two ways to consume Aseprite tilesets, depending on whether you need
Godot-side tile data (collision, terrain, navigation).

## Zero-config: import as TileSet

Switch the file's importer to *TileSet (Aseprite)* and use the imported
resource directly. Good for purely visual tilemaps. The resource is
regenerated on every reimport, so anything you edit on it in the TileSet
editor is lost — Godot treats imported resources as derived artifacts, and
so does this plugin.

## With Godot-side authoring: sync into your own TileSet

Collision polygons, physics layers, terrain sets, and navigation are authored
in Godot's TileSet editor and belong in a TileSet resource *you* own. The
plugin updates the Aseprite-derived parts of that resource in place and
leaves everything else alone:

```gdscript
@tool # run from an EditorScript or a plugin
var tile_set: TileSet = load("res://world/tiles.tres")
AseTilesetSync.sync(tile_set, "res://art/terrain.aseprite")
ResourceSaver.save(tile_set, "res://world/tiles.tres")
```

`sync` guarantees:

- Atlas sources are matched by Aseprite tileset id. Missing sources are
  created; existing ones get their texture and region size refreshed.
- Tiles that still exist in the Aseprite file keep every property you set on
  them (physics, terrain bits, probability, navigation, custom data).
- Tiles added in Aseprite appear; tiles removed from Aseprite are dropped
  along with their data.
- Nothing outside the synced sources is touched: physics/terrain/navigation
  layer definitions, other sources, and scene-collection sources all survive.
- Per-tile user data from Aseprite refreshes the `aseprite_text` custom data
  layer (created on first sync when needed).

Re-run the sync whenever the art changes; wire it into a save hook or an
EditorScript shortcut if you want it automatic.
