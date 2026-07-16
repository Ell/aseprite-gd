# StyleBoxTexture (Aseprite)

Imports a 9-patch slice as a `StyleBoxTexture`: the slice rect is cropped
out of the rendered frame and the slice's center rect becomes the four
texture margins. Author the panel once in Aseprite and use it in any Control
theme.

## Quick start

1. In Aseprite, draw your panel and mark it with a 9-patch slice (see the
   walkthrough below).
2. Select the file in the FileSystem dock, change the importer to
   *StyleBoxTexture (Aseprite)* in the Import dock, and click Reimport.
3. Assign the file to a theme override on a Control, e.g. a `Panel`'s
   `theme_override_styles/panel`.

## Options

| Option | Default | Effect |
|---|---|---|
| `exclude_layers` | `""` | Comma-separated, case-sensitive substrings; layers whose names contain any of them are hidden, including layers revealed by `include_hidden_layers`. Empty disables the filter. |
| `include_hidden_layers` | `false` | Also render layers that are hidden in Aseprite. |
| `post_import_script` | `""` | Path to a hook script whose `_post_import` runs on the built resource before it is saved — see [post-import-hooks.md](../post-import-hooks.md). |
| `slice_name` | `""` | Which slice to import, by exact name. Empty selects the first slice that has a 9-slices center. |
| `frame` | `0` | Which frame to render and take the slice key from. Values past the last frame clamp to the last frame. |

## Authoring the slice in Aseprite

1. Pick the slice tool in the toolbar and drag a rectangle around the panel
   art. This creates a named slice (`Slice 1` by default — rename it in the
   next step if you plan to use `slice_name`).
2. Right-click the slice and open Slice Properties.
3. Check the *9-slices* checkbox. A center rect appears inside the slice;
   drag its edges so the center covers the stretchable interior and the
   borders cover the corners and edges you want kept at fixed size.
4. Save the file.

The importer crops the slice rect out of the composited frame and derives
the margins from the center rect: left and top margins are the center's
offset within the slice; right and bottom are the remaining border widths.

## Using it in a theme override

```gdscript
var style: StyleBoxTexture = load("res://ui/panel.aseprite")
$Panel.add_theme_stylebox_override("panel", style)
```

Or assign the `.aseprite` file directly in the Inspector under
Theme Overrides → Styles.

For the button-versus-panel case, keep one file with several slices and
import the same art multiple ways: duplicate the `.aseprite` file, or point
two files at different `slice_name` values.

## Notes

- Hidden layers are excluded by default; `exclude_layers` wins over
  `include_hidden_layers`.
- With an empty `slice_name`, a file with no 9-patch slice fails to import
  (`no 9-patch slice in file`). A named slice does not need a center — but
  without one, no margins are set and the whole texture stretches.
- The slice must have a key at (or before) the chosen `frame` and must not
  be hidden (zero-sized) there; the slice rect is clamped to the canvas.
- If the slice's rect or center animates across frames, use `frame` to pick
  which key you import.
- Indexed and grayscale files are handled transparently.
