//! Whole-file decode throughput: bytes in, RGB8 out.
//!
//! This measures the thing a caller actually waits for — open the file, walk
//! the container, decode every tile, compose, convert colour — and nothing
//! else. One warm-up pass is discarded and the best of `runs` is reported,
//! because the best run is the one least polluted by whatever else the
//! machine was doing.
//!
//! ```text
//! cargo run --release --example throughput -- [corpus-dir] [runs] [threads]
//! ```
//!
//! `corpus-dir` defaults to `tests/fixtures`. Any directory of `.heic` files
//! works, which is how the same numbers are produced for another decoder on
//! the same inputs. `threads` is `auto` (the default) or a count; `1` is the
//! serial path, which is what the `parallel` feature turning itself off looks
//! like.

use std::time::Instant;

use heic_rs::{DecodeOptions, PixelLayout};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures").to_string());
    let runs: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(5);
    let threads = args.next().and_then(|v| v.parse::<usize>().ok());
    let options = DecodeOptions::default()
        .with_layout(PixelLayout::Rgb8)
        .with_threads(threads);

    let mut files: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(d) => d
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("heic"))
            })
            .collect(),
        Err(e) => {
            eprintln!("{dir}: {e}");
            std::process::exit(1);
        }
    };
    files.sort();

    match threads {
        Some(n) => println!("heic-rs, {n} thread(s), best of {runs}"),
        None => println!("heic-rs, default thread pool, best of {runs}"),
    }
    println!(
        "{:<22} {:>11} {:>10} {:>12}",
        "file", "size", "best", "throughput"
    );
    for path in files {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        // Warm up: fault the file in and let the allocator settle.
        if let Err(e) = heic_rs::decode(&bytes, &options) {
            println!("{name:<22} FAILED {e}");
            continue;
        }
        let mut best = f64::MAX;
        let mut dims = (0u32, 0u32);
        for _ in 0..runs {
            let started = Instant::now();
            match heic_rs::decode(&bytes, &options) {
                Ok(image) => {
                    best = best.min(started.elapsed().as_secs_f64() * 1000.0);
                    dims = (image.width, image.height);
                }
                Err(e) => {
                    println!("{name:<22} FAILED {e}");
                    break;
                }
            }
        }
        if dims.0 == 0 {
            continue;
        }
        let px = f64::from(dims.0) * f64::from(dims.1);
        println!(
            "{name:<22} {:>11} {best:>7.2} ms {:>8.1} Mpx/s",
            format!("{}x{}", dims.0, dims.1),
            px / 1e6 / (best / 1000.0)
        );
    }
}
