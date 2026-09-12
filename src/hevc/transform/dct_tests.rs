//! Tests for clause 8.6.4.2 / 8.6.2, comparing the monomorphised fast path
//! against a slow reference written straight from the specification text.

use super::*;
use core::f64::consts::PI;

/// xorshift32, so the tests need no `rand` dependency.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// A coefficient uniformly in `coeffMin..=coeffMax`.
    fn coeff(&mut self) -> i32 {
        (self.next_u32() % 65536) as i32 - 32768
    }
}

/// The `nTbS`-point matrix, spelled out as the spec defines it.
fn ref_matrix(n: usize, tr_type: u8) -> Vec<Vec<i64>> {
    let mut m = vec![vec![0i64; n]; n];
    for k in 0..n {
        for j in 0..n {
            m[k][j] = if tr_type == 1 {
                DST_MATRIX[k][j] as i64
            } else {
                DCT_MATRIX[k * (32 / n)][j] as i64
            };
        }
    }
    m
}

/// Slow, literal transcription of clauses 8.6.4.2 and 8.6.2.
fn ref_inverse(d: &[i32], n: usize, tr_type: u8, bit_depth: u8) -> Vec<i32> {
    let m = ref_matrix(n, tr_type);
    let mut g = vec![0i64; n * n];
    for x in 0..n {
        for i in 0..n {
            let mut s = 0i64;
            for j in 0..n {
                s += m[j][i] * d[j * n + x] as i64;
            }
            let v = (s + 64) >> 7;
            g[i * n + x] = v.clamp(-32768, 32767);
        }
    }
    let bd = 20i64 - bit_depth as i64;
    let mut r = vec![0i32; n * n];
    for y in 0..n {
        for i in 0..n {
            let mut s = 0i64;
            for j in 0..n {
                s += m[j][i] * g[y * n + j];
            }
            r[y * n + i] = ((s + (1 << (bd - 1))) >> bd) as i32;
        }
    }
    r
}

/// Slow, literal transcription of the transform-skip path of clause 8.6.2.
fn ref_skip(d: &[i32], n: usize, bit_depth: u8, rotate: bool) -> Vec<i32> {
    let ts = 5 + n.trailing_zeros() as i64;
    let bd = 20i64 - bit_depth as i64;
    let mut r = vec![0i32; n * n];
    for y in 0..n {
        for x in 0..n {
            let s: i64 = if rotate {
                d[(n - 1 - y) * n + (n - 1 - x)] as i64
            } else {
                d[y * n + x] as i64
            };
            r[y * n + x] = (((s << ts) + (1 << (bd - 1))) >> bd) as i32;
        }
    }
    r
}

fn check(d: &[i32], n: usize, tr_type: u8, bit_depth: u8, what: &str) {
    let expected = ref_inverse(d, n, tr_type, bit_depth);
    let mut got = d.to_vec();
    inverse_transform(&mut got, n, tr_type, bit_depth);
    assert_eq!(got, expected, "{what}: n={n} tr={tr_type} bd={bit_depth}");
}

#[test]
fn fast_path_matches_reference_on_random_blocks() {
    let mut rng = Rng::new(0x1234_5678);
    for &n in &[4usize, 8, 16, 32] {
        for &tr_type in &[0u8, 1] {
            if tr_type == 1 && n != 4 {
                continue;
            }
            for &bit_depth in &[8u8, 10] {
                for _ in 0..200 {
                    let d: Vec<i32> = (0..n * n).map(|_| rng.coeff()).collect();
                    check(&d, n, tr_type, bit_depth, "random");
                }
            }
        }
    }
}

#[test]
fn fast_path_matches_reference_on_edge_blocks() {
    for &n in &[4usize, 8, 16, 32] {
        for &tr_type in &[0u8, 1] {
            if tr_type == 1 && n != 4 {
                continue;
            }
            for &bit_depth in &[8u8, 10] {
                check(&vec![0i32; n * n], n, tr_type, bit_depth, "zero");
                check(&vec![-32768i32; n * n], n, tr_type, bit_depth, "coeff_min");
                check(&vec![32767i32; n * n], n, tr_type, bit_depth, "coeff_max");
                let mut dc = vec![0i32; n * n];
                dc[0] = 32767;
                check(&dc, n, tr_type, bit_depth, "dc_only");
                let mut hi = vec![0i32; n * n];
                hi[n * n - 1] = -32768;
                check(&hi, n, tr_type, bit_depth, "highest_freq");
            }
        }
    }
}

