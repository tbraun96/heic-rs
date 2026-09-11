//! YCbCr to RGB: matrices, ranges, bit depths, layouts and chroma upsampling.

mod common;

use common::*;
use heic_rs::color;
use heic_rs::error::Error;
use heic_rs::hevc::{ChromaFormat, Frame};
use heic_rs::image::PixelLayout;
use heic_rs::props::colr::{MatrixCoefficients, Nclx, Range};

fn nclx(matrix: MatrixCoefficients, range: Range) -> Nclx {
    Nclx {
        primaries: 1,
        transfer: 13,
        matrix,
        matrix_code: 0,
        range,
    }
}

fn convert(frame: &Frame, n: Nclx, layout: PixelLayout) -> Vec<u8> {
    color::convert(frame, None, n, layout, u64::MAX)
        .expect("converts")
        .data
}

#[test]
fn full_range_white_and_black_survive_the_round_trip() {
    let n = nclx(MatrixCoefficients::Bt601, Range::Full);
    let white = solid_frame(2, 2, 255, 128, 128);
    assert_eq!(convert(&white, n, PixelLayout::Rgb8), vec![255; 12]);
    let black = solid_frame(2, 2, 0, 128, 128);
    assert_eq!(convert(&black, n, PixelLayout::Rgb8), vec![0; 12]);
}

#[test]
fn limited_range_maps_16_to_black_and_235_to_white() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Limited);
    assert_eq!(
        convert(&solid_frame(1, 1, 16, 128, 128), n, PixelLayout::Rgb8),
        vec![0, 0, 0]
    );
    assert_eq!(
        convert(&solid_frame(1, 1, 235, 128, 128), n, PixelLayout::Rgb8),
        vec![255, 255, 255]
    );
    // Studio swing clamps rather than wrapping: below 16 is still black.
    assert_eq!(
        convert(&solid_frame(1, 1, 0, 128, 128), n, PixelLayout::Rgb8),
        vec![0, 0, 0]
    );
    assert_eq!(
        convert(&solid_frame(1, 1, 255, 128, 128), n, PixelLayout::Rgb8),
        vec![255, 255, 255]
    );
}

#[test]
fn ten_bit_full_range_uses_the_whole_coded_range() {
    let mut f = solid_frame(1, 1, 1023, 512, 512);
    f.bit_depth = 10;
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    assert_eq!(convert(&f, n, PixelLayout::Rgb8), vec![255, 255, 255]);

    let mut f = solid_frame(1, 1, 512, 512, 512);
    f.bit_depth = 10;
    let mid = convert(&f, n, PixelLayout::Rgb8);
    // Half of 1023 rounds to 128 out of 255.
    assert_eq!(mid, vec![128, 128, 128]);
}

#[test]
fn ten_bit_limited_range_uses_the_scaled_studio_swing() {
    let mut f = solid_frame(1, 1, 64, 512, 512);
    f.bit_depth = 10;
    let n = nclx(MatrixCoefficients::Bt709, Range::Limited);
    assert_eq!(convert(&f, n, PixelLayout::Rgb8), vec![0, 0, 0]);
    let mut f = solid_frame(1, 1, 940, 512, 512);
    f.bit_depth = 10;
    assert_eq!(convert(&f, n, PixelLayout::Rgb8), vec![255, 255, 255]);
}

#[test]
fn chroma_moves_the_hue_in_the_expected_direction() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    // Cr above neutral pushes red up and blue down.
    let reddish = convert(&solid_frame(1, 1, 128, 128, 200), n, PixelLayout::Rgb8);
    assert!(reddish[0] > 128, "red should rise, got {reddish:?}");
    assert!(reddish[2] < 140, "blue should not rise, got {reddish:?}");
    // Cb above neutral pushes blue up.
    let bluish = convert(&solid_frame(1, 1, 128, 200, 128), n, PixelLayout::Rgb8);
    assert!(bluish[2] > 128, "blue should rise, got {bluish:?}");
    assert!(bluish[0] < 140, "red should not rise, got {bluish:?}");
}

#[test]
fn different_matrices_give_different_answers() {
    let f = solid_frame(1, 1, 128, 200, 100);
    let a = convert(
        &f,
        nclx(MatrixCoefficients::Bt601, Range::Full),
        PixelLayout::Rgb8,
    );
    let b = convert(
        &f,
        nclx(MatrixCoefficients::Bt709, Range::Full),
        PixelLayout::Rgb8,
    );
    let c = convert(
        &f,
        nclx(MatrixCoefficients::Bt2020Ncl, Range::Full),
        PixelLayout::Rgb8,
    );
    assert_ne!(a, b);
    assert_ne!(b, c);
}

