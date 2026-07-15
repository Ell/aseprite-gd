# Aseprite (.aseprite / .ase) Binary File Format — Complete Parser Reference

Sources: canonical spec (`docs/ase-file-specs.md` @ aseprite/aseprite main, fetched 2026-07-15), Aseprite source `src/dio/aseprite_decoder.cpp`, `src/dio/aseprite_common.h`, `src/doc/blend_funcs.cpp`, `src/doc/blend_internals.h` (pixman macros), and dacap's gist "Aseprite file format differences from v1.2 → v1.3".

**All values are little-endian (Intel byte order).** `.ase` and `.aseprite` are the *same format* (same magic `0xA5E0`); the extension difference is cosmetic. There is **no `0xA5E1` magic** — that value appears nowhere in the spec or source. The format is structurally derived from FLI/FLC (whose magics are `0xAF11`/`0xAF12`), but shares only the frame-of-chunks concept and the 128-byte header size.

## 1. Primitive types

| Type | Size | Meaning |
|---|---|---|
| `BYTE` | 1 | u8 |
| `WORD` | 2 | u16 LE |
| `SHORT` | 2 | i16 LE |
| `DWORD` | 4 | u32 LE |
| `LONG` | 4 | i32 LE |
| `FIXED` | 4 | signed 16.16 fixed point (value = raw / 65536.0) |
| `FLOAT` | 4 | IEEE-754 single, LE |
| `DOUBLE` | 8 | IEEE-754 double, LE |
| `QWORD` | 8 | u64 LE |
| `LONG64` | 8 | i64 LE |
| `BYTE[n]` | n | raw bytes |
| `STRING` | 2+len | `WORD` length in **bytes**, then `BYTE[len]` UTF-8 characters. **No** NUL terminator, no padding. |
| `POINT` | 8 | `LONG` x, `LONG` y |
| `SIZE` | 8 | `LONG` w, `LONG` h |
| `RECT` | 16 | `POINT` origin, `SIZE` size |
| `PIXEL` | varies | RGBA: `BYTE[4]` R,G,B,A · Grayscale: `BYTE[2]` Value,Alpha · Indexed: `BYTE` index |
| `TILE` | 1/2/4 | tilemap cell; 8/16/32-bit per the cel's "bits per tile" (always 32 in practice) |
| `UUID` | 16 | `BYTE[16]` |

## 2. Overall file layout

```
+-------------------------------+
| Header (128 bytes)            |
+-------------------------------+
| Frame 0:                      |
|   Frame header (16 bytes)     |
|   Chunk, Chunk, Chunk, ...    |
+-------------------------------+
| Frame 1: header + chunks      |
| ...                           |
+-------------------------------+
```

Reading algorithm: read header → for each of `header.frames` frames: read 16-byte frame header, then read `nchunks` chunks; after each chunk `seek(chunk_start + chunk_size)`; after each frame `seek(frame_start + frame_size)`. Never rely on having consumed exactly the declared bytes — always seek. This is how forward compatibility works (newer versions append fields to chunks).

## 3. Header (128 bytes, at file offset 0)

| Offset | Size | Type | Field |
|---|---|---|---|
| 0 | 4 | DWORD | File size (bytes, whole file) |
| 4 | 2 | WORD | Magic = `0xA5E0` |
| 6 | 2 | WORD | Number of frames |
| 8 | 2 | WORD | Width in pixels |
| 10 | 2 | WORD | Height in pixels |
| 12 | 2 | WORD | Color depth (bpp): 32=RGBA, 16=Grayscale, 8=Indexed. Anything else → reject. |
| 14 | 4 | DWORD | Flags: bit 0 (`1`)=layer opacity field is valid; bit 1 (`2`)=blend mode/opacity valid for **groups** (composite groups separately when rendering); bit 2 (`4`)=layers carry a UUID |
| 18 | 2 | WORD | Speed: default frame duration in ms. DEPRECATED — use per-frame duration; only fall back to this when a frame's duration field is 0. |
| 20 | 4 | DWORD | Set to 0 |
| 24 | 4 | DWORD | Set to 0 |
| 28 | 1 | BYTE | Transparent color index (palette entry that is transparent in all non-background layers). **Indexed sprites only** — for 16/32 bpp force it to 0 and ignore (Aseprite does exactly this). |
| 29 | 3 | BYTE[3] | Ignore |
| 32 | 2 | WORD | Number of colors. **0 means 256** (old files). |
| 34 | 1 | BYTE | Pixel width (pixel ratio = pixel_width/pixel_height; if either is 0, ratio = 1:1) |
| 35 | 1 | BYTE | Pixel height |
| 36 | 2 | SHORT | Grid X position |
| 38 | 2 | SHORT | Grid Y position |
| 40 | 2 | WORD | Grid width (0 = no grid; UI default 16) |
| 42 | 2 | WORD | Grid height (0 = no grid) |
| 44 | 84 | BYTE[84] | Reserved (zero) — always seek to offset 128 |

