@tool
extends RefCounted
# Post-import hook: builds an Area2D with one CollisionShape2D per slice,
# positioned from the slice metadata in the Aseprite file.


func _post_import(root: Node, doc: AseDocument, _options: Dictionary, _source_file: String) -> Node:
    var area := Area2D.new()
    area.name = "Hitboxes"
    root.add_child(area)
    for sl in doc.get_slices(0):
        var shape := CollisionShape2D.new()
        shape.name = sl["name"]
        var rect := RectangleShape2D.new()
        rect.size = Vector2(sl["rect"].size)
        shape.shape = rect
        shape.position = Vector2(sl["rect"].position) + Vector2(sl["rect"].size) / 2.0
        if sl.has("text"):
            shape.set_meta("ase_text", sl["text"])
        area.add_child(shape)
    root.set_meta("sprite_tags", doc.get_tag_names())
    return root
