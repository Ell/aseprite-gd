# Runtime API

Script-facing classes for working with `.aseprite`/`.ase` files from
GDScript or C#, outside (or alongside) the editor import pipeline. Use them
for runtime content — mods, user-generated sprites, downloaded packs — and
for editor tooling.

Three classes are registered:

- `AseDocument` — parse a file and query/render it.
- `AseResourceLoader` — makes plain `load()` of `.aseprite` paths work in
  running games. You never instantiate it.
- `AseTilesetSync` — editor-side helper that updates a TileSet you own.

## AseDocument

A parsed Aseprite document. Extends RefCounted. Construct it with the static
`open()` method — do not use `AseDocument.new()`; a document that was not
produced by `open()` holds no file, and calling any accessor on it is a bug
(it reports the engine error `AseDocument used before open()`).

### open(path: String) -> AseDocument (static)

Loads and parses a file. `path` is anything `FileAccess` can read:
`res://`, `user://`, or an absolute path. Returns null on failure and logs
the reason as an engine error in the form:

```text
AseDocument.open: <path>: <details>
```

Failures include an unreadable path (`cannot read <path>`) and parse errors,
which carry the exact byte offset of the problem (see
[troubleshooting.md](troubleshooting.md)).

The document is fully parsed in memory; keep the reference around and query
it as often as you like.

Note that `AseDocument` reads the file as authored: layers hidden in
Aseprite stay hidden when rendering, and the importer options
(`exclude_layers`, `include_hidden_layers`) do not apply here.

### get_size() -> Vector2i

Canvas size in pixels (width, height).

### get_frame_count() -> int

Number of frames.

### get_frame_duration_ms(frame: int) -> int

Duration of `frame` in milliseconds. Returns 0 for an out-of-range index.

### get_layer_names() -> PackedStringArray

All layer names, in file order (bottom to top).

### get_tag_names() -> PackedStringArray

All tag names, in file order.

### get_tag_range(name: String) -> Vector2i

The tag's frame range as `(from, to)`, both inclusive, zero-based. Returns
`(-1, -1)` if no tag has that name. Name matching is exact and
case-sensitive.

### get_user_data() -> Dictionary

Sprite-level user data. Keys, each omitted when not set in the file:

| Key | Type | Meaning |
|---|---|---|
| `text` | String | user-data text |
| `color` | Color | user-data color |

### get_slices(frame: int) -> Array

All slices with a key in effect at `frame` (a slice key stays in effect
until the next key). Negative frame indices are clamped to 0. One Dictionary
per slice:

| Key | Type | Present | Meaning |
|---|---|---|---|
| `name` | String | always | slice name |
| `rect` | Rect2i | always | slice bounds in canvas coordinates (position can be negative) |
| `center` | Rect2i | 9-patch slices only | center rect, relative to the slice bounds |
| `pivot` | Vector2i | only if set | pivot, relative to the slice origin |
| `text` | String | only if set | the slice's user-data text |

Two kinds of slice are excluded from the result: slices whose first key is
at a later frame (not defined yet), and slices hidden at this frame
(Aseprite marks a slice hidden from a frame onward with a zero-size key).

### render_frame(frame: int) -> Image

Flattens one frame to an Image in RGBA8 with straight alpha, exactly as
Aseprite renders it (all blend modes, layer/cel opacity, groups, tilemaps).
Returns null on failure and logs:

```text
AseDocument.render_frame(<frame>): <details>
```

An out-of-range index fails with `frame <n> out of range`.

### render_texture(frame: int) -> ImageTexture

Convenience wrapper: the rendered frame as a ready-to-use texture. Same
failure behavior, logged as `AseDocument.render_texture(<frame>): <details>`.

### Worked example: runtime-loaded mod content

A mod ships a raw `.aseprite` file under `user://`. Load it, put the first
frame on a sprite, and read a slice for a hitbox:

