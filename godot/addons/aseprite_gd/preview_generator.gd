@tool
extends EditorResourcePreviewGenerator
# FileSystem-dock thumbnails for .aseprite/.ase files: the composited first
# frame, whatever importer the file uses. The engine only previews files
# whose imported product is a texture; this covers SpriteFrames, TileSet,
# AnimationLibrary, and the rest. Runs on the editor's preview thread —
# AseDocument does no scene access, so that's safe.


func _handles(_type: String) -> bool:
    # Dispatch is by imported type, which says nothing about the source
    # file; claim everything and decide by path. Returning null from
    # _generate_from_path lets other generators (and the generic icon)
    # take over, and the engine's own generators run first regardless.
    return true


func _generate_from_path(path: String, size: Vector2i, _metadata: Dictionary = {}) -> Texture2D:
    var ext := path.get_extension().to_lower()
    if ext != "aseprite" and ext != "ase":
        return null
    var doc: AseDocument = AseDocument.open(path)
    if doc == null:
        return null
    var img: Image = doc.render_frame(0)
    if img == null:
        return null
    if img.get_width() > size.x or img.get_height() > size.y:
        var aspect := float(img.get_width()) / float(img.get_height())
        var w := size.x
        var h := int(size.x / aspect)
        if h > size.y:
            h = size.y
            w = int(size.y * aspect)
        img.resize(maxi(w, 1), maxi(h, 1), Image.INTERPOLATE_NEAREST)
    return ImageTexture.create_from_image(img)


func _generate(_resource: Resource, _size: Vector2i, _metadata: Dictionary = {}) -> Texture2D:
    # Only path-based generation makes sense here; in-memory resources are
    # already their imported product and covered by the engine.
    return null


func _generate_small_preview_automatically() -> bool:
    return true
