//! Criterion benchmarks for the parts of the pipeline that exist today.
//!
//! Three groups:
//!
//! - `container_parse` — `probe` over a real HEIC, which walks every box,
//!   resolves the primary item and reads its properties, without decoding.
//! - `grid_compose` — stitching synthetic tiles into one frame, the step that
//!   dominates a large photograph once the codec is fast.
//! - `color_convert` — YCbCr to RGB with chroma upsampling.
//!
//! - `color_threads` — the same conversion serial and on the pool, which is
//!   what picks `ROWS_PER_BAND` and the floor below which colour stays on the
//!   calling thread.
//! - `decode_file` — the whole thing, bytes in and RGB8 out, over the
//!   fixtures, serial and on the pool.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use heic_rs::hevc::{ChromaFormat, Frame};
use heic_rs::image::PixelLayout;

fn fixture(name: &str) -> Option<Vec<u8>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
    std::fs::read(format!("{path}{name}")).ok()
}

fn synthetic(width: u32, height: u32) -> Frame {
    frame(width, height, ChromaFormat::Yuv420, 8)
}

/// A frame of non-constant samples, so nothing folds away.
fn frame(width: u32, height: u32, chroma: ChromaFormat, depth: u8) -> Frame {
    let (cw, ch) = chroma.chroma_size(width, height);
    let top = (1u32 << depth) - 1;
    Frame {
        width,
        height,
        bit_depth: depth,
        chroma,
        y: (0..width * height).map(|i| (i % top) as u16).collect(),
        cb: (0..cw * ch).map(|i| (i % top) as u16).collect(),
        cr: (0..cw * ch).map(|i| (i % (top - 55)) as u16).collect(),
        y_stride: width,
        c_stride: cw,
    }
}

fn container_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("container_parse");
    for name in ["flat-64.heic", "checker-1024.heic", "photo-2048.heic"] {
        let Some(bytes) = fixture(name) else { continue };
        group.bench_function(name, |b| {
            b.iter(|| heic_rs::probe(std::hint::black_box(&bytes)).map(|i| i.width))
        });
    }
    group.finish();
}

fn grid_compose(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_compose");
    for (rows, cols, w, h, label) in [
        (2u32, 2u32, 1024u32, 1024u32, "2x2 -> 1024x1024"),
        (3, 4, 2048, 1536, "3x4 -> 2048x1536"),
        (6, 8, 4032, 3024, "6x8 -> 4032x3024"),
    ] {
        let grid = heic_rs::Grid {
            rows,
            columns: cols,
            output_width: w,
            output_height: h,
        };
        let tiles: Vec<Frame> = (0..rows * cols).map(|_| synthetic(512, 512)).collect();
        group.bench_function(label, |b| {
            b.iter(|| heic_rs::grid::compose(&grid, std::hint::black_box(&tiles), u64::MAX))
        });
    }
    group.finish();
}

/// Every size, sampling, depth and layout the colour step is expected to be
/// fast at. Throughput is set to the pixel count, so criterion reports the
/// figure the README quotes directly.
fn color_convert(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_convert");
    let nclx = heic_rs::Nclx::default();
    let layouts = [PixelLayout::Rgb8, PixelLayout::Rgba8, PixelLayout::Gray8];
    for (w, h) in [(512u32, 512u32), (2048, 1536), (4032, 3024)] {
        group.throughput(Throughput::Elements(u64::from(w) * u64::from(h)));
        for (chroma, tag) in [
            (ChromaFormat::Yuv420, "4:2:0"),
            (ChromaFormat::Yuv444, "4:4:4"),
        ] {
            for depth in [8u8, 10] {
                let f = frame(w, h, chroma, depth);
                for layout in layouts {
                    let name = format!("{w}x{h} {tag} {depth}bit {layout:?}");
                    group.bench_function(name, |b| {
                        b.iter(|| {
                            heic_rs::color::convert(
                                std::hint::black_box(&f),
                                None,
                                nclx,
                                layout,
                                u64::MAX,
                                None,
                            )
                        })
                    });
                }
            }
        }
    }
    // The 16-bit output, at the two shapes the previous table published it at,
    // so that its before and after are the same case.
    for (w, h) in [(512u32, 512u32), (2048, 1536)] {
        let f = frame(w, h, ChromaFormat::Yuv420, 8);
        group.throughput(Throughput::Elements(u64::from(w) * u64::from(h)));
        group.bench_function(format!("{w}x{h} 4:2:0 8bit Rgb16"), |b| {
            b.iter(|| {
                heic_rs::color::convert(
                    std::hint::black_box(&f),
                    None,
                    nclx,
                    PixelLayout::Rgb16,
                    u64::MAX,
                    None,
                )
            })
        });
    }
    group.finish();
}

/// Serial against pooled, at the sizes where the answer changes.
fn color_threads(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_threads");
    let nclx = heic_rs::Nclx::default();
    for (w, h) in [(512u32, 512u32), (1024, 1024), (2048, 1536), (4032, 3024)] {
        let f = frame(w, h, ChromaFormat::Yuv420, 8);
        group.throughput(Throughput::Elements(u64::from(w) * u64::from(h)));
        for (tag, threads) in [("serial", Some(1)), ("pool", None)] {
            group.bench_function(format!("{w}x{h} {tag}"), |b| {
                b.iter(|| {
                    heic_rs::color::convert(
                        std::hint::black_box(&f),
                        None,
                        nclx,
                        PixelLayout::Rgb8,
                        u64::MAX,
                        threads,
                    )
                })
            });
        }
    }
    group.finish();
}

/// The number a caller actually waits for: a file in, RGB8 out.
fn decode_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_file");
    for name in ["gradient-512.heic", "checker-1024.heic", "photo-2048.heic"] {
        let Some(bytes) = fixture(name) else { continue };
        let Ok(info) = heic_rs::probe(&bytes) else {
            continue;
        };
        group.throughput(Throughput::Elements(
            u64::from(info.width) * u64::from(info.height),
        ));
        for (tag, threads) in [("serial", Some(1)), ("pool", None)] {
            let options = heic_rs::DecodeOptions::default()
                .with_layout(PixelLayout::Rgb8)
                .with_threads(threads);
            group.bench_function(format!("{name} {tag}"), |b| {
                b.iter(|| heic_rs::decode(std::hint::black_box(&bytes), &options))
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    container_parse,
    grid_compose,
    color_convert,
    color_threads,
    decode_file
);
criterion_main!(benches);
