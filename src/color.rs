//! Turning decoded YCbCr samples into the caller's chosen pixel layout.
//!
//! The matrix and the sample range both come from the file's `colr` property.
//! When a file carries no `colr` at all the conversion uses BT.709 limited
//! range, which is what unlabelled HEIC means in practice; that fallback is
//! [`Nclx::default`] and is chosen once, here, rather than being guessed at by
//! a parser.
//!
//! # How it is arranged
//!
//! The matrix is integer, not floating point: the private `fixed` module folds
//! the sample scaling, the matrix and the output maximum into five `i32`
//! coefficients, so a channel costs one multiply-accumulate, one rounded shift
//! and one clamp. The chroma upsampler is fused into the walk: a conversion
//! holds two upsampled chroma *rows*, never two chroma planes. The loops
//! themselves live in the private `kernel` module, one compiled per pixel
//! layout, and none of them branches on the layout or indexes a plane per
//! sample.

mod fixed;
mod kernel;

use alloc::vec::Vec;

use crate::error::{Error, Result};
use crate::hevc::{ChromaFormat, Frame};
use crate::image::{Image, PixelLayout, check_pixels};
use crate::parallel;
use crate::props::colr::Nclx;
use fixed::{AlphaScale, Coeffs};
use kernel::Mode;

/// The sample depths this conversion is defined for.
const DEPTHS: core::ops::RangeInclusive<u8> = 8..=16;

/// Convert a frame, and optionally an alpha plane, into an image.
///
/// `alpha` is the luma plane of a decoded auxiliary item, at the same size as
/// `frame`; it is used only when `layout` has an alpha channel.
///
/// # Accuracy
///
/// The arithmetic is fixed point. Every channel is within **1 least
/// significant bit** of the same conversion carried out in `f32` and rounded
/// once at the end — the coefficients carry 16 fractional bits for the 8-bit
/// layouts and 13 for the 16-bit ones, which puts the coefficient error three
/// orders of magnitude below one output step, so the only difference that
/// survives is a rounding tie. `tests/color_fixed.rs` holds a float reference
/// implementation and asserts that bound over a randomised sweep of depths,
/// ranges, matrices and layouts.
pub fn convert(
    frame: &Frame,
    alpha: Option<&Frame>,
    nclx: Nclx,
    layout: PixelLayout,
    max_pixels: u64,
    threads: Option<usize>,
) -> Result<Image> {
    frame.validate()?;
    depth_ok(frame.bit_depth)?;
    let (w, h) = (frame.width, frame.height);
    check_pixels(w, h, max_pixels)?;
    let mut image = Image::zeroed(w, h, layout, max_pixels)?;

    let wide = layout.bytes_per_channel() == 2;
    let coeffs = Coeffs::new(nclx, frame.bit_depth, wide);
    let mode = if frame.chroma == ChromaFormat::Monochrome || layout.is_gray() {
        Mode::Luma
    } else if nclx.matrix.is_identity() {
        Mode::Identity
    } else {
        Mode::Matrix
    };
    run(frame, &coeffs, mode, &mut image, threads);

    if let Some(a) = alpha.filter(|_| layout.has_alpha()) {
        depth_ok(a.bit_depth)?;
        let scale = AlphaScale::new(a.bit_depth, coeffs.max_out);
        kernel::alpha(
            &mut image.data,
            a,
            w as usize,
            h as usize,
            layout.bytes_per_pixel(),
            &scale,
        );
    }
    Ok(image)
}

/// Refuse a depth the fixed-point coefficients are not defined for, rather
/// than shifting past the width of the accumulator.
fn depth_ok(depth: u8) -> Result<()> {
    if DEPTHS.contains(&depth) {
        return Ok(());
    }
    Err(Error::Unsupported("sample bit depth is outside 8 to 16"))
}

/// Pick the loop for this layout. One instantiation per layout, chosen here
/// and never re-examined inside a row.
fn run(frame: &Frame, c: &Coeffs, mode: Mode, image: &mut Image, threads: Option<usize>) {
    let out = &mut image.data;
    match image.layout {
        PixelLayout::Gray8 => bands::<1, false, false>(frame, c, mode, out, threads),
        PixelLayout::Rgb8 => bands::<3, false, false>(frame, c, mode, out, threads),
        PixelLayout::Bgr8 => bands::<3, true, false>(frame, c, mode, out, threads),
        PixelLayout::Rgba8 => bands::<4, false, false>(frame, c, mode, out, threads),
        PixelLayout::Bgra8 => bands::<4, true, false>(frame, c, mode, out, threads),
        PixelLayout::Rgb16 => bands::<6, false, true>(frame, c, mode, out, threads),
        PixelLayout::Rgba16 => bands::<8, false, true>(frame, c, mode, out, threads),
    }
}

/// Rows per band handed to one thread.
///
/// Swept with `benches/decode.rs`'s `color_threads` group on an Apple M3 Max,
/// converting 2048x1536 4:2:0 to RGB8. The pooled time rises monotonically
/// with the band — 16 rows is the fastest measured, and 32, 64, 128 and 256
/// are each slower than the one before — because a wide band both overflows
/// L2 and leaves the pool too few pieces to balance its tail with. Below
/// about twelve rows that reverses and per-band dispatch starts to cost more
/// than the rows do, so the curve has a floor rather than a slope, and 16
/// sits on it. Re-run that group before changing this.
const ROWS_PER_BAND: usize = 16;

/// Below this many pixels a conversion is over before a pool could be woken,
/// so it is done on the calling thread whatever the caller asked for.
/// 2048x1536 converts in about 2 ms serially; a quarter-megapixel image in
/// about 170 us, which is still worth splitting, and a 512x512 one in 40 us,
/// which is not.
const PARALLEL_FLOOR_PIXELS: u64 = 256 * 1024;

/// Convert the image in row bands, on as many threads as the caller allowed.
///
/// Each band gets its own pair of scratch chroma rows, since that scratch is
/// the only mutable state a band carries between its rows.
fn bands<const N: usize, const BGR: bool, const WIDE: bool>(
    frame: &Frame,
    c: &Coeffs,
    mode: Mode,
    out: &mut [u8],
    threads: Option<usize>,
) {
    let w = frame.width as usize;
    let scratch_len = if mode == Mode::Luma { 0 } else { 2 * w };
    let small = u64::from(frame.width) * u64::from(frame.height) < PARALLEL_FLOOR_PIXELS;
    let threads = if small { Some(1) } else { threads };
    parallel::for_each_band(out, w * N, ROWS_PER_BAND, threads, |y0, dst| {
        let mut scratch: Vec<u16> = alloc::vec![0u16; scratch_len];
        kernel::planes::<N, BGR, WIDE>(frame, c, mode, &mut scratch, dst, y0);
    });
}