#[test]
fn dct_matrix_matches_float_basis() {
    // 64 * sqrt(2), the gain of the HEVC integer basis.
    const SCALE: f64 = 90.50966799187808;
    for (n, &v) in DCT_MATRIX[0].iter().enumerate() {
        assert_eq!(v, 64, "row 0 must be flat at n={n}");
    }
    for (k, row) in DCT_MATRIX.iter().enumerate().skip(1) {
        for (n, &entry) in row.iter().enumerate() {
            let ideal = SCALE * (PI * k as f64 * (2 * n + 1) as f64 / 64.0).cos();
            let err = (entry as f64 - ideal).abs();
            // The standard's table is not simply the ideal basis rounded: the
            // two magnitudes 36 and 25 sit ~1.36 and ~1.27 away from it (the
            // ideal values are 34.64 and 26.27), a deliberate adjustment that
            // keeps the integer matrix close to orthogonal. Every other entry
            // is within one of the ideal value.
            let tol = if entry.abs() == 36 || entry.abs() == 25 {
                1.4
            } else {
                1.0
            };
            assert!(
                err <= tol,
                "k={k} n={n} entry={entry} ideal={ideal} err={err}"
            );
        }
    }
}

#[test]
fn row_subsampling_gives_the_textbook_small_matrices() {
    let m4: Vec<Vec<i16>> = (0..4)
        .map(|k| (0..4).map(|n| DCT_MATRIX[k * 8][n]).collect())
        .collect();
    assert_eq!(m4[0], vec![64, 64, 64, 64]);
    assert_eq!(m4[1], vec![83, 36, -36, -83]);
    assert_eq!(m4[2], vec![64, -64, -64, 64]);
    assert_eq!(m4[3], vec![36, -83, 83, -36]);

    let m8row = |k: usize| -> Vec<i16> { (0..8).map(|n| DCT_MATRIX[k * 4][n]).collect() };
    assert_eq!(m8row(1), vec![89, 75, 50, 18, -18, -50, -75, -89]);
    assert_eq!(m8row(3), vec![75, -18, -89, -50, 50, 89, 18, -75]);
}

#[test]
fn dc_only_coefficient_gives_a_flat_residual() {
    for &n in &[4usize, 8, 16, 32] {
        for &v in &[1i32, -1, 100, -4096, 32767, -32768] {
            let mut b = vec![0i32; n * n];
            b[0] = v;
            inverse_transform(&mut b, n, 0, 8);
            let g = ((64 * v + 64) >> 7).clamp(-32768, 32767);
            let expected = (64 * g + (1 << 11)) >> 12;
            for (i, &s) in b.iter().enumerate() {
                assert_eq!(s, expected, "n={n} v={v} i={i}");
            }
        }
    }
}

#[test]
fn transform_skip_matches_reference() {
    let mut rng = Rng::new(0x9E37_79B9);
    for &n in &[4usize, 8, 16, 32] {
        for &bit_depth in &[8u8, 10] {
            for &rotate in &[false, true] {
                for _ in 0..50 {
                    let d: Vec<i32> = (0..n * n).map(|_| rng.coeff()).collect();
                    let expected = ref_skip(&d, n, bit_depth, rotate);
                    let mut got = d.clone();
                    transform_skip(&mut got, n, bit_depth, rotate);
                    assert_eq!(got, expected, "n={n} bd={bit_depth} rot={rotate}");
                }
            }
        }
    }
}

#[test]
fn invalid_arguments_leave_the_block_untouched() {
    let original = vec![7i32; 64];
    for &(n, tr, bd, len) in &[
        (5usize, 0u8, 8u8, 64usize),
        (8, 0, 7, 64),
        (8, 0, 17, 64),
        (8, 1, 8, 64),
        (8, 0, 8, 63),
    ] {
        let mut b = original[..len].to_vec();
        inverse_transform(&mut b, n, tr, bd);
        assert_eq!(
            b,
            &original[..len],
            "inverse n={n} tr={tr} bd={bd} len={len}"
        );
    }
    for &(n, bd, len) in &[(5usize, 8u8, 64usize), (8, 7, 64), (8, 17, 64), (8, 8, 63)] {
        let mut c = original[..len].to_vec();
        transform_skip(&mut c, n, bd, false);
        assert_eq!(c, &original[..len], "skip n={n} bd={bd} len={len}");
    }
}
