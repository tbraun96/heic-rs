//! The fixed-point matrix against a float reference, and against a known RGB
//! image pushed through the inverse matrix.

mod common;

use common::reference::{Rng, channels, noisy, reference};
use heic_rs::color;
use heic_rs::hevc::{ChromaFormat, Frame};
use heic_rs::image::PixelLayout;
use heic_rs::props::colr::{MatrixCoefficients, Nclx, Range};

#[test]
fn the_integer_matrix_stays_within_one_bit_of_the_float_one() {
    let matrices = [
        MatrixCoefficients::Identity,
        MatrixCoefficients::Bt601,
        MatrixCoefficients::Bt709,
        MatrixCoefficients::Bt2020Ncl,
        MatrixCoefficients::Smpte240m,
    ];
    let layouts = [
        PixelLayout::Rgb8,
        PixelLayout::Bgra8,
        PixelLayout::Gray8,
        PixelLayout::Rgb16,
        PixelLayout::Rgba16,
    ];
    let chromas = [
        ChromaFormat::Yuv420,
        ChromaFormat::Yuv422,
        ChromaFormat::Yuv444,
        ChromaFormat::Monochrome,
    ];
    let mut rng = Rng(0x5eed_1234_9876_abcd);
    let mut worst = 0i32;
    let mut cases = 0;
    for depth in [8u8, 10, 12] {
        for range in [Range::Full, Range::Limited] {
            for matrix in matrices {
                for chroma in chromas {
                    // Odd sizes on purpose: they exercise the edge clamps.
                    let (w, h) = (1 + rng.below(9), 1 + rng.below(9));
                    let frame = noisy(&mut rng, w, h, depth, chroma);
                    let n = Nclx {
                        primaries: 1,
                        transfer: 13,
                        matrix,
                        matrix_code: 0,
                        range,
                    };
                    for layout in layouts {
                        let got = color::convert(&frame, None, n, layout, u64::MAX, None)
                            .expect("converts")
                            .data;
                        let want = reference(&frame, n, layout);
                        for (a, b) in channels(&got, layout)
                            .into_iter()
                            .zip(channels(&want, layout))
                        {
                            worst = worst.max((a - b).abs());
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 600, "the sweep should cover every combination");
    assert!(worst <= 1, "fixed point drifted by {worst}, not at most 1");
}

#[test]
fn a_known_rgb_image_survives_the_trip_through_ycbcr() {
    // Forward matrix here, inverse matrix in the crate: the only losses are
    // the two quantisations to eight bits, which cost under two steps.
    let n = Nclx {
        primaries: 1,
        transfer: 13,
        matrix: MatrixCoefficients::Bt709,
        matrix_code: 1,
        range: Range::Full,
    };
    let (w, h) = (37u32, 23u32);
    let (kr, kb) = n.matrix.luma_weights();
    let kg = 1.0 - kr - kb;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    let (mut y, mut cb, mut cr) = (Vec::new(), Vec::new(), Vec::new());
    for row in 0..h {
        for col in 0..w {
            let (r, g, b) = (
                (col * 7 % 256) as u8,
                (row * 11 % 256) as u8,
                ((col + row) * 5 % 256) as u8,
            );
            rgb.extend_from_slice(&[r, g, b]);
            let (rf, gf, bf) = (f32::from(r), f32::from(g), f32::from(b));
            let luma = kr * rf + kg * gf + kb * bf;
            y.push((luma + 0.5) as u16);
            cb.push(((bf - luma) / (2.0 * (1.0 - kb)) + 128.5) as u16);
            cr.push(((rf - luma) / (2.0 * (1.0 - kr)) + 128.5) as u16);
        }
    }
    let frame = Frame {
        width: w,
        height: h,
        bit_depth: 8,
        chroma: ChromaFormat::Yuv444,
        y,
        cb,
        cr,
        y_stride: w,
        c_stride: w,
    };
    let out = color::convert(&frame, None, n, PixelLayout::Rgb8, u64::MAX, None).expect("converts");
    let worst = out
        .data
        .iter()
        .zip(&rgb)
        .map(|(a, b)| i32::from(*a) - i32::from(*b))
        .map(i32::abs)
        .max()
        .unwrap_or(0);
    assert!(worst <= 2, "round trip drifted by {worst}, not at most 2");
}
