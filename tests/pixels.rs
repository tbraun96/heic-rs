//! Pixel-level comparison against the platform decoder.
//!
//! Every test in this file is `#[ignore]`d, because the HEVC still-picture
//! decoder is not linked in yet: `decode` currently stops at the codec seam
//! and returns `Error::Unsupported`. When the decoder lands, drop the
//! `#[ignore]` attributes and run `cargo test -- --ignored` — nothing else
//! here needs to change.
//!
//! The ground truth is `tests/fixtures/<name>.ref.png`, which is what Apple's
//! own decoder produced from the same HEIC. Comparing against the platform
//! decoder rather than against another Rust crate avoids inheriting someone
//! else's bugs as "expected", and keeps AGPL and LGPL code out of the test
//! harness.

mod common;

use common::png;
use heic_rs::{DecodeOptions, PixelLayout};

fn load(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} is missing: {e}"))
}

fn reference(name: &str) -> png::Png {
    let stem = name.strip_suffix(".heic").unwrap_or(name);
    png::decode(&load(&format!("{stem}.ref.png"))).unwrap_or_else(|e| panic!("{stem}.ref.png: {e}"))
}

/// Peak signal-to-noise ratio in decibels, over interleaved 8-bit samples.
fn psnr(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "images differ in size");
    let mut sum = 0f64;
    for (x, y) in a.iter().zip(b) {
        let d = f64::from(*x) - f64::from(*y);
        sum += d * d;
    }
    let mse = sum / a.len() as f64;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0f64 * 255.0 / mse).log10()
    }
}

/// The largest absolute difference between two buffers.
fn max_delta(a: &[u8], b: &[u8]) -> u8 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}

const CORPUS: [&str; 9] = [
    "flat-white-16.heic",
    "flat-64.heic",
    "checker-64.heic",
    "rgb-strips-96.heic",
    "gradient-512.heic",
    "checker-1024.heic",
    "photo-2048.heic",
    "rotated-90.heic",
    "with-exif.heic",
];

#[test]
fn the_reference_pngs_are_readable() {
    // This one is deliberately not ignored: it is independent of the decoder
    // and proves the comparison harness itself works, so that when the codec
    // lands a failure there is known to be the codec's.
    for name in CORPUS {
        let r = reference(name);
        assert_eq!(
            r.rgb.len(),
            r.width as usize * r.height as usize * 3,
            "{name}"
        );
    }
}

#[test]
#[ignore = "needs the HEVC decoder, which is not linked in yet"]
fn decoded_pixels_match_the_platform_decoder() {
    // Smooth content drifts by a code or two through chroma upsampling; flat
    // and hard-edged content should land much closer.
    for (name, min_psnr) in [
        ("flat-white-16.heic", 60.0),
        ("flat-64.heic", 45.0),
        ("checker-64.heic", 45.0),
        ("rgb-strips-96.heic", 35.0),
        ("gradient-512.heic", 40.0),
        ("checker-1024.heic", 40.0),
        ("photo-2048.heic", 35.0),
    ] {
        let r = reference(name);
        let image = heic_rs::decode(&load(name), &DecodeOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (r.width, r.height), "{name}");
        let db = psnr(&image.data, &r.rgb);
        assert!(
            db >= min_psnr,
            "{name}: {db:.1} dB, wanted at least {min_psnr} dB"
        );
    }
}

#[test]
#[ignore = "needs the HEVC decoder, which is not linked in yet"]
fn flat_fixtures_are_very_nearly_exact() {
    // A solid colour has no chroma detail to lose, so anything worse than a
    // couple of codes would be a matrix or range bug.
    for name in ["flat-white-16.heic", "flat-64.heic"] {
        let r = reference(name);
        let image = heic_rs::decode(&load(name), &DecodeOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            max_delta(&image.data, &r.rgb) <= 4,
            "{name} drifted too far"
        );
    }
}

#[test]
#[ignore = "needs the HEVC decoder, which is not linked in yet"]
fn grid_fixtures_have_no_seams_at_tile_boundaries() {
    // A 512-pixel tile boundary in a smooth image is where a compositing bug
    // shows first: compare the columns either side of it against the platform
    // decoder's, which has no seam.
    let name = "photo-2048.heic";
    let r = reference(name);
    let image = heic_rs::decode(&load(name), &DecodeOptions::default())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    for boundary in [512usize, 1024, 1536] {
        for y in (0..r.height as usize).step_by(37) {
            for x in [boundary - 1, boundary] {
                let i = (y * r.width as usize + x) * 3;
                assert!(
                    max_delta(&image.data[i..i + 3], &r.rgb[i..i + 3]) <= 8,
                    "{name}: seam at ({x}, {y})"
                );
            }
        }
    }
}

#[test]
#[ignore = "needs the HEVC decoder, which is not linked in yet"]
fn every_layout_produces_a_consistent_buffer() {
    let name = "checker-64.heic";
    let bytes = load(name);
    let rgb = heic_rs::decode(&bytes, &DecodeOptions::default()).expect("decodes");
    for layout in [
        PixelLayout::Rgba8,
        PixelLayout::Bgr8,
        PixelLayout::Bgra8,
        PixelLayout::Gray8,
        PixelLayout::Rgb16,
        PixelLayout::Rgba16,
    ] {
        let out = heic_rs::decode(&bytes, &DecodeOptions::default().with_layout(layout))
            .unwrap_or_else(|e| panic!("{layout:?}: {e}"));
        assert_eq!(
            (out.width, out.height),
            (rgb.width, rgb.height),
            "{layout:?}"
        );
        assert_eq!(
            out.data.len(),
            out.row_bytes() * out.height as usize,
            "{layout:?}"
        );
    }
    // BGR really is RGB with the ends swapped.
    let bgr = heic_rs::decode(
        &bytes,
        &DecodeOptions::default().with_layout(PixelLayout::Bgr8),
    )
    .expect("decodes");
    for (a, b) in rgb.data.chunks(3).zip(bgr.data.chunks(3)) {
        assert_eq!([a[2], a[1], a[0]], [b[0], b[1], b[2]]);
    }
}
