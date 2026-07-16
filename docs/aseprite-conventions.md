# Aseprite conventions reference

Everything the extension reads from your art files, in one place. All of it
is authored with stock Aseprite features — no naming schemes beyond the layer
names listed here, no external metadata.

## Tags

| In Aseprite | In Godot |
|---|---|
| Tag | One animation per tag (SpriteFrames and AnimationLibrary) |
| No tags at all | A single `default` animation over all frames |
| Tag direction: forward / reverse / ping-pong / ping-pong reverse | Frame order, with ping-pong unrolled (endpoints not doubled) |
| Repeat = 0 (the default "infinity" in the UI) | Looping animation |
| Repeat = any number | Non-looping animation |
| Frame durations (ms) | Exact playback timing — no FPS rounding |

## Layers

| In Aseprite | In Godot |
|---|---|
| Hidden layer | Excluded from every import (opt back in with `include_hidden_layers`) |
| Layer opacity, blend mode, groups | Composited exactly as Aseprite renders them |
| Layer named `normal` (or ending in `_normal` / ` normal`) | Normal map on CanvasTexture imports; excluded from color output |
| Layer named `specular` or `emission` (same suffix rules) | Specular map on CanvasTexture imports |
| Reference layers | Never rendered |

## User data

Set user data via right-click → Properties on the object.

| Attached to | In Godot |
|---|---|
| A cel (text) | AnimationLibrary: a Call Method track keys that method name at the frame's start |
| A tag | Available to scripts via `AseDocument` |
| A tile (text) | `aseprite_text` custom data layer on imported/synced TileSets |
| A slice (text) | `text` key in `AseDocument.get_slices()` entries |
| The sprite (text/color) | `AseDocument.get_user_data()` |

## Slices

| In Aseprite | In Godot |
|---|---|
| Slice with 9-slices enabled | StyleBoxTexture import: center rect becomes the four texture margins |
| Slice pivot | `pivot` key in `AseDocument.get_slices()` entries |
| Slice moved/resized per frame (slice keys) | Animated `<slice name>:position` / `:size` tracks (AnimationLibrary with `slice_tracks` on) |
| Slice bounds | `rect` key in `AseDocument.get_slices()` entries |

## Tilemaps

| In Aseprite | In Godot |
|---|---|
| Tileset | A `TileSetAtlasSource` (source id = Aseprite tileset id) |
| The empty tile (id 0) | Skipped |
| Tile flips on map cells | Composite correctly in rendered frames; Godot cells carry their own flip bits |
| Tilemap layer contents | Composited into rendered frames like any layer |

## Color modes

RGBA, grayscale, and indexed files all work; indexed transparency and
per-frame palettes follow Aseprite's rules. Files from old Aseprite versions
(back to the pre-1.2 format) load.
