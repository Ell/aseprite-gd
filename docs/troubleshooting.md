# Troubleshooting

Real failure modes, what they mean, and how to fix them.

## My edits to an imported resource disappeared

Imported resources are derived artifacts: every reimport regenerates them
from the `.aseprite` source, discarding anything you changed on the
resource itself. This is how Godot's import pipeline works for all imported
formats, and this plugin follows it.

If you need to author data on top of Aseprite content — collision polygons,
terrain, navigation, per-tile properties — put it in a resource you own and
have the plugin update only the Aseprite-derived parts. For tilesets that
workflow exists today: see [tileset-workflow.md](tileset-workflow.md) and
`AseTilesetSync.sync` in [runtime-api.md](runtime-api.md).

## The file didn't reimport after I hand-edited the .import file

Godot triggers reimports when the source file's content changes or when you
change import settings *through the editor* (the Import dock). Editing the
`.import` file by hand can leave the editor's cache thinking nothing
changed. Fix: select the file and click *Reimport* in the Import dock, or
delete the file's artifacts under `.godot/imported/` and let the editor
rescan.

## Cold headless import exits nonzero after importing successfully

Running `godot --headless --path <project> --import` on a machine with no
editor cache (CI, fresh clone) can exit with a nonzero status *after* all
imports completed successfully. This is an engine bug
([godot#111645](https://github.com/godotengine/godot/issues/111645)): on
the first scan of a project with a GDExtension, a deferred
documentation-generation call runs during editor teardown against an
already-freed singleton. Warm runs short-circuit through the editor cache
and exit cleanly.

Your files did import; only the exit code is wrong. Workarounds:

- Run `--import` twice and only gate on the second run's exit code — this
  is what this repository's own CI does.
- Run under `xvfb-run` instead of `--headless`.

Do check the log of the tolerated first run for real errors (for example
`Can't open dynamic library`) so a genuinely broken setup doesn't hide
behind the known-bad exit code.

## "extension class ... unavailable (library not loaded?)"

On editor start, the addon logs one error per importer it cannot
instantiate:

```text
aseprite-gd: extension class AseTextureImporter unavailable (library not loaded?)
```

This means the GDExtension library itself did not load, so none of the
Rust classes exist. The `[libraries]` section of
`addons/aseprite_gd/aseprite_gd.gdextension` maps platform/architecture
feature tags to library paths; the error appears when no entry matches your
platform, or the entry points at a file that does not exist.

Check that the library for your platform is actually at the configured path
(in this repository's demo project that is `../target/debug/` or
`../target/release/` relative to the Godot project — build it with
`cargo build -p aseprite-gd`). Note that editor builds select the `debug`
variant of each entry, exported release games the `release` variant, so
both paths need to resolve for a full workflow.

## Import errors mention a byte offset

Parse errors always carry the absolute byte offset of the problem, so a
message like:

```text
unexpected end of file at offset 4096 (needed 16 more bytes)
bad magic at offset 0: expected 0xA5E0, found 0x4D5A
invalid <field> at offset <n>
invalid UTF-8 string at offset <n>
```

means the file is corrupt, truncated, or not an Aseprite file at all — the
parser read valid data up to that offset and hit something the format does
not allow. A truncated download or an interrupted save are the common
causes. Re-export or re-copy the file; if Aseprite itself opens the file
fine, report the offset in a bug — it pinpoints the disagreement.

## "no embedded tilesets in file"

The TileSet importer and `AseTilesetSync.sync` need at least one tileset
with pixel data embedded in the file. You get this error when the file has
no tilesets at all (no tilemap layer was ever created) or when its tilesets
are external references to another file, which carry no pixels.

Fix in Aseprite: create a tilemap layer (*Layer → New → New Tilemap Layer*)
and draw tiles into it — tilesets created this way are embedded. External
tileset files are not supported.

## "a sheet path with multiple embedded tilesets is not supported yet"

`AseTilesetSync.sync_with_sheet` writes one sheet file, and a file with
several embedded tilesets would need one per source. Use plain `sync` for
those files (textures embed in the TileSet), or split the tilesets across
separate Aseprite files.

## "no 9-patch slice in file"

The StyleBoxTexture importer, with `slice_name` left empty, picks the first
slice that has a center rect — that is what makes a slice a 9-patch. No
slice in the file has one.

Fix in Aseprite: select the slice, open its properties, and enable
*9-slices* to give it a center rect. Alternatively set the `slice_name`
import option to a specific slice; if the name does not exist you get
`no slice named "<name>"` instead.

## "slice is hidden at this frame" / "slice has no key at this frame"

Slice keys are per-frame in Aseprite: a key takes effect at its frame and
stays in effect until the next key. Two related errors from the
StyleBoxTexture importer:

- `slice has no key at this frame` — the slice's first key is at a later
  frame than the one being imported.
- `slice is hidden at this frame` — the key in effect has zero width or
  height, which is how the format records a slice hidden from a frame
  onward.

Fix in Aseprite by adjusting the slice's keys so it exists and is visible
at the imported frame, or point the importer's `frame` option at a frame
where it is. The same visibility rules apply to `AseDocument.get_slices`,
which silently omits such slices rather than erroring.

## Hidden layers don't show up in the import

By default, layers hidden in Aseprite are excluded from rendering — imports
match what Aseprite displays. To include them, enable the
`include_hidden_layers` import option (available on every importer). The
converse also exists: `exclude_layers` hides any layer whose name contains
the given substring, useful for guide or reference layers you keep visible
while drawing.

## Very large files are rejected

Parsing enforces hard safety limits on everything size-like the file
declares — decompressed image bytes (per image and per file), canvas
dimensions, palette entries, tile counts, user-data nesting depth. The
constants live in `crates/ase-core/src/limits.rs` and are deliberately
generous for legitimate art (for example, a single cel may decompress to
256 MiB — a 4096×4096 RGBA cel is 64 MiB) and deliberately fatal for
decompression bombs. A file over a limit is rejected with:

```text
safety limit exceeded at offset <n>: <which limit>
```

Rejection is not clamping: no partial result is produced. If a legitimate
file trips a limit, that is worth a bug report.
