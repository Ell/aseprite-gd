# Yazi previewer

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
