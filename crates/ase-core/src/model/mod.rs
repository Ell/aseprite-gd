//! The document model: what a parsed `.aseprite` file *is*, independent of how
//! it's encoded. Parsing (`crate::parse`) fills these in; compositing
//! (`crate::composite`) consumes them.

/// Color depth of the sprite (header §3). Determines the PIXEL encoding in
/// every cel of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 32bpp — R, G, B, A bytes, straight (non-premultiplied) alpha.
    Rgba,
    /// 16bpp — value byte then alpha byte (gotcha #20).
    Grayscale,
    /// 8bpp — palette index; header's transparent index applies (§7).
    Indexed,
}

impl ColorDepth {
    pub fn from_bpp(bpp: u16) -> Option<Self> {
        match bpp {
            32 => Some(ColorDepth::Rgba),
            16 => Some(ColorDepth::Grayscale),
            8 => Some(ColorDepth::Indexed),
            _ => None,
        }
    }

    pub fn bytes_per_pixel(self) -> usize {
        match self {
            ColorDepth::Rgba => 4,
            ColorDepth::Grayscale => 2,
            ColorDepth::Indexed => 1,
        }
    }
}

/// Parsed 128-byte file header (§3), with spec-mandated normalizations already
/// applied (see `parse::header`).
#[derive(Debug, Clone)]
pub struct Header {
    pub frames: u16,
    pub width: u16,
    pub height: u16,
    pub color_depth: ColorDepth,
    /// Raw header flags DWORD; use the `has_*` accessors.
    pub flags: u32,
    /// Deprecated global "speed": fallback duration for frames whose own
    /// duration field is 0 (gotcha #19).
    pub default_frame_duration_ms: u16,
    /// Transparent palette index. Normalized to 0 for non-indexed sprites,
    /// mirroring Aseprite's decoder (§3, gotcha #7).
    pub transparent_index: u8,
    /// Number of palette colors; 0 in old files means 256 (normalized here,
    /// gotcha #8).
    pub num_colors: u16,
    /// Pixel aspect ratio (width:height). Normalized to (1, 1) when either
    /// stored byte is 0 (gotcha #8).
    pub pixel_ratio: (u8, u8),
    pub grid_x: i16,
    pub grid_y: i16,
    /// 0 means "no grid".
    pub grid_width: u16,
    pub grid_height: u16,
}

impl Header {
    /// Bit 0: the layer chunks' opacity field is meaningful (gotcha #4).
    pub fn layer_opacity_valid(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Bit 1: group layers carry meaningful blend mode/opacity and must be
    /// composited into their own buffer (§6.2 NOTE.6).
    pub fn group_blend_valid(&self) -> bool {
        self.flags & 2 != 0
    }

    /// Bit 2: layer chunks are followed by a 16-byte UUID.
    pub fn layers_have_uuid(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// Parsed 16-byte frame header (§4), with the chunk-count overflow rule
/// already resolved.
#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    /// Total size of the frame in bytes, header included — used to seek to the
    /// next frame regardless of what was parsed in between.
    pub frame_bytes: u32,
    /// Resolved chunk count (old WORD / new DWORD rule, gotcha #3).
    pub num_chunks: u32,
    /// This frame's duration. 0 in old files — callers substitute
    /// `Header::default_frame_duration_ms` (gotcha #19).
    pub duration_ms: u16,
}

/// Layer blend mode (§9.3). Unknown values are preserved rather than dropped
/// so future modes degrade to Normal with a warning at composite time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    HslHue,
    HslSaturation,
    HslColor,
    HslLuminosity,
    Addition,
    Subtract,
    Divide,
    Unknown(u16),
}

impl BlendMode {
    pub fn from_u16(v: u16) -> Self {
        use BlendMode::*;
        match v {
            0 => Normal,
            1 => Multiply,
            2 => Screen,
            3 => Overlay,
            4 => Darken,
            5 => Lighten,
            6 => ColorDodge,
            7 => ColorBurn,
            8 => HardLight,
            9 => SoftLight,
            10 => Difference,
            11 => Exclusion,
            12 => HslHue,
            13 => HslSaturation,
            14 => HslColor,
            15 => HslLuminosity,
            16 => Addition,
            17 => Subtract,
            18 => Divide,
            other => Unknown(other),
        }
    }
}

/// Layer kind (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    Image,
    Group,
    /// Carries the index into [`AseFile::tilesets`].
    Tilemap { tileset_index: u32 },
}

