@tool
extends RefCounted
# Dual-output recipe: while this file imports as SpriteFrames (or anything
# else), also refresh a TileSet resource from its tilesets on every reimport.

const TILESET_PATH := "res://dual_tiles.tres"


func _post_import(resource: Resource, _doc: AseDocument, _options: Dictionary, source_file: String) -> Resource:
    var ts: TileSet
    if ResourceLoader.exists(TILESET_PATH):
        ts = load(TILESET_PATH)
    else:
        ts = TileSet.new()
    if AseTilesetSync.sync(ts, source_file) > 0:
        ResourceSaver.save(ts, TILESET_PATH)
    return resource
