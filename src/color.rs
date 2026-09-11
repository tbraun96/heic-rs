//! Turning decoded YCbCr samples into the caller's chosen pixel layout.
//!
//! The matrix and the sample range both come from the file's `colr` property.
//! When a file carries no `colr` at all the conversion uses BT.709 limited
//! range, which is what unlabelled HEIC means in practice; that fallback is
//! [`Nclx::default`] and is chosen once, here, rather than being guessed at by
//! a parser.

use alloc::vec::Vec;

use crate::error::Result;
use crate::hevc::{ChromaFormat, Frame};
use crate::image::{Image, PixelLayout, check_pixels};
use crate::props::colr::{Nclx, Range};
use crate::upsample;

/// The scaling a given bit depth and range imply.
struct Scale {
    y_offset: f32,
    y_scale: f32,
    c_offset: f32,
    c_scale: f32,
}

impl Scale {
    fn new(depth: u8, range: Range) -> Scale {
        let max = ((1u32 << depth) - 1) as f32;
        match range {
            Range::Full => Scale {
                y_offset: 0.0,
                y_scale: 1.0 / max,
                c_offset: (1u32 << (depth - 1)) as f32,
                c_scale: 1.0 / max,
            },
            Range::Limited => {
                let unit = (1u32 << (depth - 8)) as f32;
                Scale {
                    y_offset: 16.0 * unit,
                    y_scale: 1.0 / (219.0 * unit),
                    c_offset: 128.0 * unit,
                    c_scale: 1.0 / (224.0 * unit),
                }
            }
        }
    }
}

/// Convert a frame, and optionally an alpha plane, into an image.
///
/// `alpha` is the luma plane of a decoded auxiliary item, at the same size as
/// `frame`; it is used only when `layout` has an alpha channel.
pub fn convert(
    frame: &Frame,
    alpha: Option<&Frame>,
    nclx: Nclx,
    layout: PixelLayout,
    max_pixels: u64,
) -> Result<Image> {
    frame.validate()?;
    let (w, h) = (frame.width, frame.height);
    check_pixels(w, h, max_pixels)?;
    let mut image = Image::zeroed(w, h, layout, max_pixels)?;

    let mono = frame.chroma == ChromaFormat::Monochrome;
    let (cw, ch) = frame.chroma.chroma_size(w, h);
    let (cb, cr) = if mono || layout.is_gray() {
        (Vec::new(), Vec::new())
    } else {
        (
            upsample::plane(&frame.cb, frame.c_stride, cw, ch, w, h, frame.chroma),
            upsample::plane(&frame.cr, frame.c_stride, cw, ch, w, h, frame.chroma),
        )
    };

    let s = Scale::new(frame.bit_depth, nclx.range);
    let (kr, kb) = nclx.matrix.luma_weights();
    let kg = 1.0 - kr - kb;
    let (rv, bu) = (2.0 * (1.0 - kr), 2.0 * (1.0 - kb));
    let (gu, gv) = (-2.0 * kb * (1.0 - kb) / kg, -2.0 * kr * (1.0 - kr) / kg);
    let identity = nclx.matrix.is_identity();

    let alpha_plane = alpha.map(|a| (a.y.as_slice(), a.y_stride, a.bit_depth));
    let bpp = layout.bytes_per_pixel();
    for y in 0..h as usize {
        let yrow = y * frame.y_stride as usize;
        let crow = y * w as usize;
        for x in 0..w as usize {
            let luma = f32::from(frame.y[yrow + x]);
            let (mut r, mut g, mut b);
            if mono || layout.is_gray() {
                let v = (luma - s.y_offset) * s.y_scale;
                r = v;
                g = v;
                b = v;
            } else if identity {
                // Matrix 0 means the planes already hold G, B and R, all on
                // the luma range rather than the chroma one.
                g = (luma - s.y_offset) * s.y_scale;
                b = (f32::from(cb[crow + x]) - s.y_offset) * s.y_scale;
                r = (f32::from(cr[crow + x]) - s.y_offset) * s.y_scale;
            } else {
                let u = (f32::from(cb[crow + x]) - s.c_offset) * s.c_scale;
                let v = (f32::from(cr[crow + x]) - s.c_offset) * s.c_scale;
                let yy = (luma - s.y_offset) * s.y_scale;
                r = yy + rv * v;
                g = yy + gu * u + gv * v;
                b = yy + bu * u;
            }
            r = r.clamp(0.0, 1.0);
            g = g.clamp(0.0, 1.0);
            b = b.clamp(0.0, 1.0);
            let a = alpha_at(alpha_plane, x, y);
            write_pixel(
                &mut image.data[(y * w as usize + x) * bpp..],
                layout,
                r,
                g,
                b,
                a,
            );
        }
    }
    Ok(image)
}

/// Sample the alpha plane, returning fully opaque when there is none.
fn alpha_at(plane: Option<(&[u16], u32, u8)>, x: usize, y: usize) -> f32 {
    let Some((data, stride, depth)) = plane else {
        return 1.0;
    };
    let max = ((1u32 << depth) - 1) as f32;
    let idx = y * stride as usize + x;
    data.get(idx).map_or(1.0, |v| f32::from(*v) / max)
}

/// Store one pixel in the requested layout.
fn write_pixel(out: &mut [u8], layout: PixelLayout, r: f32, g: f32, b: f32, a: f32) {
    let q8 = |v: f32| (v * 255.0 + 0.5) as u8;
    let q16 = |v: f32| (v * 65535.0 + 0.5) as u16;
    match layout {
        PixelLayout::Gray8 => out[0] = q8(r),
        PixelLayout::Rgb8 | PixelLayout::Bgr8 | PixelLayout::Rgba8 | PixelLayout::Bgra8 => {
            let (first, third) = if layout.is_bgr() { (b, r) } else { (r, b) };
            out[0] = q8(first);
            out[1] = q8(g);
            out[2] = q8(third);
            if layout.has_alpha() {
                out[3] = q8(a);
            }
        }
        PixelLayout::Rgb16 | PixelLayout::Rgba16 => {
            let n = if layout.has_alpha() { 4 } else { 3 };
            let vals = [q16(r), q16(g), q16(b), q16(a)];
            for (i, v) in vals.iter().take(n).enumerate() {
                out[i * 2..i * 2 + 2].copy_from_slice(&v.to_ne_bytes());
            }
        }
    }
}
