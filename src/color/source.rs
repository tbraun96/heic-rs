//! Where a conversion reads its rows from: one decoded frame, or a grid's
//! tiles.
//!
//! The kernel asks for "luma row `y`" and "chroma rows `r0` and `r1`" and
//! nothing else, so a grid never has to be composed onto a canvas for the
//! colour pass: its rows are assembled into a band's scratch buffer as they
//! are needed, from the tiles that hold them. A frame answers the same
//! questions with slices of its planes and uses no scratch at all.

use crate::grid::Mosaic;
use crate::hevc::{ChromaFormat, Frame};

/// The planes a conversion reads.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Source<'a> {
    /// One decoded picture.
    Frame(&'a Frame),
    /// The tiles of a grid, read in place.
    Mosaic(&'a Mosaic<'a>),
}

/// Two chroma rows of both planes: Cb row `r0`, Cb row `r1`, Cr row `r0`,
/// Cr row `r1`.
pub(crate) type ChromaRows<'b> = [&'b [u16]; 4];

impl Source<'_> {
    /// Width in luma samples.
    pub fn width(&self) -> u32 {
        match self {
            Source::Frame(f) => f.width,
            Source::Mosaic(m) => m.width(),
        }
    }

    /// Height in luma samples.
    pub fn height(&self) -> u32 {
        match self {
            Source::Frame(f) => f.height,
            Source::Mosaic(m) => m.height(),
        }
    }

    /// Chroma sampling.
    pub fn chroma(&self) -> ChromaFormat {
        match self {
            Source::Frame(f) => f.chroma,
            Source::Mosaic(m) => m.chroma(),
        }
    }

    /// Sample bit depth.
    pub fn bit_depth(&self) -> u8 {
        match self {
            Source::Frame(f) => f.bit_depth,
            Source::Mosaic(m) => m.bit_depth(),
        }
    }

    /// Width of one chroma row.
    fn chroma_width(&self) -> usize {
        self.chroma().chroma_size(self.width(), self.height()).0 as usize
    }

    /// Scratch samples a band needs to assemble rows: none for a frame; one
    /// luma row and four chroma rows for a mosaic.
    pub fn stitch_len(&self) -> usize {
        match self {
            Source::Frame(_) => 0,
            Source::Mosaic(_) => self.width() as usize + 4 * self.chroma_width(),
        }
    }

    /// Luma row `y`, assembled into `buf` when the source is a mosaic.
    pub fn luma<'b>(&'b self, y: usize, buf: &'b mut [u16]) -> Option<&'b [u16]> {
        match self {
            Source::Frame(f) => {
                let (w, stride) = (f.width as usize, f.y_stride as usize);
                let start = y.checked_mul(stride)?;
                f.y.get(start..start + w)
            }
            Source::Mosaic(m) => {
                let row = buf.get_mut(..m.width() as usize)?;
                m.luma_row(y, row)?;
                Some(row)
            }
        }
    }

    /// Chroma rows `r0` and `r1` of both planes, assembled into `buf` when
    /// the source is a mosaic.
    pub fn chroma_rows<'b>(
        &'b self,
        r0: usize,
        r1: usize,
        buf: &'b mut [u16],
    ) -> Option<ChromaRows<'b>> {
        let cw = self.chroma_width();
        match self {
            Source::Frame(f) => {
                let stride = f.c_stride as usize;
                let (o0, o1) = (r0.checked_mul(stride)?, r1.checked_mul(stride)?);
                Some([
                    f.cb.get(o0..o0 + cw)?,
                    f.cb.get(o1..o1 + cw)?,
                    f.cr.get(o0..o0 + cw)?,
                    f.cr.get(o1..o1 + cw)?,
                ])
            }
            Source::Mosaic(m) => {
                let (b0, rest) = buf.split_at_mut_checked(cw)?;
                let (b1, rest) = rest.split_at_mut_checked(cw)?;
                let (v0, rest) = rest.split_at_mut_checked(cw)?;
                let v1 = rest.get_mut(..cw)?;
                m.chroma_row(false, r0, b0)?;
                m.chroma_row(false, r1, b1)?;
                m.chroma_row(true, r0, v0)?;
                m.chroma_row(true, r1, v1)?;
                Some([b0, b1, v0, v1])
            }
        }
    }
}
