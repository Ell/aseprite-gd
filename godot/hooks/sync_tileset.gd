@tool
extends RefCounted
# Dual-output recipe: while this file imports as SpriteFrames (or anything
# else), also refresh a TileSet resource from its tilesets on every reimport.
# The sheet is saved as its own file and referenced, not embedded, so anything
# else pointed at it (extracted AtlasTextures, scenes) shares one texture.

const TILESET_PATH := "res://dual_tiles.tres"
const SHEET_PATH := "res://dual_tiles.sheet.res"


func _post_import(resource: Resource, _doc: AseDocument, _options: Dictionary, source_file: String) -> Resource:
    var ts: TileSet
    if ResourceLoader.exists(TILESET_PATH):
        ts = load(TILESET_PATH)
    else:
        ts = TileSet.new()
    if AseTilesetSync.sync_with_sheet(ts, source_file, SHEET_PATH) > 0:
        ResourceSaver.save(ts, TILESET_PATH)
    return resource
