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
    /// Carries the index into [`AseFile::tilesets`](crate::AseFile::tilesets).
    Tilemap {
        tileset_index: u32,
    },
}

/// One layer (§6.2). Layers are stored flat in file order — the index in
/// [`AseFile::layers`](crate::AseFile::layers) is exactly the layer index cels reference (gotcha #5).
#[derive(Debug, Clone)]
pub struct Layer {
    pub flags: u16,
    pub layer_type: LayerType,
    pub child_level: u16,
    pub blend_mode: BlendMode,
    /// Already normalized: 255 when the header says layer opacity is invalid.
    pub opacity: u8,
    pub name: String,
    /// Index of the parent group in [`AseFile::layers`](crate::AseFile::layers), derived from
    /// child levels during parsing.
    pub parent: Option<usize>,
    pub uuid: Option<[u8; 16]>,
    pub user_data: UserData,
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
    pub extra: Option<CelExtra>,
    pub user_data: UserData,
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
    /// v1.3 files attach one user data chunk per tag (in order) right after
    /// the tags chunk; the authoritative tag color lives there (gotcha #18).
    pub user_data: UserData,
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
    pub user_data: UserData,
    /// Per-tile user data, indexed by tile ID; present only in files written
    /// with per-tile data (gotcha #17).
    pub tile_user_data: Vec<UserData>,
}

impl Tileset {
    /// Flag 4: tile ID 0 is the empty tile (set in all release-format files).
    pub fn zero_is_empty(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// RGBA palette entries, straight alpha (§6.10).
#[derive(Debug, Clone, Default)]
pub struct Palette {
    pub entries: Vec<[u8; 4]>,
}

/// A typed user-data property value (§6.11).
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    /// 16.16 fixed point, converted.
    Fixed(f64),
    F32(f32),
    F64(f64),
    Str(String),
    Point(i32, i32),
    Size(i32, i32),
    Rect(i32, i32, i32, i32),
    Vector(Vec<PropertyValue>),
    Map(Properties),
    Uuid([u8; 16]),
}

/// Ordered name→value pairs (order preserved for deterministic output).
pub type Properties = Vec<(String, PropertyValue)>;

/// One properties map inside a user data chunk. Key 0 = user properties;
/// other keys reference an External Files entry (extension data).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertiesMap {
    pub key: u32,
    pub properties: Properties,
}

/// User data attached to an object (§6.11).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserData {
    pub text: Option<String>,
    pub color: Option<[u8; 4]>,
    pub maps: Vec<PropertiesMap>,
}

impl UserData {
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.color.is_none() && self.maps.is_empty()
    }
}

/// Cel extra chunk 0x2006: precise subpixel bounds (§6.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CelExtra {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Color profile chunk 0x2007 (§6.5).
#[derive(Debug, Clone, PartialEq)]
pub enum ColorProfile {
    /// Old files: no profile recorded.
    None {
        fixed_gamma: Option<f64>,
    },
    Srgb {
        fixed_gamma: Option<f64>,
    },
    Icc {
        fixed_gamma: Option<f64>,
        data: Vec<u8>,
    },
}

/// External files chunk entry (§6.6).
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalFile {
    pub id: u32,
    /// 0=palette, 1=tileset, 2=properties extension, 3=tile-management extension.
    pub kind: u8,
    pub name: String,
}

/// One slice key: valid from `frame` until the next key (§6.12).
#[derive(Debug, Clone)]
pub struct SliceKey {
    pub frame: u32,
    /// Slice origin in sprite coords; can be negative.
    pub x: i32,
    pub y: i32,
    /// Zero width/height means the slice is hidden from this frame on.
    pub width: u32,
    pub height: u32,
    /// 9-patch center rect, relative to the slice bounds.
    pub center: Option<(i32, i32, u32, u32)>,
    /// Pivot, relative to the slice origin.
    pub pivot: Option<(i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct Slice {
    pub name: String,
    pub keys: Vec<SliceKey>,
    pub user_data: UserData,
}

impl Slice {
    /// The key in effect at `frame`, if the slice is defined there yet.
    pub fn key_for(&self, frame: u32) -> Option<&SliceKey> {
        self.keys.iter().rev().find(|k| k.frame <= frame)
    }
}

/// One frame: its duration and cels (§4, §6.3).
#[derive(Debug, Clone)]
pub struct Frame {
    /// Effective duration (frame value, or header default when 0).
    pub duration_ms: u16,
    pub cels: Vec<Cel>,
}
