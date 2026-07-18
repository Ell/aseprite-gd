# Yazi previewers

## aseprite.yazi

Preview `.aseprite`/`.ase` files in the yazi file manager, rendered by the
same compositor as the Godot importers. Scrolling the preview pane steps
through frames.

Setup:

1. Install the CLI: `cargo install --path crates/ase-cli` (provides `ase`).
2. Link the plugin:
   `ln -s "$(pwd)/tools/yazi/aseprite.yazi" ~/.config/yazi/plugins/aseprite.yazi`
3. Register the previewer in `~/.config/yazi/yazi.toml`:

```toml
[plugin]
prepend_previewers = [
  { url = "*.aseprite", run = "aseprite" },
  { url = "*.ase", run = "aseprite" },
]
```

## godot.yazi

Preview Godot `.tres`/`.res` resources as images, rendered by a headless
Godot one-shot (`godot` must be in PATH). Textures show as themselves —
AtlasTextures cropped to their region — TileSets show their first atlas
sheet, SpriteFrames their first frame, StyleBoxTextures and CanvasTextures
their texture. Small images upscale nearest-neighbor so pixel art stays
readable. The plugin walks up to the nearest `project.godot` so `res://`
references inside the resource resolve; non-visual `.tres` files fall back
to the text previewer.

Setup:

1. Link the plugin:
   `ln -s "$(pwd)/tools/yazi/godot.yazi" ~/.config/yazi/plugins/godot.yazi`
2. Register the previewer in `~/.config/yazi/yazi.toml`:

```toml
[plugin]
prepend_previewers = [
  { url = "*.tres", run = "godot" },
  { url = "*.res", run = "godot" },
]
```