## 4. Frame header (16 bytes)

| Offset | Size | Type | Field |
|---|---|---|---|
| 0 | 4 | DWORD | Bytes in this frame (header included) — use to seek to next frame |
| 4 | 2 | WORD | Magic = `0xF1FA` |
| 6 | 2 | WORD | Old chunk count. `0xFFFF` = maybe more chunks, consult new field. |
| 8 | 2 | WORD | Frame duration in ms (0 → use header Speed) |
| 10 | 2 | BYTE[2] | Reserved |
| 12 | 4 | DWORD | New chunk count (0 → use old field) |

Chunk count resolution: spec says "if new field is 0, use old field". Aseprite's actual decoder: `chunks = old; if (old == 0xFFFF && new > old) chunks = new;`. Encoder writes `old = min(count, 0xFFFF)`, `new = count`. Safe parser rule: **if old == 0xFFFF and new != 0, use new; else use old.**

## 5. Chunk envelope + chunk type registry

Each chunk:

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | DWORD chunk size — **includes these 6 header bytes**, so min value is 6 |
| 4 | 2 | WORD chunk type |
| 6 | size−6 | chunk data |

| ID | Name | Status |
|---|---|---|
| `0x0004` | Old palette (8-bit RGB components) | legacy, still written for small palettes |
| `0x0011` | Old palette (6-bit RGB components) | legacy |
| `0x2004` | Layer | current |
| `0x2005` | Cel | current |
| `0x2006` | Cel Extra | current |
| `0x2007` | Color Profile | current (v1.2.9+) |
| `0x2008` | External Files | current (v1.3+) |
| `0x2016` | Mask | DEPRECATED |
| `0x2017` | Path | never used — ignore |
| `0x2018` | Tags | current |
| `0x2019` | Palette | current |
| `0x2020` | User Data | current |
| `0x2021` | "Slices" | DEPRECATED — only in dev builds between v1.2-beta7 and v1.2-beta8 |
| `0x2022` | Slice | current |
| `0x2023` | Tileset | current (v1.3+) |

Skip unknown chunk types via the size field. **Note:** in Aseprite's decoder, an unknown/ignored chunk does *not* reset the "last object with user data" pointer — mimic this for user-data association correctness.

## 6. Chunk details

### 6.1 Old palette chunk `0x0004` (RGB 0–255) and `0x0011` (RGB 0–63)

Identical layout; only component range differs.

```
WORD   nPackets
for each packet:
  BYTE  skip      ; entries to skip from the last packet's end (cumulative; first starts at 0)
  BYTE  count     ; colors in packet, 0 means 256
  for each color:
    BYTE r, BYTE g, BYTE b       ; 0-255 for 0x0004, 0-63 for 0x0011
```

