//! The document model: what a parsed `.aseprite` file *is*, independent of how
//! it's encoded. Parsing (`crate::parse`) fills these in; compositing
//! (`crate::composite`, future) consumes them.

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
