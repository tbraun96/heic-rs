//! The two grids from `scripts/make-grid-vectors.sh`, against the platform
//! decoder, as `pixels.rs` does for the `sips` corpus.
//!
//! Both are layouts `sips` cannot write, so they come from libheif's
//! `heif-enc`; the reference is still Apple's decode. The floors sit about
//! 3 dB under what the decoder measures with tbraun96/heic-rs#7 and #10, and
//! far above what it measured before them (in brackets below).

mod common;

use common::compare::{load, luma_psnr, psnr, reference};
use heic_rs::DecodeOptions;

/// The mean of all three channels over the `n` x `n` square at (`x0`, `y0`).
fn patch_mean(rgb: &[u8], width: usize, x0: usize, y0: usize, n: usize) -> f64 {
    let mut sum = 0f64;
    for y in y0..y0 + n {
        let row = &rgb[(y * width + x0) * 3..(y * width + x0 + n) * 3];
        sum += row.iter().map(|&v| f64::from(v)).sum::<f64>();
    }
    sum / (n * n * 3) as f64
}

#[test]
fn a_4_2_2_grid_matches_the_platform_decoder() {
    // The layout of Canon and Sony `.HIF` files. Measured: 48.3 dB RGB,
    // 61.4 dB luma (18.8 and 58.3 with the 4:2:2 chroma modes misread).
    let name = "grid-422-10bit.heic";
    let r = reference(name);
    let image = heic_rs::decode(&load(name), &DecodeOptions::default())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!((image.width, image.height), (r.width, r.height), "{name}");
    let (rgb, luma) = (psnr(&image.data, &r.rgb), luma_psnr(&image.data, &r.rgb));
    assert!(rgb >= 45.0, "{name}: {rgb:.2} dB RGB, wanted at least 45.0");
    assert!(
        luma >= 57.0,
        "{name}: {luma:.2} dB luma, wanted at least 57.0"
    );
}

#[test]
fn full_range_said_only_in_the_bitstream_stays_full_range() {
    // No `colr` anywhere: the tiles' VUI is the only colour description.
    // Read as limited range, the near-black quarter (8) comes out at 0 and the
    // near-white one (248) at 255. Measured: 8.00 and 248.00, as Apple's;
    // 41.5 dB RGB, 54.9 dB luma (29.6 and 30.4 read as limited range). RGB is
    // lower than elsewhere because of the seams between the quarters, where
    // Apple upsamples each tile's chroma on its own.
    let name = "full-range-no-colr.heic";
    let r = reference(name);
    let image = heic_rs::decode(&load(name), &DecodeOptions::default())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!((image.width, image.height), (r.width, r.height), "{name}");
    let width = r.width as usize;
    for (what, x0) in [("near black", 16), ("near white", 80)] {
        let ours = patch_mean(&image.data, width, x0, 16, 32);
        let theirs = patch_mean(&r.rgb, width, x0, 16, 32);
        assert!(
            (ours - theirs).abs() <= 1.0,
            "{name}: {what} is {ours:.2}, Apple's is {theirs:.2}"
        );
    }
    let (rgb, luma) = (psnr(&image.data, &r.rgb), luma_psnr(&image.data, &r.rgb));
    assert!(rgb >= 38.0, "{name}: {rgb:.2} dB RGB, wanted at least 38.0");
    assert!(
        luma >= 51.0,
        "{name}: {luma:.2} dB luma, wanted at least 51.0"
    );
}
