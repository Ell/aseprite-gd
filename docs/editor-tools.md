# Editor tools

Beyond the importers, the plugin adds these pieces of editor UI.

## FileSystem thumbnails

Every `.aseprite`/`.ase` file shows its composited first frame as its
thumbnail in the FileSystem dock and file dialogs (grid display mode),
whatever importer it uses. Godot only previews files whose imported product
is a texture; the plugin's generator covers the rest — SpriteFrames,
TileSet, AnimationLibrary, StyleBoxTexture, CanvasTexture, and scene
imports. Thumbnails refresh when the file changes, like any other preview.

## Inspector import section

Selecting an AnimationPlayer, AnimatedSprite2D, or AnimatedSprite3D shows an
"Aseprite Import" section at the top of the Inspector.

For an **AnimationPlayer**: pick a file, set the sprite path (from the
player's root node to your sprite), choose options, and press Import. The
file's animations merge into the player's library non-destructively:

- tracks you added by hand (other node paths) survive re-imports;
- animations the file doesn't produce are never touched;
- imported tracks are replaced in place, never duplicated.

The file and options are stored on the node, so "Reimport last" repeats the
import with one click after the art changes.

For an **AnimatedSprite2D/3D**: pick a file and Import assigns a generated
SpriteFrames to the node, with the same remembered-options behavior.

The same operations are scriptable — the section is a thin layer over:

```gdscript
AseAnimationImport.merge_into_player(player, "res://hero.aseprite",
        {"sprite_path": "Sprite2D", "slice_tracks": true})
AseAnimationImport.reimport(player)
AseAnimationImport.assign_sprite_frames(sprite, "res://hero.aseprite", {})
```

## Tools menu

Project → Tools → "aseprite-gd: Reimport all Aseprite files" re-runs every
`.aseprite`/`.ase` import in the project — useful after changing shared
import defaults or updating the plugin.
