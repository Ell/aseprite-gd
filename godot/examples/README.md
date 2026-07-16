# Example scenes

Each scene uses resources imported straight from the `.aseprite` files in
`../sprites/` — open one and run it to see the corresponding importer in use.

| Scene | Shows |
|---|---|
| `animated_character.tscn` | AnimatedSprite2D playing a SpriteFrames import (tags as animations) |
| `animation_player.tscn` | AnimationPlayer with an imported AnimationLibrary |
| `ui_panel.tscn` | Panel themed with a StyleBoxTexture from a 9-patch slice |
| `lit_sprite.tscn` | Sprite2D with a CanvasTexture (normal-map layer) under a PointLight2D |

The `.import` files next to the sprites pin which importer each file uses —
that's the only setup. `verify_import.gd` in the project root asserts all of
these load correctly and is run by CI.