#[test]
fn the_identity_matrix_reads_the_planes_as_gbr() {
    let mut f = solid_frame(1, 1, 10, 20, 30);
    f.chroma = ChromaFormat::Yuv444;
    f.c_stride = 1;
    f.cb = vec![20];
    f.cr = vec![30];
    let n = nclx(MatrixCoefficients::Identity, Range::Full);
    // Luma is G, Cb is B, Cr is R, all on the luma scale.
    assert_eq!(convert(&f, n, PixelLayout::Rgb8), vec![30, 10, 20]);
}

#[test]
fn every_layout_has_the_size_and_order_it_promises() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let f = solid_frame(2, 1, 255, 128, 128);
    for (layout, len) in [
        (PixelLayout::Rgb8, 6),
        (PixelLayout::Rgba8, 8),
        (PixelLayout::Bgr8, 6),
        (PixelLayout::Bgra8, 8),
        (PixelLayout::Gray8, 2),
        (PixelLayout::Rgb16, 12),
        (PixelLayout::Rgba16, 16),
    ] {
        let out = color::convert(&f, None, n, layout, u64::MAX).expect("converts");
        assert_eq!(out.data.len(), len, "{layout:?}");
        assert_eq!(out.data.len(), out.row_bytes() * out.height as usize);
    }
    // With a red-leaning pixel, RGB and BGR really are reversed.
    let red = solid_frame(1, 1, 128, 100, 200);
    let rgb = convert(&red, n, PixelLayout::Rgb8);
    let bgr = convert(&red, n, PixelLayout::Bgr8);
    assert_eq!(rgb, vec![bgr[2], bgr[1], bgr[0]]);
}

#[test]
fn alpha_is_opaque_when_the_file_has_none() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let out = convert(&solid_frame(1, 1, 255, 128, 128), n, PixelLayout::Rgba8);
    assert_eq!(out[3], 255);
}

#[test]
fn an_alpha_plane_is_carried_into_the_output() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let frame = solid_frame(2, 1, 255, 128, 128);
    let alpha = mono_frame(2, 1, |x, _| if x == 0 { 0 } else { 255 });
    let out =
        color::convert(&frame, Some(&alpha), n, PixelLayout::Rgba8, u64::MAX).expect("converts");
    assert_eq!(out.data[3], 0);
    assert_eq!(out.data[7], 255);
}

#[test]
fn sixteen_bit_output_uses_the_full_range() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let out = color::convert(
        &solid_frame(1, 1, 255, 128, 128),
        None,
        n,
        PixelLayout::Rgb16,
        u64::MAX,
    )
    .expect("converts");
    let first = u16::from_ne_bytes([out.data[0], out.data[1]]);
    assert_eq!(first, 65_535);
}

#[test]
fn monochrome_frames_become_grey() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let f = mono_frame(2, 1, |x, _| if x == 0 { 0 } else { 255 });
    assert_eq!(
        convert(&f, n, PixelLayout::Rgb8),
        vec![0, 0, 0, 255, 255, 255]
    );
    assert_eq!(convert(&f, n, PixelLayout::Gray8), vec![0, 255]);
}

#[test]
fn flat_chroma_survives_upsampling_unchanged() {
    // A 4:2:0 frame with constant chroma must upsample to constant chroma,
    // whatever the filter does at the edges.
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let out = convert(&solid_frame(6, 4, 128, 200, 60), n, PixelLayout::Rgb8);
    let first: Vec<u8> = out[0..3].to_vec();
    for px in out.chunks(3) {
        assert_eq!(px, first.as_slice());
    }
}

#[test]
fn upsampling_interpolates_between_chroma_samples() {
    // Two chroma columns, far apart in Cr; the luma is 4 wide, so the odd
    // columns must land between their neighbours rather than duplicating.
    let mut f = solid_frame(4, 2, 128, 128, 0);
    f.cr = vec![0, 255];
    f.cb = vec![128, 128];
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let out = convert(&f, n, PixelLayout::Rgb8);
    let reds: Vec<u8> = out.chunks(3).take(4).map(|p| p[0]).collect();
    assert!(
        reds[0] < reds[1],
        "column 1 should interpolate upward: {reds:?}"
    );
    assert!(
        reds[1] < reds[2],
        "column 2 should be the second sample: {reds:?}"
    );
    assert_eq!(
        reds[2], reds[3],
        "the last column clamps to the edge: {reds:?}"
    );
}

#[test]
fn conversion_respects_the_pixel_limit() {
    let n = nclx(MatrixCoefficients::Bt709, Range::Full);
    let f = solid_frame(4, 4, 128, 128, 128);
    let e = color::convert(&f, None, n, PixelLayout::Rgb8, 8).expect_err("16 pixels over 8");
    assert_eq!(
        e,
        Error::PixelLimit {
            pixels: 16,
            max_pixels: 8
        }
    );
}