- For `0x0011`, scale 6-bit → 8-bit: `v8 = (v6 << 2) | (v6 >> 4)` (Aseprite's `scale_6bits_to_8bits`).
- Alpha is implicitly 255.
- **Ignore both old chunk types once a new Palette chunk (`0x2019`) has been seen** (Aseprite sets a global `ignore_old_color_chunks` flag at the first 0x2019). v1.1 wrote both for compatibility. v1.3.5+ writes 0x0004 alone when palette ≤256 colors and has no alpha; otherwise only 0x2019.

### 6.2 Layer chunk `0x2004`

All layer chunks appear in the **first frame**, in tree order (pre-order). The Nth layer chunk in the file has layer index N (counting groups!), which is what Cel chunks reference.

```
WORD    flags
WORD    layerType      ; 0=Normal(image), 1=Group, 2=Tilemap
WORD    childLevel     ; depth in the tree, see below
WORD    defaultWidth   ; ignored
WORD    defaultHeight  ; ignored
WORD    blendMode      ; see §9.3 table
BYTE    opacity        ; valid only if header flags bit 0 set; else assume 255
BYTE[3] reserved
STRING  name
if layerType == 2:
  DWORD tilesetIndex   ; index into tilesets declared by Tileset chunks
if header flags bit 2 (4, "layers have UUID"):
  UUID  layerUuid
```

Flags (WORD bitfield):

| Bit | Value | Meaning |
|---|---|---|
| 0 | 1 | Visible |
| 1 | 2 | Editable |
| 2 | 4 | Lock movement |
| 3 | 8 | Background (bottom layer, opaque; blend/opacity fields are *not* applied to it) |
| 4 | 16 | Prefer linked cels |
| 5 | 32 | Group displayed collapsed (UI only) |
| 6 | 64 | Reference layer |

**Hierarchy (child level):** each layer's `childLevel` relates it to the previously read layer:
- `childLevel == prevLevel` → sibling of previous layer (same parent)
- `childLevel == prevLevel + 1` → child of previous layer (previous must be a group)
- `childLevel < prevLevel` → walk up `prevLevel − childLevel` parents from previous layer's parent, add there

Example from the spec:

```
Layer                    childLevel   layerIndex
- Background                0            0
  `- Layer1                 1            1
- Foreground                0            2
  |- My set1                1            3
  |  `- Layer2              2            4
  `- Layer3                 1            5
```

**Visibility for rendering:** a layer renders only if it and *all ancestors* are visible.

**Blend/opacity applicability (spec NOTE.6):** blend mode & opacity are always semantically present for image/tilemap layers (opacity only trusted with header flag bit 0). For **group** layers they are only meaningful when header flag bit 1 (`2`) is set — in that case the group must be composited to its own buffer first, then blended onto the backdrop with its blend mode/opacity. Background layers never use them.

### 6.3 Cel chunk `0x2005`

```
WORD    layerIndex    ; index per §6.2 numbering
SHORT   x             ; may be negative; cel may extend past canvas
SHORT   y
BYTE    opacity       ; 0-255
WORD    celType       ; 0=Raw, 1=Linked, 2=Compressed image, 3=Compressed tilemap
SHORT   zIndex        ; 0=default order, +N = show N layers later, -N = N layers back
BYTE[5] reserved
... type-specific payload
```

**Type 0 — Raw image (legacy; only in very old files):**
```
WORD width, WORD height
PIXEL[width*height]   ; rows top→bottom, pixels left→right, PIXEL per color depth
```

**Type 1 — Linked cel:**
```
WORD framePosition    ; frame index that holds the real cel (same layer)
```
Resolution: find the cel of the *same layer* at `framePosition` and share its image. Notes: the target is always an earlier frame; Aseprite normalizes chains so links point at the original, but a defensive parser should resolve recursively. Gotcha: an old beta allowed a link with different x/y/opacity than its target — Aseprite honors the link chunk's own x/y/opacity if they differ (makes a copy). Use the link chunk's x/y/opacity/z-index, only the *image* comes from the target.

**Type 2 — Compressed image (the normal case):**
```
WORD width, WORD height
BYTE[...]             ; one zlib stream (RFC 1950 wrapper + RFC 1951 deflate)
```
Decompressed bytes are exactly the Raw layout (width×height×bytesPerPixel, row-major, top-down). The compressed data runs from the current position to `chunk_start + chunk_size` — bound your inflate by chunk end. Expected decompressed size = `width * height * bpp/8`.

**Type 3 — Compressed tilemap:**
```
WORD    widthInTiles
WORD    heightInTiles
WORD    bitsPerTile      ; always 32 currently; Aseprite rejects anything else
DWORD   tileIdMask       ; e.g. 0x1FFFFFFF
DWORD   xFlipMask        ; Aseprite writes 0x80000000
DWORD   yFlipMask        ; Aseprite writes 0x40000000
DWORD   dFlipMask        ; diagonal flip (swap X/Y axes); Aseprite writes 0x20000000
BYTE[10] reserved
BYTE[...]                ; zlib stream of TILE[w*h], row-major top-down
```
Decoding each 32-bit tile `t`:
```
tileIndex = (t & tileIdMask) >> trailing_zero_count(tileIdMask)
xflip = (t & xFlipMask) == xFlipMask
yflip = (t & yFlipMask) == yFlipMask
dflip = (t & dFlipMask) == dFlipMask     ; apply as: transpose, then x/y flips (D, then X, then Y)
```
Always use the masks stored in the file, not hard-coded constants (old builds used different bit meanings — one beta had a "90° CW rotation" bit where dflip now lives). Tile index 0 = empty tile **if** the owning tileset has flag `4` (ZERO_IS_NOTILE — all release-version files do); in rare internal-build files without that flag, empty = `0xFFFFFFFF` and Aseprite remaps indices +1 on load.

If `width == 0 || height == 0` for any image cel type, Aseprite creates no cel — treat as absent.

### 6.4 Cel Extra chunk `0x2006`

Applies to the **most recently read cel**.

```
DWORD   flags        ; bit 0 (1) = precise bounds are set
FIXED   preciseX
FIXED   preciseY
FIXED   scaledWidth  ; cel size in the sprite (real-time scaling, used by reference layers)
FIXED   scaledHeight
BYTE[16] reserved
```
Aseprite ignores the bounds when w or h is 0.

### 6.5 Color Profile chunk `0x2007`

```
WORD    type       ; 0 = none (old files), 1 = sRGB, 2 = embedded ICC
WORD    flags      ; bit 0 (1) = use special fixed gamma
FIXED   gamma      ; 1.0 = linear. sRGB + fixed gamma 1.0 means Linear sRGB.
BYTE[8] reserved
if type == 2:
  DWORD  iccLength
  BYTE[iccLength]  ; ICC profile data (ICC.1 spec)
```
Semantics per Aseprite: type 0 + gamma flag → "sRGB with gamma"; type 0 without flag → no color space; type 1 → sRGB (optionally with fixed gamma); type 2 → ICC. Note sRGB's overall ~2.2 gamma is *not* what the fixed-gamma field expresses (sRGB has piecewise linear/power sections). For game use, treating everything as sRGB is nearly always fine.

### 6.6 External Files chunk `0x2008` (first frame)

```
DWORD   nEntries
BYTE[8] reserved
for each entry:
  DWORD   entryId      ; referenced by tilesets / properties maps
  BYTE    type         ; 0=external palette, 1=external tileset,
                       ; 2=extension name for properties, 3=extension name for tile management
                       ;   (max one type-3 per sprite)
  BYTE[7] reserved
  STRING  fileNameOrExtensionId   ; extension IDs look like "publisher/ExtensionName"
```

### 6.7 Mask chunk `0x2016` — DEPRECATED

```
SHORT x, SHORT y
WORD width, WORD height
BYTE[8] reserved
STRING name
BYTE[height * ((width+7)/8)]   ; 1bpp bitmap, MSB = leftmost pixel of each byte
```

### 6.8 Path chunk `0x2017` — never used; skip.

### 6.9 Tags chunk `0x2018`

```
WORD    nTags
BYTE[8] reserved
for each tag:
  WORD    fromFrame          ; inclusive, 0-based
  WORD    toFrame            ; inclusive
  BYTE    loopDirection      ; 0=Forward, 1=Reverse, 2=Ping-pong, 3=Ping-pong Reverse
                             ; (Aseprite treats any other value as Forward)
  WORD    repeat             ; 0 = unspecified (∞ in UI, once on export; ping-pong: once each direction)
                             ; 1 = once (ping-pong: one direction only)
                             ; 2 = twice (ping-pong: there and back)
                             ; N = N times
  BYTE[6] reserved
  BYTE[3] tagColorRGB        ; DEPRECATED (v1.2.x compat only) — real color lives in the
                             ; tag's User Data chunk. Read it as a fallback for old files.
  BYTE    extra              ; zero
  STRING  name
```
The `repeat` field occupies bytes that were reserved in v1.2 — old files have 0 there, which conveniently means "unspecified".

After the Tags chunk come **one User Data chunk per tag, in tag order** (see §6.11). Files from ≤v1.2 have none — then use the in-chunk RGB.

### 6.10 Palette chunk `0x2019`

```
DWORD   newPaletteSize      ; total entries after this chunk (can be > 256!)
DWORD   fromIndex           ; first index changed
DWORD   toIndex             ; last index changed (inclusive) → (to-from+1) entries follow
BYTE[8] reserved
for each entry in [from..to]:
  WORD  entryFlags          ; bit 0 (1) = has name
  BYTE  r, g, b, a          ; 0-255 each, straight (non-premultiplied) alpha
  if entryFlags & 1:
    STRING colorName        ; Aseprite itself ignores names on load
```
The palette is a delta against the previous palette state (per-frame palettes are possible: a palette chunk in frame N changes the palette from frame N onward — copy the previous frame's palette and apply the range). Palettes can exceed 256 entries in RGB/grayscale sprites; indexed pixels are single bytes so only 0–255 are addressable.

### 6.11 User Data chunk `0x2020`

Attaches to the **last read chunk/object**:

- After a Layer chunk → that layer. After a Cel chunk → that cel. After a Slice chunk → that slice.
- **Tags special case:** after a Tags chunk with N tags, the next N user data chunks belong to tags 1..N in order.
- **Tileset special case:** a Tileset chunk may be followed by one user data chunk for the tileset itself, then (if the file has per-tile data) exactly `numTiles` more user data chunks, one per tile index 0..n−1. Aseprite reads those greedily right after the tileset's own user data (peeking each chunk header; if a non-user-data chunk appears, it stops). These per-tile chunks are counted in the frame's chunk count.
- **Sprite user data (v1.3+):** a user data chunk in the first frame appearing *before any layer/cel/tag/slice/tileset chunk* (i.e., right after the Palette/Color Profile chunks) belongs to the sprite itself. (Implementation detail: Aseprite initializes "last object" = sprite; palette, old-palette, and color-profile chunks do not change it.)
- If the preceding chunk failed to parse/was null, the user data is dropped (pointer set to null).

```
DWORD flags        ; 1=has text, 2=has color, 4=has properties
if flags & 1: STRING text
if flags & 2: BYTE r, g, b, a
if flags & 4:
  DWORD totalSize        ; bytes of the whole properties blob INCLUDING this DWORD and the next (≥8)
  DWORD nMaps
  for each map:
    DWORD mapKey         ; 0 = user properties; else an External Files entry ID (extension)
    <PROPERTIES payload, see below with type 0x0012 semantics>
```
Aseprite seeks to `propsStart + totalSize` afterward regardless of parse success — do the same to survive unknown property types.

Each **properties map** payload (also the payload of nested type `0x0012`):
```
DWORD nProperties
for each property:
  STRING name
  WORD   type
  <value per type>
```

Property value types:

| Type | Value encoding |
|---|---|
| `0x0000` | nullptr — must not appear in files |
| `0x0001` bool | `BYTE` (0=false, nonzero=true) |
| `0x0002` int8 | `BYTE` (signed) |
| `0x0003` uint8 | `BYTE` |
| `0x0004` int16 | `SHORT` |
| `0x0005` uint16 | `WORD` |
| `0x0006` int32 | `LONG` |
| `0x0007` uint32 | `DWORD` |
| `0x0008` int64 | `LONG64` |
| `0x0009` uint64 | `QWORD` |
| `0x000A` fixed | `FIXED` (16.16) |
| `0x000B` float | `FLOAT` |
| `0x000C` double | `DOUBLE` |
| `0x000D` string | `STRING` |
| `0x000E` point | `POINT` (2×LONG) |
| `0x000F` size | `SIZE` (2×LONG) |
| `0x0010` rect | `RECT` (4×LONG) |
| `0x0011` vector | `DWORD` nElems, `WORD` elemsType; if elemsType==0 each element is `WORD type` + value (heterogeneous); else nElems values of elemsType back-to-back |
| `0x0012` properties (nested map) | `DWORD` nProperties + properties as above |
| `0x0013` uuid | `UUID` (16 bytes) |

Nesting limit: 128 levels (Aseprite throws beyond that). Unknown type → abort properties parsing and seek to `propsStart + totalSize`.

### 6.12 Slice chunk `0x2022`

```
DWORD   nKeys
DWORD   flags        ; 1 = 9-patch slice, 2 = has pivot
DWORD   reserved
STRING  name
for each key:
  DWORD frameNumber  ; key is valid from this frame to end of animation (or next key)
  LONG  x            ; slice origin in sprite coords (can be negative)
  LONG  y
  DWORD width        ; 0 = slice hidden from this frame on
  DWORD height
  if flags & 1:      ; 9-patch center rect, RELATIVE TO SLICE BOUNDS
    LONG centerX, LONG centerY
    DWORD centerWidth, DWORD centerHeight
  if flags & 2:      ; pivot, RELATIVE TO SLICE ORIGIN
    LONG pivotX, LONG pivotY
```
Aseprite's "no pivot" sentinel is an implementation detail; presence is governed purely by flag bit 1 (value 2).

### 6.13 Tileset chunk `0x2023`

```
DWORD   tilesetId          ; the index referenced by tilemap layers
DWORD   flags
DWORD   numTiles
WORD    tileWidth
WORD    tileHeight
SHORT   baseIndex          ; UI-only: number shown for tile index 1 (default 1; 0 = zero-based display).
                           ; Does NOT affect stored data.
BYTE[14] reserved
STRING  name
if flags & 1:              ; external file link
  DWORD externalFileId     ; entry in External Files chunk
  DWORD externalTilesetId  ; tileset ID inside that external file
if flags & 2:              ; embedded tiles
  DWORD dataLength         ; length of the compressed blob that follows
  BYTE[dataLength]         ; zlib stream → image of size tileWidth × (tileHeight * numTiles)
                           ; i.e. a vertical strip, tile i at rows [i*tileHeight, (i+1)*tileHeight)
                           ; pixels in the sprite's color depth
```

Flags:

| Bit | Value | Meaning |
|---|---|---|
| 0 | 1 | Includes link to external file |
| 1 | 2 | Includes tiles embedded in this file |
| 2 | 4 | Tile ID 0 = empty tile (**the new/normal format**). If clear (rare internal builds), empty = `0xFFFFFFFF` and the embedded strip has no empty tile at index 0 — remap old indices by +1. |
| 3 | 8 | Auto mode tries to match modified tiles with X-flipped versions |
| 4 | 16 | Same for Y flips |
| 5 | 32 | Same for D flips |

Tile 0 in a normal (flag-4) tileset is the empty tile and its pixels in the strip are blank. `numTiles` includes it. Validation: Aseprite rejects tilesets with `tileWidth < 1 || tileHeight < 1`.

## 7. Color depth / pixel storage

| Mode | bpp | PIXEL bytes | Notes |
|---|---|---|---|
| RGBA | 32 | R, G, B, A | **Straight (non-premultiplied) alpha** |
| Grayscale | 16 | Value, Alpha | byte 0 = value (0=black, 255=white), byte 1 = alpha |
| Indexed | 8 | palette index | header's transparent index = transparent on non-background layers; drawn as its palette color on a background layer |

Indexed rendering rules:
- `index == header.transparent_index` → skip pixel (transparent), *except* on a background layer where every index is opaque paint.
- Convert other indices via the current frame's palette; the entry alpha (from 0x2019) applies. Out-of-range index → treat as transparent (Aseprite ignores src indices ≥ palette size when compositing indexed→indexed).
- The transparent index can be non-zero.

Grayscale gotcha: grayscale files may still contain palette chunks (a gray ramp for UI); ignore them for pixel decoding — the pixel bytes are the value/alpha directly.

## 8. Rendering pipeline summary

For each frame, composite visible layers bottom-to-top (layer index order = file order, adjusted by z-index):

1. Compute cel draw order: for each cel, `order = layerIndex + zIndex`; sort ascending, ties broken by smaller `zIndex` first (back-to-front). (Spec NOTE.5.)
2. Effective per-cel opacity: `opacity = MUL_UN8(cel.opacity, layer.opacity)` (both 0–255; layer opacity = 255 if header flag bit 0 unset; background layer ignores both, effectively SRC).
3. Convert pixel to RGBA (via palette for indexed, replicate value for grayscale), skip transparent pixels.
4. Blend onto backdrop with the layer's blend mode (formulas below).
5. Groups: if header flag bit 1 (`2`) set, composite each group's children into a scratch buffer, then blend that buffer with the group's blend mode/opacity; otherwise groups are pure pass-through folders.

## 9. Blend modes

### 9.1 Integer helper macros (from pixman, exact Aseprite math)

```c
// t is a uint32 temp; results are 0..255
#define MUL_UN8(a, b, t)  ((t) = (a) * (uint16_t)(b) + 0x80, \
                           ((((t) >> 8) + (t)) >> 8))          // round(a*b/255)
