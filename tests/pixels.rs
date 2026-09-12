//! Pixel-level comparison against the platform decoder.
//!
//! The ground truth is `tests/fixtures/<name>.ref.png`, which is what Apple's
//! own decoder produced from the same HEIC. Comparing against the platform
//! decoder rather than against another Rust crate avoids inheriting someone
//! else's bugs as "expected", and keeps AGPL and LGPL code out of the test
//! harness.
//!
//! # Where the thresholds come from
//!
//! Two decoders that both implement H.265 correctly produce *identical* luma
//! and chroma samples — the standard's inverse transform and intra prediction
//! are exactly specified in integer arithmetic. Everything these tests
//! tolerate happens after that, in the two steps the standard does not pin
//! down: chroma upsampling and the YCbCr-to-RGB matrix. So:
//!
//! * A fixture whose chroma is constant (an achromatic checkerboard, a solid
//!   colour) has nothing for an upsampler to disagree about, and is asserted
//!   **bit exact**. That is the strongest statement available and it is the
//!   one that would break first if the codec regressed.
//! * Everywhere else the floor is stated twice: once on RGB, which includes
//!   the upsampler's choices, and once on BT.601 luma, which does not. Luma is
//!   the tighter of the two and is what actually pins the decoder.
//!
//! The floors below sit 3 to 5 dB under what this crate measures today, which
//! is the width of a rounding-tie disagreement and nothing more. A wrong
//! coefficient, a wrong prediction mode or a mis-sited chroma plane costs tens
//! of decibels, not three. If a fixture regresses past one of these, the bug
//! is in the decoder; do not move the number.

mod common;

use common::compare::{load, luma_psnr, max_delta, psnr, reference};
use heic_rs::{DecodeOptions, PixelLayout};

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

/// What one fixture is expected to achieve, and why.
struct Bar {
    name: &'static str,
    /// Demand an identical buffer, not a PSNR.
    exact: bool,
    /// Floor on RGB PSNR, which includes chroma upsampling.
    rgb_db: f64,
    /// Floor on luma PSNR, which does not.
    luma_db: f64,
}

/// `exact` is claimed exactly where the fixture's chroma is constant, so that
/// the upsampler has no freedom to exercise. Everything else measures 47.8 to
/// 49.8 dB RGB and 57.6 to 60.0 dB luma today; the floors are set below that
/// by the width of a rounding disagreement.
const BARS: &[Bar] = &[
    // Solid white: no chroma detail, no residual, nothing to disagree about.
    Bar {
        name: "flat-white-16.heic",
        exact: true,
        rgb_db: 0.0,
        luma_db: 0.0,
    },
    // Black and white squares: achromatic, so both chroma planes are flat.
    Bar {
        name: "checker-64.heic",
        exact: true,
        rgb_db: 0.0,
        luma_db: 0.0,
    },
    // The same, four 512x512 tiles of it: also exercises grid compositing.
    Bar {
        name: "checker-1024.heic",
        exact: true,
        rgb_db: 0.0,
        luma_db: 0.0,
    },
    // A solid mid-grey that is *not* achromatic in the coded chroma planes:
    // the last chroma column is deblocked against the cropped-away region.
    Bar {
        name: "flat-64.heic",
        exact: false,
        rgb_db: 45.0,
        luma_db: 55.0,
    },
    // Saturated vertical bars: the hardest case for a chroma upsampler, and
    // still within a code of the platform's.
    Bar {
        name: "rgb-strips-96.heic",
        exact: false,
        rgb_db: 45.0,
        luma_db: 55.0,
    },
    // Smooth ramps: nothing for the upsampler to ring on, so the residual
    // path is what is being measured here.
    Bar {
        name: "gradient-512.heic",
        exact: false,
        rgb_db: 45.0,
        luma_db: 55.0,
    },
    // A 4x3 grid of 512x512 tiles of photographic content: the whole chain.
    Bar {
        name: "photo-2048.heic",
        exact: false,
        rgb_db: 45.0,
        luma_db: 55.0,
    },
];

#[test]
fn the_reference_pngs_are_readable() {
    // Independent of the decoder: it proves the comparison harness itself
    // works, so that a failure below is known to be the codec's.
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
fn decoded_pixels_match_the_platform_decoder() {
    for bar in BARS {
        let name = bar.name;
        let r = reference(name);
        let image = heic_rs::decode(&load(name), &DecodeOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (r.width, r.height), "{name}");
        if bar.exact {
            assert_eq!(
                max_delta(&image.data, &r.rgb),
                0,
                "{name}: chroma is constant here, so this must be bit exact"
            );
            continue;
        }
        let (rgb, luma) = (psnr(&image.data, &r.rgb), luma_psnr(&image.data, &r.rgb));
        assert!(
            rgb >= bar.rgb_db,
            "{name}: {rgb:.2} dB RGB, wanted at least {:.1}",
            bar.rgb_db
        );
        assert!(
            luma >= bar.luma_db,
            "{name}: {luma:.2} dB luma, wanted at least {:.1}",
            bar.luma_db
        );
    }
}

#[test]
fn flat_fixtures_are_very_nearly_exact() {
    // A solid colour has no chroma detail to lose, so anything worse than a
    // couple of codes would be a matrix or range bug rather than a filter
    // disagreement. Measured today: 0 and 2.
    for (name, allowed) in [("flat-white-16.heic", 0u8), ("flat-64.heic", 3)] {
        let r = reference(name);
        let image = heic_rs::decode(&load(name), &DecodeOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let d = max_delta(&image.data, &r.rgb);
        assert!(d <= allowed, "{name} drifted by {d}, allowed {allowed}");
    }
}

#[test]
fn grid_fixtures_have_no_seams_at_tile_boundaries() {
    // A 512-pixel tile boundary in a smooth image is where a compositing bug
    // shows first: compare the columns either side of it against the platform
    // decoder's, which has no seam. Measured worst case today: 4.
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
