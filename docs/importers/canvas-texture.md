# CanvasTexture (Aseprite)

Imports one frame as a `CanvasTexture` for lit 2D rendering: ordinary layers
become the diffuse texture, and layers named by convention become the normal
and specular maps. Everything stays in one file, in perfect registration.

## Quick start

1. In Aseprite, add a layer named `normal` (and optionally `specular` or
   `emission`) and paint the map there, aligned with your color art.
2. Select the file in the FileSystem dock, change the importer to
   *CanvasTexture (Aseprite)* in the Import dock, and click Reimport.
3. Assign the file to a `Sprite2D`'s `texture` property and add a
   `PointLight2D` or `DirectionalLight2D` to the scene.

## Options

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Comma-separated, case-sensitive substrings; layers whose names contain any of them are hidden, including layers revealed by `include_hidden_layers`. Empty disables the filter. |
| `include_hidden_layers` | `false` | Also render layers that are hidden in Aseprite. |
| `exclude_tags` | `""` | Comma-separated, case-sensitive substrings; tags whose names contain any of them produce no animations. |
| `post_import_script` | `""` | Path to a hook script whose `_post_import` runs on the built resource before it is saved — see [post-import-hooks.md](../post-import-hooks.md). |
| `frame` | `0` | Which frame to composite. Values past the last frame clamp to the last frame. |

## Naming convention

A layer is treated as a map layer when its name matches one of `normal`,
`specular`, or `emission` — either the exact name, or any name ending in
`_<kind>` or ` <kind>` (underscore or space before the suffix). Matching is
case-insensitive. Examples:

| Layer name | Treated as |
|---|---|
| `normal`, `Body_normal`, `body normal` | normal map |
| `specular`, `metal_specular` | specular map |
| `emission`, `eyes emission` | specular map (see notes) |
| `normals`, `abnormal` | color art (no match) |

Every layer that does not match is color art and composites into the
diffuse texture. Each map kind is rendered with only its matching layers
visible, so multiple `*_normal` layers composite together into one normal
map, and map layers never bleed into the diffuse.

## Walkthrough

`res://sprites/lantern.aseprite` has layers `glass`, `frame` (color art),
`lantern_normal` (a painted normal map), and `glow_emission`.

1. Import with *CanvasTexture (Aseprite)*. The result is a `CanvasTexture`
   with `glass` + `frame` as diffuse, `lantern_normal` as the normal
   texture, and `glow_emission` as the specular texture.
2. Assign it and light it:

```gdscript
var tex: CanvasTexture = load("res://sprites/lantern.aseprite")
$Sprite2D.texture = tex

var light := PointLight2D.new()
light.texture = preload("res://fx/light_falloff.png")
add_child(light)
```

The normal map shifts shading as the light moves; the specular channel
brightens highlights. Flat lighting with no normal response usually means
the map layer name did not match the convention — check the table above.

## Notes

- Hidden layers are excluded by default; `exclude_layers` wins over
  `include_hidden_layers`. Both filters apply before the diffuse/map split,
  so you can exclude a map variant by name.
- `CanvasTexture` has no emission slot in Godot, so `emission`-named layers
  land in the specular texture. If a file has both `specular` and
  `emission` layers, they composite together into that one texture.
- A map whose composite is fully transparent is omitted (that slot on the
  `CanvasTexture` stays null). If no color layers are visible, the import
  fails with `no visible color layers`.
- Normal maps use the standard encoding: flat is `(128, 128, 255)`. Paint in
  RGB even in indexed files — indexed and grayscale files are handled
  transparently, but a grayscale file cannot encode a normal map.
- Only one frame imports. Animated lit sprites are out of scope for this
  importer.
