//! Loading a fixture, loading its reference decode, and measuring the
//! distance between the two.
//!
//! Kept here rather than in one test file so that every comparison in the
//! suite reports the same numbers the same way.

#![allow(dead_code)]

use super::png;

/// Read a file out of `tests/fixtures`.
pub fn load(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} is missing: {e}"))
}

/// Read `<stem>.ref.png`, the platform decoder's output for a fixture.
pub fn reference(name: &str) -> png::Png {
    let stem = name.strip_suffix(".heic").unwrap_or(name);
    png::decode(&load(&format!("{stem}.ref.png"))).unwrap_or_else(|e| panic!("{stem}.ref.png: {e}"))
}

/// Peak signal-to-noise ratio in decibels, over interleaved 8-bit samples.
pub fn psnr(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "images differ in size");
    let mut sum = 0f64;
    for (x, y) in a.iter().zip(b) {
        let d = f64::from(*x) - f64::from(*y);
        sum += d * d;
    }
    from_mse(sum / a.len() as f64)
}

/// The same, over BT.601 luma, which no chroma upsampling choice can move.
pub fn luma_psnr(a: &[u8], b: &[u8]) -> f64 {
    let luma =
        |p: &[u8]| 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]);
    let mut sum = 0f64;
    for (x, y) in a.chunks_exact(3).zip(b.chunks_exact(3)) {
        let d = luma(x) - luma(y);
        sum += d * d;
    }
    from_mse(sum / (a.len() / 3) as f64)
}

/// Decibels from a mean squared error, with an exact match reported as
/// infinite rather than as a very large number.
fn from_mse(mse: f64) -> f64 {
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0f64 * 255.0 / mse).log10()
    }
}

/// The largest absolute difference between two buffers.
pub fn max_delta(a: &[u8], b: &[u8]) -> u8 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}