#define DIV_UN8(a, b)     (((uint16_t)(a) * 0xFF + ((b) / 2)) / (b))  // round(a*255/b)
```

### 9.2 Alpha compositing (the "normal" blender — used as final step by ALL modes)

`rgba_blender_normal(backdrop B, source S, opacity)` — straight-alpha "over":

```
Sa = MUL_UN8(S.a, opacity)
if B.a == 0: result = (S.rgb, Sa)
if Sa  == 0: result = B
Ra = Sa + B.a - MUL_UN8(B.a, Sa)          // Sa + Ba*(1-Sa)
Rc = Bc + (Sc - Bc) * Sa / Ra             // per channel, integer division
```

Every non-normal mode works by computing a per-channel blended color `blend(Bc, Sc)`, substituting it for the source RGB (keeping source alpha), then running the normal blender. Grayscale uses identical math on the single value channel; HSL modes and tints have no grayscale variant (grayscale falls back to normal for HSL modes).

### 9.3 Mode table (Layer chunk `blendMode` WORD) and formulas

`b`,`s` are channel values; integer formulas use the macros above; normalized formulas in [0,1] shown where clearer. These match W3C Compositing & Adobe PDF blend definitions.

| ID | Mode | Per-channel formula |
|---|---|---|
| 0 | Normal | (source used as-is; just alpha compositing) |
| 1 | Multiply | `MUL_UN8(b, s)` |
| 2 | Screen | `b + s − MUL_UN8(b, s)` |
| 3 | Overlay | `hardlight(s, b)` (hard light with args swapped) |
| 4 | Darken | `min(b, s)` |
| 5 | Lighten | `max(b, s)` |
| 6 | Color Dodge | `b==0 → 0; s'=255−s; b≥s' → 255; else DIV_UN8(b, s')` |
| 7 | Color Burn | `b==255 → 255; b'=255−b; b'≥s → 0; else 255 − DIV_UN8(b', s)` |
| 8 | Hard Light | `s < 128 → multiply(b, 2s); else screen(b, 2s−255)` |
| 9 | Soft Light | float, per W3C: `b≤0.25 → d=((16b−12)b+4)b else d=√b`; `s≤0.5 → r=b−(1−2s)·b·(1−b) else r=b+(2s−1)(d−b)`; result `round(r·255)` |
| 10 | Difference | `abs(b − s)` |
| 11 | Exclusion | `b + s − 2·MUL_UN8(b, s)` |
| 12 | Hue | non-separable: `S' = setLum(setSat(S_rgb, sat(B_rgb)), lum(B_rgb))` |
| 13 | Saturation | `S' = setLum(setSat(B_rgb, sat(S_rgb)), lum(B_rgb))` |
| 14 | Color | `S' = setLum(S_rgb, lum(B_rgb))` |
| 15 | Luminosity | `S' = setLum(B_rgb, lum(S_rgb))` |
| 16 | Addition | `min(b + s, 255)` |
| 17 | Subtract | `max(b − s, 0)` |
| 18 | Divide | `b==0 → 0; b≥s → 255; else DIV_UN8(b, s)` |

Non-separable helpers (doubles, channels normalized to [0,1]):
```
lum(r,g,b) = 0.3r + 0.59g + 0.11b
sat(r,g,b) = max(r,g,b) − min(r,g,b)
setSat: min-channel→0, max→s, mid→((mid−min)·s)/(max−min); all 0 if max==min
setLum(r,g,b,l): d = l − lum(r,g,b); add d to each channel; then clipColor:
  clipColor: l=lum; n=min; x=max
    if n<0: c = l + (c−l)·l/(l−n)      for each channel
    if x>1: c = l + (c−l)·(1−l)/(x−l)
```

### 9.4 Old vs new blend method ("newBlend")

The file does **not** record which compositing method to use. Legacy Aseprite (≤v1.2.x default) computed `blend(Bc,Sc)` even where the backdrop is transparent, producing artifacts. Modern Aseprite uses "new" blenders (`rgba_blender_*_n`) that, when the backdrop has alpha:

```
normal  = normal_blend(B, S, opacity)
blend   = legacy_mode_blend(B, S, opacity)
m1      = merge(normal, blend, B.a)                       // lerp by backdrop alpha
srcA    = MUL_UN8(S.a, opacity)
compA   = MUL_UN8(B.a, srcA)
result  = merge(m1, blend, compA)
// where merge(x, y, t) lerps each channel: x + (y-x)*t/255 (with special cases for 0 alpha)
```
and fall back to plain normal blending when backdrop alpha is 0. For pixel-exact parity with current Aseprite, implement the `_n` variants; legacy behavior matches old exports.

## 10. Version history & compatibility

- **Original .ase (Allegro Sprite Editor era):** same magic `0xA5E0`, same 128-byte FLC-style header. Earliest change: per-frame duration field added to the frame header — old files have 0 there, so seed every frame's duration from the header `speed` WORD and override when a frame's duration > 0.
- **v1.1:** writes both old palette (0x0004) and new palette (0x2019) chunks.
- **v1.2:** stable chunk set: layers (normal/group), cels raw(0)/linked(1)/compressed(2), tags (color stored in-chunk), slices, user data (text+color only), masks deprecated.
- **v1.2.9+:** Color Profile chunk (0x2007) added; older files have none (type 0 semantics).
- **v1.2-beta7..beta8 dev builds only:** chunk 0x2021 ("Slices") — deprecated, skip.
- **v1.3:** tilemaps (layer type 2 + tileset index field, cel type 3, Tileset chunk 0x2023, TILE type), External Files chunk 0x2008, tag `repeat` WORD (carved out of reserved bytes), tag colors moved to per-tag User Data chunks (in-chunk RGB deprecated), sprite-level user data, user data `properties` (flag 4) with typed variant maps, per-tile user data after tileset user data, cel z-index SHORT (carved from reserved bytes — old files have 0), 9-patch/pivot already existed in slices.
- **v1.3.5+:** old palette chunk written *instead of* 0x2019 when palette ≤ 256 entries and fully opaque.
- **Recent v1.3.x/1.4:** header flag bit 1 (`2`) = group blend/opacity valid ("composite groups"), header flag bit 2 (`4`) = per-layer UUIDs appended to layer chunks.
- **There is no 0xA5E1 magic.** `.ase` vs `.aseprite` is extension-only. Frame magic is always `0xF1FA`.

## 11. Parser gotchas checklist

1. **Always seek by declared sizes** (chunk size incl. 6-byte header; frame size; header to offset 128; properties blob to `start + totalSize`). Newer versions append fields; trailing unread bytes are normal.
2. **zlib, not raw deflate:** cel/tileset image data has the 2-byte zlib header (`0x78 ...`) and adler32 trailer. Bound inflation by chunk end (or `dataLength` for tilesets). One independent stream per cel/tileset.
3. **Chunk count:** old WORD `0xFFFF` → use the new DWORD (if nonzero).
4. **Layer opacity valid flag:** if header flags bit 0 is clear, ignore stored layer opacity (use 255). Nearly all modern files set it.
5. **Layer index includes groups** — count every 0x2004 chunk. A cel pointing at a group/missing layer is invalid (skip it; also treat any following user data chunk as orphaned).
6. **Background layer:** ignores blend mode, layer & cel opacity for compositing purposes; indexed transparent index paints opaque on it.
7. **Transparent index:** only meaningful at depth 8; zero it otherwise. Can be non-zero.
8. **`ncolors == 0` means 256.** Pixel ratio: 0 in either byte → 1:1.
9. **Palette > 256 entries** possible in 0x2019 (`newPaletteSize` is a DWORD); palettes are deltas and can change per frame.
10. **Old palette chunks:** ignore once a 0x2019 has been seen; scale 0x0011 components via `(v<<2)|(v>>4)`.
11. **Strings are UTF-8, length-prefixed, not NUL-terminated.**
12. **Straight alpha everywhere** (pixels, palette entries, user-data colors). Premultiply yourself if your engine needs it.
13. **Linked cels:** honor the link chunk's own x/y/opacity/z-index (beta files may differ from target); resolve image from same layer at the linked frame; resolve chains defensively.
14. **Cels can have negative positions and extend beyond canvas** — clip when compositing. `w==0||h==0` → no cel.
15. **Tilemap masks are data-driven** — read shift from `tileIdMask` trailing zeros; don't hardcode `0x1FFFFFFF/0x80000000/0x40000000/0x20000000` even though that's what current Aseprite writes. Reject `bitsPerTile != 32` (nothing else was ever shipped).
16. **Empty tile:** ID 0 when tileset flag 4 set (always, in practice). Tileset strip is vertical: tile i at row `i*tileHeight`.
17. **User data association order** is stateful and quirky (tags: N chunks after tags chunk; tileset: tileset UD then per-tile UDs, which still count as frame chunks; sprite UD: first-frame UD before any other UD-able object; unknown chunks don't reset the association).
18. **Tag loop direction** out of range → Forward. Tag color: prefer user-data color, fall back to deprecated in-chunk RGB for v1.2 files.
19. **Frame durations:** default from header `speed`, override with frame header duration when > 0.
20. **Grayscale value is byte 0, alpha byte 1** (as a LE WORD: `v = w & 0xFF`, `a = w >> 8`).
21. **Z-index:** sort cels by `layerIndex + zIndex`, tie-break by `zIndex` (see §8) — files from ≤v1.2 always have 0 there.
22. **Property maps:** guard recursion (128 levels max), and on any unknown property type bail out by seeking to the end of the properties blob (its total size is stored precisely so readers can do this).
23. **Aseprite tolerates and skips unknown chunk types** — you should too, for forward compatibility.
