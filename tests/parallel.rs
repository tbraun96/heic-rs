//! The promise the `parallel` feature makes: it changes when a sample is
//! computed, never what it is.
//!
//! Every thread count is compared against `Some(1)`, which is the serial code
//! itself rather than a one-worker pool, and which is therefore also what a
//! build without the feature runs. A build without the feature runs all of
//! these too; there they all take the same path and the test is a tautology,
//! which is the point — it cannot fail for a reason the feature introduced.

mod common;

use common::compare::load;
use heic_rs::{DecodeOptions, PixelLayout};

/// Fixtures worth the comparison: a 4x3 grid, a 2x2 grid, a single tile, and
/// one small enough to sit under every threshold in the crate.
const CORPUS: [&str; 4] = [
    "photo-2048.heic",
    "checker-1024.heic",
    "gradient-512.heic",
    "flat-64.heic",
];

fn decode(name: &str, threads: Option<usize>, layout: PixelLayout) -> heic_rs::Image {
    let options = DecodeOptions::default()
        .with_layout(layout)
        .with_threads(threads);
    heic_rs::decode(&load(name), &options).unwrap_or_else(|e| panic!("{name} {threads:?}: {e}"))
}

#[test]
fn every_thread_count_produces_the_same_bytes() {
    for name in CORPUS {
        let serial = decode(name, Some(1), PixelLayout::Rgb8);
        // 2 exercises the private-pool path, 3 an awkward count that does not
        // divide any band or tile count evenly, and None the shared pool.
        for threads in [Some(2), Some(3), Some(16), None] {
            let other = decode(name, threads, PixelLayout::Rgb8);
            assert_eq!(
                (other.width, other.height),
                (serial.width, serial.height),
                "{name} at {threads:?}"
            );
            assert!(
                other.data == serial.data,
                "{name} at {threads:?} differs from the serial decode"
            );
        }
    }
}

#[test]
fn every_layout_is_thread_independent_too() {
    // The colour kernel is compiled once per layout, so each instantiation
    // needs its own proof that banding it changed nothing.
    let name = "photo-2048.heic";
    for layout in [
        PixelLayout::Rgb8,
        PixelLayout::Rgba8,
        PixelLayout::Bgr8,
        PixelLayout::Bgra8,
        PixelLayout::Gray8,
        PixelLayout::Rgb16,
        PixelLayout::Rgba16,
    ] {
        let serial = decode(name, Some(1), layout);
        let pooled = decode(name, None, layout);
        assert!(pooled.data == serial.data, "{layout:?} differs");
    }
}

#[test]
fn zero_threads_means_this_thread() {
    // `Some(0)` is nonsense as a pool size; it is read as "do not use a pool"
    // rather than refused, because refusing a decode over a thread count
    // would be a worse answer than decoding it correctly on one thread.
    let name = "checker-1024.heic";
    let serial = decode(name, Some(1), PixelLayout::Rgb8);
    let zero = decode(name, Some(0), PixelLayout::Rgb8);
    assert!(zero.data == serial.data);
}
