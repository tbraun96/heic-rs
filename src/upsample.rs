//! Chroma upsampling.
//!
//! # Which filter
//!
//! Horizontally, chroma in HEVC is left-sited: chroma sample `i` sits on luma
//! column `2i`. Even output columns are therefore a copy, and odd ones are the
//! average of the two neighbours — plain bilinear.
//!
//! Vertically, 4:2:0 chroma sits halfway between two luma rows, so the two
//! rows a chroma row feeds are a 3:1 blend of it and its neighbour. 4:2:2
//! needs no vertical work at all, and 4:4:4 needs none in either direction.
//!
//! This is a clean, cheap, standards-sited bilinear. It is not a windowed-sinc
//! or edge-directed upsampler; if that matters for your use it is the obvious
//! place to improve, and the filter is isolated here so that it can be.

use alloc::vec::Vec;

use crate::hevc::ChromaFormat;

/// Expand one chroma plane to full luma resolution.
///
/// `cw` and `ch` are the chroma plane's dimensions, `stride` its row length in
/// samples, and `w`/`h` the luma dimensions to produce.
pub fn plane(
    src: &[u16],
    stride: u32,
    cw: u32,
    ch: u32,
    w: u32,
    h: u32,
    chroma: ChromaFormat,
) -> Vec<u16> {
    let mut out = alloc::vec![0u16; w as usize * h as usize];
    if cw == 0 || ch == 0 {
        return out;
    }
    let x_shift = chroma.x_shift();
    let y_shift = chroma.y_shift();
    for y in 0..h as usize {
        // Pick the chroma row (or pair of rows) this luma row draws from.
        let (r0, r1, w0) = if y_shift == 0 {
            let r = core::cmp::min(y, ch as usize - 1);
            (r, r, 4u32)
        } else {
            let cy = y >> 1;
            let cy = core::cmp::min(cy, ch as usize - 1);
            let near = if y & 1 == 0 {
                cy.saturating_sub(1)
            } else {
                core::cmp::min(cy + 1, ch as usize - 1)
            };
            (cy, near, 3u32)
        };
        let base0 = r0 * stride as usize;
        let base1 = r1 * stride as usize;
        let dst = y * w as usize;
        for x in 0..w as usize {
            let (c0, c1, wx) = if x_shift == 0 {
                let c = core::cmp::min(x, cw as usize - 1);
                (c, c, 2u32)
            } else {
                let cx = core::cmp::min(x >> 1, cw as usize - 1);
                if x & 1 == 0 {
                    (cx, cx, 2u32)
                } else {
                    (cx, core::cmp::min(cx + 1, cw as usize - 1), 1u32)
                }
            };
            // Blend horizontally within each of the two chroma rows, then
            // blend the two results vertically, all in integer arithmetic.
            let a = blend(src.get(base0 + c0), src.get(base0 + c1), wx, 2);
            let b = blend(src.get(base1 + c0), src.get(base1 + c1), wx, 2);
            out[dst + x] = blend(Some(&a), Some(&b), w0, 4);
        }
    }
    out
}

/// `(v0 * weight + v1 * (total - weight) + total / 2) / total`, rounded.
///
/// A missing sample counts as zero; the callers clamp their indices first, so
/// that only happens on a plane shorter than its declared geometry, which
/// `Frame::validate` has already refused.
fn blend(v0: Option<&u16>, v1: Option<&u16>, weight: u32, total: u32) -> u16 {
    let a = v0.map_or(0u32, |v| u32::from(*v));
    let b = v1.map_or(0u32, |v| u32::from(*v));
    let n = a * weight + b * (total - weight) + total / 2;
    (n / total) as u16
}
