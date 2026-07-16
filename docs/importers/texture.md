# Texture2D (Aseprite)

The default importer. One composited frame of the file becomes an
`ImageTexture`, so a `.aseprite` file dropped into the project works anywhere
a plain image would.

## Quick start

1. Save a `.aseprite` (or `.ase`) file anywhere inside your project. It
   imports with this importer automatically — no setup.
2. Select the file in the FileSystem dock. In the Import dock the importer
   reads *Texture2D (Aseprite)*; adjust options if needed and click Reimport.
3. Use the file like any texture: drag it onto a `Sprite2D`, a
   `TextureRect`, a material slot, or `load()` it from a script.

## Options

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Case-sensitive substring matched against layer names; matching layers are hidden, including layers revealed by `include_hidden_layers`. Empty disables the filter. |
| `include_hidden_layers` | `false` | Also render layers that are hidden in Aseprite. |
| `frame` | `0` | Which frame to composite. Values past the last frame clamp to the last frame; negative values clamp to `0`. |

## Walkthrough

Suppose `res://sprites/crate.aseprite` has three layers: `wood`, `shading`,
and a hidden `sketch` layer with construction lines.

1. Select the file. The Import dock shows *Texture2D (Aseprite)* with the
   options above.
2. The `sketch` layer is hidden in Aseprite, so it is already excluded — the
   import respects Aseprite layer visibility.
3. If you later want a variant without shading, set `exclude_layers` to
   `shading` and click Reimport. Any layer whose name contains that substring
   is hidden.

```gdscript
var crate: Texture2D = load("res://sprites/crate.aseprite")
$Sprite2D.texture = crate
```

The result is composited with Aseprite's exact blend math — all blend modes,
layer and cel opacity, and groups render identically to Aseprite.

## Notes

- Hidden layers are excluded by default; set `include_hidden_layers` to bring
  them in. `exclude_layers` wins when both apply to the same layer.
- Indexed and grayscale files are handled transparently; the output is always
  RGBA.
- Multi-frame files import only the frame selected by `frame`. For animation,
  switch to the [SpriteFrames](sprite-frames.md) or
  [AnimationLibrary](animation-library.md) importer.
- In a running game (outside the editor), plain `load()` of a raw
  `.aseprite` file that never went through the import pipeline — for example
  from `user://` — also returns an `ImageTexture` of frame 0, via the
  runtime loader.
