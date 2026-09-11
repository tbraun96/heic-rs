//! Decode a HEIF file and report the resulting buffer.
//!
//! ```text
//! cargo run --example decode -- tests/fixtures/photo-2048.heic
//! ```
//!
//! Until the HEVC decoder lands this prints the codec seam's refusal, which is
//! the honest current behaviour rather than a silent wrong answer.

use heic_rs::{DecodeOptions, PixelLayout};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: decode <file.heic> [rgb8|rgba8|bgr8|bgra8|gray8|rgb16|rgba16]");
        std::process::exit(2);
    };
    let layout = match std::env::args().nth(2).as_deref() {
        None | Some("rgb8") => PixelLayout::Rgb8,
        Some("rgba8") => PixelLayout::Rgba8,
        Some("bgr8") => PixelLayout::Bgr8,
        Some("bgra8") => PixelLayout::Bgra8,
        Some("gray8") => PixelLayout::Gray8,
        Some("rgb16") => PixelLayout::Rgb16,
        Some("rgba16") => PixelLayout::Rgba16,
        Some(other) => {
            eprintln!("unknown layout: {other}");
            std::process::exit(2);
        }
    };
    let options = DecodeOptions::default().with_layout(layout);
    match heic_rs::io::decode_file(&path, &options) {
        Ok(image) => println!(
            "{}x{} {:?}, {} bytes",
            image.width,
            image.height,
            image.layout,
            image.data.len()
        ),
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    }
}
