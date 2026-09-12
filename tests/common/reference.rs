//! A float reference for the conversion, and the random generator that feeds
//! it.
//!
//! `heic_rs::color::convert` is integer arithmetic. [`reference`] is the same
//! conversion written in `f32` and rounded once at the end — the form the
//! crate used before the integer path landed — so a test can assert the
//! documented bound directly rather than freezing a table of numbers.

#![allow(dead_code)]

use heic_rs::hevc::{ChromaFormat, Frame};
use heic_rs::image::PixelLayout;
use heic_rs::props::colr::{Nclx, Range};
use heic_rs::upsample;

/// A tiny deterministic generator, so a failure is reproducible from its seed.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 33) as u32
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// The float conversion, kept here as the thing the integer path is measured
/// against. Chroma comes from `upsample::plane`, which both paths share, so
/// what this compares is the matrix and nothing else.
pub fn reference(frame: &Frame, nclx: Nclx, layout: PixelLayout) -> Vec<u8> {
    let (w, h) = (frame.width as usize, frame.height as usize);
    let top = ((1u32 << frame.bit_depth) - 1) as f32;
    let (y_off, y_scale, c_off, c_scale) = match nclx.range {
        Range::Full => (
            0.0,
            1.0 / top,
            (1u32 << (frame.bit_depth - 1)) as f32,
            1.0 / top,
        ),
        Range::Limited => {
            let step = (1u32 << (frame.bit_depth - 8)) as f32;
            (
                16.0 * step,
                1.0 / (219.0 * step),
                128.0 * step,
                1.0 / (224.0 * step),
            )
        }
    };
    let grey = frame.chroma == ChromaFormat::Monochrome || layout.is_gray();
    let (cw, ch) = frame.chroma.chroma_size(frame.width, frame.height);
    let (cb, cr) = if grey {
        (Vec::new(), Vec::new())
    } else {
        let up = |p: &[u16]| {
            upsample::plane(
                p,
                frame.c_stride,
                cw,
                ch,
                frame.width,
                frame.height,
                frame.chroma,
            )
        };
        (up(&frame.cb), up(&frame.cr))
    };
    let (kr, kb) = nclx.matrix.luma_weights();
    let kg = 1.0 - kr - kb;
    let (rv, bu) = (2.0 * (1.0 - kr), 2.0 * (1.0 - kb));
    let (gu, gv) = (-2.0 * kb * (1.0 - kb) / kg, -2.0 * kr * (1.0 - kr) / kg);
    let bpp = layout.bytes_per_pixel();
    let mut out = vec![0u8; w * h * bpp];
    for y in 0..h {
        for x in 0..w {
            let luma = f32::from(frame.y[y * frame.y_stride as usize + x]);
            let (mut r, mut g, mut b);
            if grey {
                let v = (luma - y_off) * y_scale;
                (r, g, b) = (v, v, v);
            } else if nclx.matrix.is_identity() {
                g = (luma - y_off) * y_scale;
                b = (f32::from(cb[y * w + x]) - y_off) * y_scale;
                r = (f32::from(cr[y * w + x]) - y_off) * y_scale;
            } else {
                let u = (f32::from(cb[y * w + x]) - c_off) * c_scale;
                let v = (f32::from(cr[y * w + x]) - c_off) * c_scale;
                let yy = (luma - y_off) * y_scale;
                r = yy + rv * v;
                g = yy + gu * u + gv * v;
                b = yy + bu * u;
            }
            r = r.clamp(0.0, 1.0);
            g = g.clamp(0.0, 1.0);
            b = b.clamp(0.0, 1.0);
            write_pixel(&mut out[(y * w + x) * bpp..], layout, r, g, b);
        }
    }
    out
}

/// Store one pixel the way the float path used to.
pub fn write_pixel(out: &mut [u8], layout: PixelLayout, r: f32, g: f32, b: f32) {
    let (first, third) = if layout.is_bgr() { (b, r) } else { (r, b) };
    if layout.bytes_per_channel() == 2 {
        let vals = [first, g, third, 1.0];
        for i in 0..layout.channels() {
            let v = (vals[i] * 65_535.0 + 0.5) as u16;
            out[i * 2..i * 2 + 2].copy_from_slice(&v.to_ne_bytes());
        }
        return;
    }
    let vals = [first, g, third, 1.0];
    for i in 0..layout.channels() {
        out[i] = (vals[i] * 255.0 + 0.5) as u8;
    }
}

/// Read an image back as one integer per channel, whatever its depth.
pub fn channels(data: &[u8], layout: PixelLayout) -> Vec<i32> {
    if layout.bytes_per_channel() == 2 {
        return data
            .chunks_exact(2)
            .map(|c| i32::from(u16::from_ne_bytes([c[0], c[1]])))
            .collect();
    }
    data.iter().map(|b| i32::from(*b)).collect()
}

/// A frame of random samples at the given geometry, depth and sampling.
pub fn noisy(rng: &mut Rng, w: u32, h: u32, depth: u8, chroma: ChromaFormat) -> Frame {
    let top = 1u32 << depth;
    let (cw, ch) = chroma.chroma_size(w, h);
    let mut fill = |n: u32| (0..n).map(|_| rng.below(top) as u16).collect::<Vec<u16>>();
    Frame {
        width: w,
        height: h,
        bit_depth: depth,
        chroma,
        y: fill(w * h),
        cb: fill(cw * ch),
        cr: fill(cw * ch),
        y_stride: w,
        c_stride: cw,
    }
}
