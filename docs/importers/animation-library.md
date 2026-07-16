# AnimationLibrary (Aseprite)

Imports the file as an `AnimationLibrary` for `AnimationPlayer`: one
`Animation` per tag, each with a texture value track that swaps atlas frames
at exact times. Cel user data becomes method tracks, and slices can
optionally drive animated hitbox tracks.

## Quick start

1. Select the `.aseprite` file in the FileSystem dock.
2. In the Import dock, change the importer to *AnimationLibrary (Aseprite)*
   and click Reimport.
3. Build a scene where the `AnimationPlayer`'s root node has a `Sprite2D`
   child named `Sprite2D` (or set the `sprite_path` option to match your
   layout — the path is resolved from the player's root node).
4. In the AnimationPlayer panel, choose Animation → Manage Animations →
   Load Library and pick the `.aseprite` file, or add it from a script:

```gdscript
@onready var player: AnimationPlayer = $AnimationPlayer

func _ready() -> void:
    player.add_animation_library("player", load("res://sprites/player.aseprite"))
    player.play("player/walk")
```

## Options

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Comma-separated, case-sensitive substrings; layers whose names contain any of them are hidden, including layers revealed by `include_hidden_layers`. Empty disables the filter. |
| `include_hidden_layers` | `false` | Also render layers that are hidden in Aseprite. |
| `exclude_tags` | `""` | Comma-separated, case-sensitive substrings; tags whose names contain any of them produce no animations. |
| `post_import_script` | `""` | Path to a hook script whose `_post_import` runs on the built resource before it is saved — see [post-import-hooks.md](../post-import-hooks.md). |
| `atlas_padding` | `1` | Pixels of space between packed frames (0-16). |
| `atlas_extrude` | `false` | Replicate each frame's edge pixels one pixel into the padding gutter — prevents bleeding under filtering or mipmaps. Needs padding of at least 1. |
| `split_layers` | `false` | Each animation gains one texture track per visible leaf layer, targeting `<sprite_path>/<layer>:texture` — `sprite_path` becomes the container holding one sprite child per layer. One animation drives every layer in sync. |
| `sprite_path` | `"Sprite2D"` | Node path (relative to the AnimationPlayer's root node) that the texture and method tracks target. An empty value falls back to `Sprite2D`. |
| `slice_tracks` | `false` | Also emit `<slice name>:position` and `<slice name>:size` value tracks from per-frame slice keys. |

## What maps to what

- **Tags → animations.** Same rules as the SpriteFrames importer: a file
  with no tags becomes one looping `default` animation; a tag loops unless it
  has a finite repeat count; reverse and ping-pong directions are unrolled
  into the key order.
- **Texture track.** Each animation gets a discrete value track on
  `<sprite_path>:texture`, keyed with `AtlasTexture` frames (from the same
  shared trimmed atlas the SpriteFrames importer uses) at the exact
  millisecond each frame starts. The animation length is the sum of the
  frame durations.
- **Cel user data text → method tracks.** Any cel whose user data text is
  non-empty adds a method-call key at that frame's start time, calling the
  text as a method name (no arguments) on the `sprite_path` node.

## Frame-accurate gameplay events from cel user data

To author an event in Aseprite:

1. In the timeline, select the cel on the frame where the event should fire
   (for example the frame where the foot hits the ground in `walk`).
2. Right-click the cel and choose Cel Properties.
3. Expand the user data section (the clipboard icon in the properties
   popup) and type the method name into the text field, e.g. `footstep`.
4. Save and reimport.

The importer turns that into a method track keyed at the frame's start,
so the node at `sprite_path` needs a matching method:

```gdscript
# Attached to the Sprite2D at sprite_path.
func footstep() -> void:
    $FootstepAudio.play()
```

The text must be exactly the method name. One method track is created per
animation; multiple cels with text (on any layer) each add a key.

## Animated hitboxes from slices (`slice_tracks`)

With `slice_tracks` enabled, every slice that has keys produces two discrete
value tracks per animation: `<slice name>:position` and `<slice name>:size`,
both `Vector2`, keyed from the slice's per-frame rects. Draw a slice named
`hurtbox` in Aseprite, move and resize it per frame, then add a child node
named `hurtbox` under the AnimationPlayer's root node — for example an
`Area2D`, repositioned by the track, or a `ColorRect` for debugging. Frames
where the slice has zero width or height (hidden) get no key, so the last
rect stays in effect.

## Notes

- Hidden layers are excluded by default; `exclude_layers` wins over
  `include_hidden_layers` when both apply to a layer.
- Tracks are keyed in seconds internally but derived from millisecond
  durations, so timings are exact.
- The texture track uses discrete updates: frames switch instantly, no
  interpolation.
- `sprite_path` is baked into the imported resource. If you rename or move
  the sprite node, update the option and reimport.
- Slice track paths are the slice name verbatim; slice names that are not
  valid node names (or nodes that do not exist) leave those tracks inert.
- Indexed and grayscale files are handled transparently.

`create_reset_animation` adds a RESET animation keying frame 0 on every
texture track, defining the neutral pose for the editor and AnimationMixer.
