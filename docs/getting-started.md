# Getting started

aseprite-gd reads `.aseprite` and `.ase` files directly — Aseprite does not
need to be installed. Files import in the editor, on teammates' machines,
and in headless CI with no configuration.

## Requirements

- Godot 4.2 or newer
- A project on Linux (x86_64), Windows (x86_64), or macOS

## Install

1. Download the release zip and extract it into your project so you have
   `addons/aseprite_gd/` (containing `aseprite_gd.gdextension`, `plugin.cfg`,
   `plugin.gd`, and `bin/` with the platform libraries).
2. Open the project. Godot loads the extension; if the editor was already
   open, restart it once.
3. Enable the plugin: Project → Project Settings → Plugins → aseprite-gd.

The plugin registers the importers. Without step 3 the extension classes
still exist (the runtime API works), but `.aseprite` files won't import.

## First import

Copy any `.aseprite` file into your project. After the file system scan it
imports as a `Texture2D` — drag it onto a Sprite2D and it renders exactly as
Aseprite flattens it: layer visibility, opacity, blend modes, groups, and
tilemap layers all match Aseprite's own output.

To get a different resource from the same file, select it in the FileSystem
dock, open the Import dock, and change **Import As**:

| Import As | Resource | Use for |
|---|---|---|
| Texture2D (Aseprite) | `ImageTexture` | Static images, single frames |
| SpriteFrames (Aseprite) | `SpriteFrames` | AnimatedSprite2D / AnimatedSprite3D |
| AnimationLibrary (Aseprite) | `AnimationLibrary` | AnimationPlayer workflows |
| TileSet (Aseprite) | `TileSet` | Tilemaps |
| StyleBoxTexture (Aseprite) | `StyleBoxTexture` | 9-patch UI panels |
| CanvasTexture (Aseprite) | `CanvasTexture` | Lit (2D-lighting) sprites |

Press **Reimport** after changing the importer or its options.

## Walkthroughs

Each importer has a guide under [importers/](importers/):
[texture](importers/texture.md),
[sprite-frames](importers/sprite-frames.md),
[animation-library](importers/animation-library.md),
[tileset](importers/tileset.md),
[stylebox](importers/stylebox.md),
[canvas-texture](importers/canvas-texture.md).

Shorter task-oriented recipes live in the [cookbook](cookbook.md), and
[aseprite-conventions.md](aseprite-conventions.md) lists everything the
extension reads from your art files. Working
example scenes are in the repository's demo project under `godot/examples/`.

## Options common to all importers

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Case-sensitive substring; layers whose names contain it are hidden for this import |
| `include_hidden_layers` | `false` | Render layers that are hidden in Aseprite too |

## Scripting and runtime loading

Running games can `load()` `.aseprite` files directly, and the `AseDocument`
class exposes parsing, rendering, tags, and slices to GDScript — see
[runtime-api.md](runtime-api.md). For tilemaps with Godot-authored collision
or terrain, see [tileset-workflow.md](tileset-workflow.md).

If something behaves unexpectedly, check
[troubleshooting.md](troubleshooting.md).