```gdscript
func load_mod_sprite() -> void:
    var doc := AseDocument.open("user://mods/slime.aseprite")
    if doc == null:
        return  # reason is in the error log

    $Sprite2D.texture = doc.render_texture(0)

    # Drive a tag manually.
    var walk: Vector2i = doc.get_tag_range("walk")
    if walk != Vector2i(-1, -1):
        _play_frames(doc, walk.x, walk.y)

    # Slices as hitboxes: the artist drew a "hurtbox" slice in Aseprite.
    for slice in doc.get_slices(0):
        if slice["name"] == "hurtbox":
            var rect: Rect2i = slice["rect"]
            $Hurtbox.position = rect.position
            $Hurtbox/CollisionShape2D.shape.size = rect.size

func _play_frames(doc: AseDocument, from: int, to: int) -> void:
    for i in range(from, to + 1):
        $Sprite2D.texture = doc.render_texture(i)
        await get_tree().create_timer(
            doc.get_frame_duration_ms(i) / 1000.0).timeout
```

For per-frame animation of slice rects driven by the import pipeline instead
of hand-rolled code, see the `slice_tracks` option in
[cookbook.md](cookbook.md).

## AseResourceLoader

A ResourceFormatLoader that makes plain `load()`/`preload()` of `.aseprite`
and `.ase` paths work in running games:

```gdscript
var tex: ImageTexture = load("user://mods/portrait.aseprite")
```

The result is the composited first frame as an ImageTexture. You never
instantiate or touch this class; the extension registers it automatically —
but only outside the editor. In the editor the import pipeline owns
`.aseprite` files, and `load()` there returns whatever the file's chosen
importer produced (Texture2D, SpriteFrames, TileSet, ...).

It works for:

- `user://` and absolute paths — any raw `.aseprite` file on disk.
- `res://` paths whose raw file is present in the exported pck.

On failure it logs an engine error in the form:

```text
aseprite-gd runtime load: <path>: <details>
```

and the `load()` call fails with `ERR_FILE_CORRUPT`.

### Exporting raw .aseprite files

Godot exports the *imported* artifacts of your resources, not the source
files. If you want the raw `.aseprite` bytes in the pck (for
`AseDocument.open` or the runtime loader on `res://` paths), add `*.aseprite`
(and `*.ase` if you use that extension) to the export preset's
*Resources → Filters to export non-resource files/folders*.

## AseTilesetSync

Editor-side helper (a tool class — available in the editor, from
EditorScripts and `@tool` scripts). Updates a TileSet resource you own from
an Aseprite file, preserving everything authored in Godot: collision,
terrain, navigation, per-tile properties, other sources. The full semantics
— source matching by tileset id, tile survival rules, the `aseprite_text`
custom data layer — are documented in
[tileset-workflow.md](tileset-workflow.md).

### sync(tile_set: TileSet, path: String) -> int (static)

Syncs the file's embedded tilesets into `tile_set` in place. Returns the
number of atlas sources synced; 0 on failure, with the reason logged as an
engine error:

```text
AseTilesetSync.sync: tile_set is null
AseTilesetSync.sync: <path>: <details>
```

A file without embedded tilesets fails with
`no embedded tilesets in file`. A source id collision with a non-atlas
source fails with `TileSet source <id> exists but is not an atlas source`.

```gdscript
@tool # run from an EditorScript or a plugin
var tile_set: TileSet = load("res://world/tiles.tres")
var synced := AseTilesetSync.sync(tile_set, "res://art/terrain.aseprite")
if synced > 0:
    ResourceSaver.save(tile_set, "res://world/tiles.tres")
```

## Parsing untrusted files

Runtime loading is designed for untrusted input — mods, downloads,
user-generated content. All file access in the parser is bounds-checked
(no `unsafe` in parse paths), every size the file declares is validated
against hard allocation caps before memory is allocated (decompression
output, palette sizes, tile counts, nesting depth — see
`crates/ase-core/src/limits.rs`), and the parser and compositor are
continuously fuzzed. A hostile or corrupt file produces a structured error
with the exact byte offset of the problem; it does not crash the game.
