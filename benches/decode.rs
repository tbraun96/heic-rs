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
//! There is deliberately no end-to-end decode benchmark: the codec seam is
//! still a placeholder, so such a number would measure nothing.

use criterion::{Criterion, criterion_group, criterion_main};
use heic_rs::hevc::{ChromaFormat, Frame};
use heic_rs::image::PixelLayout;

fn fixture(name: &str) -> Option<Vec<u8>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
    std::fs::read(format!("{path}{name}")).ok()
}

fn synthetic(width: u32, height: u32) -> Frame {
    let (cw, ch) = ChromaFormat::Yuv420.chroma_size(width, height);
    Frame {
        width,
        height,
        bit_depth: 8,
        chroma: ChromaFormat::Yuv420,
        y: (0..width * height).map(|i| (i % 255) as u16).collect(),
        cb: (0..cw * ch).map(|i| (i % 255) as u16).collect(),
        cr: (0..cw * ch).map(|i| (i % 200) as u16).collect(),
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

fn color_convert(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_convert");
    let nclx = heic_rs::Nclx::default();
    for (w, h) in [(512u32, 512u32), (2048, 1536)] {
        let frame = synthetic(w, h);
        for layout in [PixelLayout::Rgb8, PixelLayout::Rgba8, PixelLayout::Rgb16] {
            group.bench_function(format!("{w}x{h} {layout:?}"), |b| {
                b.iter(|| {
                    heic_rs::color::convert(
                        std::hint::black_box(&frame),
                        None,
                        nclx,
                        layout,
                        u64::MAX,
                    )
                })
            });
        }
    }
    group.finish();
}

criterion_group!(benches, container_parse, grid_compose, color_convert);
criterion_main!(benches);
