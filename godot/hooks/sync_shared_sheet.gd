@tool
extends RefCounted
# Shared-sheet recipe: this file's import extracts named tiles into
# extract_dir; the hook syncs a user-owned TileSet against the same file and
# points it at the extraction's sheet, so the TileSet and every extracted
# AtlasTexture reference one texture file.

const TILESET_PATH := "res://shared_tiles.tres"


func _post_import(resource: Resource, _doc: AseDocument, options: Dictionary, source_file: String) -> Resource:
    var ts: TileSet
    if ResourceLoader.exists(TILESET_PATH):
        ts = load(TILESET_PATH)
    else:
        ts = TileSet.new()
    var sheet: String = String(options.get("extract_dir", "")).path_join("sheet.res")
    if AseTilesetSync.sync_with_sheet(ts, source_file, sheet) > 0:
        ResourceSaver.save(ts, TILESET_PATH)
    return resource
