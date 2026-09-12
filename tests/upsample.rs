//! The chroma upsampler's two forms agree: `row`, which expands and blends
//! two chroma rows in one pass, is exactly `vblend` over `horizontal` of each.

use heic_rs::upsample::{horizontal, row, vblend};

/// A deterministic, non-repeating sample row.
fn samples(n: usize, seed: u32) -> Vec<u16> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (x >> 8) as u16
        })
        .collect()
}

#[test]
fn row_is_vblend_over_horizontal() {
    for x_shift in [0u32, 1] {
        for w0 in [3u32, 4, 1] {
            for n in [1usize, 2, 3, 17, 64] {
                let src0 = samples(n, 7 + n as u32);
                let src1 = samples(n, 99 + n as u32);
                // Output rows both exactly as wide as the expansion and wider,
                // so the edge clamp is exercised too.
                for w in [n << x_shift, (n << x_shift) + 3] {
                    let mut fused = vec![0u16; w];
                    row(&mut fused, &src0, &src1, w0, x_shift);
                    let (mut h0, mut h1) = (vec![0u16; w], vec![0u16; w]);
                    horizontal(&mut h0, &src0, x_shift);
                    horizontal(&mut h1, &src1, x_shift);
                    let split: Vec<u16> = h0
                        .iter()
                        .zip(&h1)
                        .map(|(a, b)| vblend(*a, *b, w0))
                        .collect();
                    assert_eq!(fused, split, "x_shift {x_shift} w0 {w0} n {n} w {w}");
                }
            }
        }
    }
}

#[test]
fn horizontal_copies_even_columns_and_averages_odd_ones() {
    let src = [10u16, 20, 40];
    let mut out = [0u16; 8];
    horizontal(&mut out, &src, 1);
    // Columns past the last chroma sample clamp to it.
    assert_eq!(out, [10, 15, 20, 30, 40, 40, 40, 40]);
    let mut same = [0u16; 5];
    horizontal(&mut same, &src, 0);
    assert_eq!(same, [10, 20, 40, 40, 40]);
    let mut none = [7u16; 0];
    horizontal(&mut none, &src, 1);
    let mut empty_src = [7u16; 4];
    horizontal(&mut empty_src, &[], 1);
    assert_eq!(empty_src, [7; 4]);
}
