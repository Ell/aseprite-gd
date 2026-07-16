# Post-import hooks

Every importer accepts a `post_import_script` option: a GDScript whose
`_post_import` method runs after the importer builds its product and before
the product is saved. The hook receives the parsed Aseprite document, so it
can read anything in the file — user data, slices, tags, layer names — and
reshape the output accordingly.

The script must be marked `@tool` (it runs inside the editor) and define:

```gdscript
@tool
extends RefCounted

func _post_import(product, doc: AseDocument, options: Dictionary,
        source_file: String):
    # mutate `product`, or return a replacement; returning null keeps it
    return product
```

- `product` — what the importer built. A `Resource` subtype for the resource
  importers (ImageTexture, SpriteFrames, AnimationLibrary, TileSet,
  StyleBoxTexture, CanvasTexture); the root `Node` for the PackedScene
  importer.
- `doc` — the parsed file as an [AseDocument](runtime-api.md), after layer
  filtering options were applied.
- `options` — the import options dictionary as configured in the Import dock.
- `source_file` — the `res://` path of the `.aseprite` file.

A configured hook that cannot run (missing file, no `_post_import` method,
instantiation failure) fails the import with an error rather than being
skipped, and a return value of the wrong type fails it too.

## Scene hooks

The *PackedScene (Aseprite)* importer builds a live node tree — a Node2D root
named after the file with an AnimatedSprite2D child playing the first
animation — and runs the hook on it before ownership finalization and
packing. Nodes the hook adds are adopted into the scene automatically;
returning a different node replaces the root entirely.

This is where file metadata turns into gameplay structure. The demo project's
`hooks/add_slice_hitboxes.gd` builds collision shapes from slices:

```gdscript
@tool
extends RefCounted

func _post_import(root: Node, doc: AseDocument, _options: Dictionary,
        _source_file: String) -> Node:
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
        area.add_child(shape)
    return root
```

Draw a slice named `hurtbox` in Aseprite, reimport, and the scene carries a
ready collision shape.

## Resource hooks

For the other importers the hook receives the resource. Typical uses: stamp
metadata read from the file, adjust animation loop flags, post-process a
TileSet. The demo project's `hooks/tag_metadata.gd`:

```gdscript
@tool
extends RefCounted

func _post_import(resource: Resource, doc: AseDocument,
        _options: Dictionary, _source_file: String) -> Resource:
    resource.set_meta("ase_tags", doc.get_tag_names())
    return resource
```

## Notes

- Hooks run on every reimport of that file, including the automatic reimport
  when the source changes on disk.
- Keep hooks deterministic: same file in, same product out, or version
  control will churn on the imported artifacts.
- Errors printed by a hook (or by the importer about the hook) appear in the
  editor's Output panel and in `godot --headless --import` logs.
