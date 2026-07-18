# SpriteFrames (Aseprite)

Imports the file as a `SpriteFrames` resource for `AnimatedSprite2D` /
`AnimatedSprite3D`. Every tag becomes an animation with exact per-frame
timings, and all frames share one trimmed, deduplicated atlas.

## Quick start

1. Select the `.aseprite` file in the FileSystem dock.
2. In the Import dock, change the importer to *SpriteFrames (Aseprite)* and
   click Reimport.
3. Add an `AnimatedSprite2D` to your scene and assign the file to its
   `sprite_frames` property (drag the file from the FileSystem dock, or
   `load()` it).

## Options

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Comma-separated, case-sensitive substrings; layers whose names contain any of them are hidden, including layers revealed by `include_hidden_layers`. Empty disables the filter. |
| `include_hidden_layers` | `false` | Also render layers that are hidden in Aseprite. |
| `exclude_tags` | `""` | Comma-separated, case-sensitive substrings; tags whose names contain any of them produce no animations. |
| `post_import_script` | `""` | Path to a hook script whose `_post_import` runs on the built resource before it is saved — see [post-import-hooks.md](../post-import-hooks.md). |
| `atlas_padding` | `1` | Pixels of space between packed frames (0-16). |
| `atlas_extrude` | `false` | Replicate each frame's edge pixels one pixel into the padding gutter — prevents bleeding under filtering or mipmaps. Needs padding of at least 1. |
| `scale` | `1` | Integer nearest-neighbor upscale (1-8) applied to output pixels. |
| `compress_mode` | `Lossless` | Texture storage: embedded lossless, PortableCompressedTexture2D lossless, or lossy. |
| `snap_to_fps` | `0` | When above 0, re-time frame durations to this frame rate's tick grid (exact milliseconds otherwise). |
| `split_layers` | `false` | One animation per visible leaf layer per tag, named `<layer>/<tag>`, all sharing one atlas. Stack one AnimatedSprite2D per layer and play the same tag on each for multi-layer characters. |
| `split_grid` | `""` | "WxH" (e.g. "16x16"): chop the canvas into cells, row-major, partial edge cells dropped. A single-frame sheet becomes one `default` animation indexable by cell; a multi-frame file becomes one animation per cell (named "0", "1", ...) playing that cell across the frames — stacked animation sets. |

## What maps to what

- **Tags → animations.** Each tag becomes an animation with the tag's name.
  A file with no tags becomes a single looping animation named `default`
  covering every frame.
- **Frame durations.** Animation speed is fixed at 1000 FPS and each frame's
  relative duration is its Aseprite duration in milliseconds, so a 120 ms
  frame plays for exactly 120 ms regardless of the other frames.
- **Loops.** A tag with no repeat count set in Aseprite (the default) loops;
  a tag with a finite repeat count imports as non-looping.
- **Direction.** Reverse tags import reversed. Ping-pong and ping-pong
  reverse tags are unrolled into the frame list (`0,1,2,3` becomes
  `0,1,2,3,2,1`) without doubling the endpoints, matching Aseprite playback.

## Walkthrough

`res://sprites/player.aseprite` has tags `idle`, `walk`, and `attack`, where
`attack` has a repeat count of 1.

```gdscript
@onready var sprite: AnimatedSprite2D = $AnimatedSprite2D

func _ready() -> void:
    sprite.sprite_frames = load("res://sprites/player.aseprite")
    sprite.play("idle")

func attack() -> void:
    sprite.play("attack")   # non-looping: repeat count set in Aseprite
    await sprite.animation_finished
    sprite.play("idle")
```

## The shared atlas

Every frame is rendered, trimmed to its non-transparent bounding box, and
deduplicated (identical frames — including linked cels — are stored once).
The unique images are packed into shared atlas pages, and each animation
frame is an `AtlasTexture` into those pages. Margins restore the trimmed-away
border, so every frame still reports and renders at full canvas size —
nothing shifts between frames:

```gdscript
var sf: SpriteFrames = load("res://sprites/player.aseprite")
var tex := sf.get_frame_texture("idle", 0)
print(tex)             # AtlasTexture
print(tex.get_size())  # canvas size, e.g. (32, 32)
```

Atlas pages stay under Godot's 16384 px texture-dimension cap; very large
files split across multiple pages automatically.

## Notes

- Hidden layers are excluded by default; `exclude_layers` wins over
  `include_hidden_layers` when both apply to a layer.
- When the file has tags, no `default` animation is created — only the tags.
- Fully transparent frames are kept (as a 1x1 transparent region), so frame
  counts and timings always match the source file.
- Indexed and grayscale files are handled transparently.
- Changing `AnimatedSprite2D.speed_scale` still works as usual; the 1000 FPS
  base only exists so per-frame millisecond durations are exact.
