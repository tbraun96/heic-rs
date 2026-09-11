//! The seam between the HEIF container and the HEVC still-picture decoder.
//!
//! Everything above this module — box parsing, item resolution, grid
//! compositing, transforms, colour conversion — is complete and tested
//! without a working codec, because the codec is reached only through the two
//! functions below.
//!
//! # Status
//!
//! The real decoder is being written as a standalone crate and lands next, as
//! the body of this module. Until then [`decode_still`] and [`probe`] return
//! [`Error::Unsupported`], and `heic_rs::decode` therefore fails at the seam
//! rather than silently returning wrong pixels. `heic_rs::probe` does not go
//! through here at all when the file carries `ispe`, which every conformant
//! HEIF file does, so probing works today.
//!
//! # Contract
//!
//! `parameter_sets` are the VPS, SPS and PPS NAL units taken from the item's
//! `hvcC` property, in that order, each without a length prefix or start code.
//! `slices` are the remaining NAL units of the item's data, already split on
//! their length prefixes. Both borrow from the caller; the decoder must not
//! assume they outlive the call.

use alloc::vec::Vec;

use crate::error::{Error, Result};

/// The message returned by every entry point until the decoder lands.
const NOT_LINKED: &str = "the HEVC decoder is not linked in this build";

/// How the chroma planes are sampled relative to luma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromaFormat {
    /// No chroma planes at all.
    Monochrome,
    /// Chroma at half resolution in both directions.
    #[default]
    Yuv420,
    /// Chroma at half resolution horizontally only.
    Yuv422,
    /// Chroma at full resolution.
    Yuv444,
}

impl ChromaFormat {
    /// Map an HEVC `chroma_format_idc` onto this enum.
    pub const fn from_idc(idc: u8) -> Option<ChromaFormat> {
        Some(match idc {
            0 => ChromaFormat::Monochrome,
            1 => ChromaFormat::Yuv420,
            2 => ChromaFormat::Yuv422,
            3 => ChromaFormat::Yuv444,
            _ => return None,
        })
    }

    /// How much to shift a luma x coordinate to reach the chroma plane.
    pub const fn x_shift(self) -> u32 {
        match self {
            ChromaFormat::Yuv420 | ChromaFormat::Yuv422 => 1,
            _ => 0,
        }
    }

    /// How much to shift a luma y coordinate to reach the chroma plane.
    pub const fn y_shift(self) -> u32 {
        match self {
            ChromaFormat::Yuv420 => 1,
            _ => 0,
        }
    }

    /// The size of a chroma plane for a picture of the given luma size.
    pub const fn chroma_size(self, width: u32, height: u32) -> (u32, u32) {
        if matches!(self, ChromaFormat::Monochrome) {
            return (0, 0);
        }
        (
            width.div_ceil(1 << self.x_shift()),
            height.div_ceil(1 << self.y_shift()),
        )
    }
}

/// One decoded picture, in planar YCbCr, one sample per `u16` regardless of
/// bit depth so that 8-bit and 10-bit content share a representation.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// Luma width in samples.
    pub width: u32,
    /// Luma height in samples.
    pub height: u32,
    /// Bits actually used in each sample, 8 through 12.
    pub bit_depth: u8,
    /// Chroma sampling.
    pub chroma: ChromaFormat,
    /// Luma plane, `y_stride` samples per row.
    pub y: Vec<u16>,
    /// Cb plane, `c_stride` samples per row. Empty when monochrome.
    pub cb: Vec<u16>,
    /// Cr plane, `c_stride` samples per row. Empty when monochrome.
    pub cr: Vec<u16>,
    /// Samples per row of the luma plane, at least `width`.
    pub y_stride: u32,
    /// Samples per row of each chroma plane.
    pub c_stride: u32,
}

impl Frame {
    /// Check that the planes are as large as the declared geometry needs.
    ///
    /// The compositor calls this on everything it is handed, so that a buggy
    /// or malicious decoder cannot make the rest of the crate index past the
    /// end of a plane.
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(Error::Malformed("decoded frame has a zero dimension"));
        }
        if self.y_stride < self.width {
            return Err(Error::Malformed("luma stride is narrower than the frame"));
        }
        let need = (self.y_stride as usize).saturating_mul(self.height as usize);
        if self.y.len() < need {
            return Err(Error::Malformed("luma plane is shorter than its geometry"));
        }
        if self.chroma == ChromaFormat::Monochrome {
            return Ok(());
        }
        let (cw, ch) = self.chroma.chroma_size(self.width, self.height);
        if self.c_stride < cw {
            return Err(Error::Malformed("chroma stride is narrower than the frame"));
        }
        let need = (self.c_stride as usize).saturating_mul(ch as usize);
        if self.cb.len() < need || self.cr.len() < need {
            return Err(Error::Malformed(
                "chroma plane is shorter than its geometry",
            ));
        }
        Ok(())
    }
}

/// What a parameter set says about the pictures that follow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    /// Luma width in samples.
    pub width: u32,
    /// Luma height in samples.
    pub height: u32,
    /// Luma bit depth.
    pub bit_depth: u8,
    /// Chroma sampling.
    pub chroma: ChromaFormat,
}

/// Decode a single still picture.
///
/// See the module documentation for the shape of the inputs.
pub fn decode_still(parameter_sets: &[&[u8]], slices: &[&[u8]]) -> Result<Frame> {
    let _ = (parameter_sets, slices);
    Err(Error::Unsupported(NOT_LINKED))
}

/// Read the geometry out of a sequence parameter set without decoding.
pub fn probe(parameter_sets: &[&[u8]]) -> Result<Info> {
    let _ = parameter_sets;
    Err(Error::Unsupported(NOT_LINKED))
}

/// True when `e` is the placeholder's own refusal, as opposed to a real
/// failure inside a linked decoder. Used by the tests that assert the seam is
/// reached, so that they keep passing once the decoder lands.
pub fn is_not_linked(e: &Error) -> bool {
    matches!(e, Error::Unsupported(m) if *m == NOT_LINKED)
}
