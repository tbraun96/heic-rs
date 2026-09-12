//! Throughput benchmarks for the hot stages inside the HEVC decoder.
//!
//! These reach past the public API into `heic_rs::hevc::bench_api`, which only
//! exists under the `bench` feature and is not public API:
//!
//! ```text
//! cargo bench --features bench --bench hevc
//! ```
//!
//! `benches/decode.rs` measures whole files; this measures the four stages
//! that dominate them, so that a regression can be attributed rather than
//! merely noticed.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use heic_rs::hevc::bench_api;

const BINS: usize = 200_000;

fn cabac(c: &mut Criterion) {
    let data = bench_api::encode_bins(BINS, 26);
    let mut g = c.benchmark_group("cabac");
    g.throughput(Throughput::Elements(BINS as u64));
    g.bench_function("decode_bins", |b| {
        b.iter(|| bench_api::decode_bins(black_box(&data), BINS, 26))
    });
    g.finish();
}

fn transforms(c: &mut Criterion) {
    let mut g = c.benchmark_group("inverse_transform");
    for n in [4usize, 8, 16, 32] {
        let mut src = vec![0i32; n * n];
        let mut s = 0x1234_5678u32;
        for v in src.iter_mut() {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            *v = (s as i32 % 2048) - 1024;
        }
        g.throughput(Throughput::Elements((n * n) as u64));
        g.bench_function(format!("{n}x{n}"), |b| {
            let mut block = src.clone();
            b.iter(|| {
                block.copy_from_slice(&src);
                bench_api::dequant_and_transform(black_box(&mut block), n, 30, 8);
            })
        });
    }
    g.finish();
}

fn intra(c: &mut Criterion) {
    let mut g = c.benchmark_group("intra_prediction");
    for (name, mode) in [
        ("planar", 0u8),
        ("dc", 1),
        ("angular_33", 33),
        ("angular_18", 18),
    ] {
        for n in [4usize, 32] {
            let refs = bench_api::ramp_refs(n);
            let mut out = vec![0u16; n * n];
            g.throughput(Throughput::Elements((n * n) as u64));
            g.bench_function(format!("{name}_{n}x{n}"), |b| {
                b.iter(|| bench_api::filter_and_predict(&refs, mode, 8, black_box(&mut out), n))
            });
        }
    }
    g.finish();
}

fn whole_picture(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode_still");
    for (w, h) in [(256u32, 256u32), (512, 512)] {
        let (sets_owned, slice) = heic_rs::hevc::synth::picture(w, h);
        let sets: Vec<&[u8]> = sets_owned.iter().map(|v| v.as_slice()).collect();
        g.throughput(Throughput::Elements(u64::from(w) * u64::from(h)));
        g.bench_function(format!("synthetic_{w}x{h}"), |b| {
            b.iter(|| heic_rs::hevc::decode_still(black_box(&sets), &[&slice]))
        });
    }
    g.finish();
}

criterion_group!(benches, cabac, transforms, intra, whole_picture);
criterion_main!(benches);