/// One layer (§6.2). Layers are stored flat in file order — the index in
/// [`AseFile::layers`] is exactly the layer index cels reference (gotcha #5).
#[derive(Debug, Clone)]
pub struct Layer {
    pub flags: u16,
    pub layer_type: LayerType,
    pub child_level: u16,
    pub blend_mode: BlendMode,
    /// Already normalized: 255 when the header says layer opacity is invalid.
    pub opacity: u8,
    pub name: String,
    /// Index of the parent group in [`AseFile::layers`], derived from
    /// child levels during parsing.
    pub parent: Option<usize>,
    pub uuid: Option<[u8; 16]>,
}

impl Layer {
    pub fn is_visible(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn is_background(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn is_reference(&self) -> bool {
        self.flags & 64 != 0
    }
}

/// Decoded image data of a cel: raw pixels in the sprite's color depth,
/// row-major, top-down (§6.3).
#[derive(Debug, Clone)]
pub struct CelImage {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

/// Decoded tilemap grid of a cel (§6.3 type 3). Tiles are already
/// mask-decoded into index + flip flags.
#[derive(Debug, Clone)]
pub struct CelTilemap {
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<Tile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub index: u32,
    pub x_flip: bool,
    pub y_flip: bool,
    pub d_flip: bool,
}

#[derive(Debug, Clone)]
pub enum CelContent {
    Image(CelImage),
    /// Frame index holding the real image (same layer). Position/opacity of
    /// *this* cel still apply (gotcha #13).
    Linked(u16),
    Tilemap(CelTilemap),
}

/// One cel (§6.3).
#[derive(Debug, Clone)]
pub struct Cel {
    pub layer_index: usize,
    pub x: i16,
    pub y: i16,
    pub opacity: u8,
    pub z_index: i16,
    pub content: CelContent,
}

/// Tag loop direction (§6.9). Out-of-range values decode as Forward
/// (gotcha #18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AniDir {
    Forward,
    Reverse,
    PingPong,
    PingPongReverse,
}

#[derive(Debug, Clone)]
pub struct Tag {
    /// Inclusive frame range, 0-based.
    pub from_frame: u16,
    pub to_frame: u16,
    pub direction: AniDir,
    /// 0 = unspecified (infinite in UI); see §6.9 for ping-pong semantics.
    pub repeat: u16,
    /// Deprecated in-chunk RGB; the authoritative color arrives via the tag's
    /// user data chunk in v1.3 files (not yet parsed).
    pub color: [u8; 3],
    pub name: String,
}

/// One tileset (§6.13).
#[derive(Debug, Clone)]
pub struct Tileset {
    pub id: u32,
    pub flags: u32,
    pub num_tiles: u32,
    pub tile_width: u16,
    pub tile_height: u16,
    pub base_index: i16,
    pub name: String,
    /// Decoded vertical strip: tile i occupies rows
    /// `[i*tile_height, (i+1)*tile_height)`. Absent for external tilesets.
    pub pixels: Option<Vec<u8>>,
    pub external: Option<(u32, u32)>,
}

impl Tileset {
    /// Flag 4: tile ID 0 is the empty tile (set in all release-format files).
    pub fn zero_is_empty(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// RGBA palette entries, straight alpha (§6.10). Currently the state after
/// applying all palette chunks in file order; per-frame palette snapshots are
/// a TODO (only matters for indexed sprites with animated palettes).
#[derive(Debug, Clone, Default)]
pub struct Palette {
    pub entries: Vec<[u8; 4]>,
}

/// One frame: its duration and cels (§4, §6.3).
#[derive(Debug, Clone)]
pub struct Frame {
    /// Effective duration (frame value, or header default when 0).
    pub duration_ms: u16,
    pub cels: Vec<Cel>,
}
