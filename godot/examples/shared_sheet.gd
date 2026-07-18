extends Node2D
# The TileMapLayer's TileSet and the Sprite2D's AtlasTexture both reference
# res://extracted_tiles/sheet.res — one texture on disk and on the GPU, so
# the tilemap and the sprite batch together (see docs/tileset-workflow.md).


func _ready() -> void:
    var map: TileMapLayer = $TileMapLayer
    var source_id: int = map.tile_set.get_source_id(0)
    for x in range(8):
        map.set_cell(Vector2i(x, 2), source_id, Vector2i(0, 0))
