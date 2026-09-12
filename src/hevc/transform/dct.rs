//! Inverse DCT-II / DST-VII and transform-skip residual derivation,
//! ITU-T H.265 clause 8.6.4.2 ("Transformation process for scaled transform
//! coefficients") and the final bit-depth shift of clause 8.6.2.
//!
//! Written from the specification text alone. No code, table layout or
//! butterfly schedule was taken from any existing decoder.

/// Lower bound `coeffMin` on an intermediate transform coefficient (16 bit).
const COEFF_MIN: i32 = -32768;
/// Upper bound `coeffMax` on an intermediate transform coefficient (16 bit).
const COEFF_MAX: i32 = 32767;
/// Right shift applied between the two one-dimensional stages (clause 8.6.4.2).
const STAGE1_SHIFT: u32 = 7;
/// Lowest bit depth this module accepts; below it `bdShift` would be undefined.
const MIN_BIT_DEPTH: u8 = 8;
/// Highest bit depth this module accepts (H.265 range extensions cap).
const MAX_BIT_DEPTH: u8 = 16;

/// One quarter period of the integer cosine used by `transMatrix`, indexed by
/// the reduced angle `j` in units of pi/64. `CTAB[j]` is the magnitude of
/// `64 * sqrt(2) * cos(pi * j / 64)` as tabulated by the standard.
const CTAB: [i16; 33] = [
    0, 90, 90, 90, 89, 88, 87, 85, 83, 82, 80, 78, 75, 73, 70, 67, 64, 61, 57, 54, 50, 46, 43, 38,
    36, 31, 25, 22, 18, 13, 9, 4, 0,
];

/// Builds the 32x32 `transMatrix` of clause 8.6.4.2 from [`CTAB`] by folding
/// the angle `k * (2n + 1) * pi / 64` into the tabulated quarter period.
const fn build_dct() -> [[i16; 32]; 32] {
    let mut m = [[0i16; 32]; 32];
    let mut n = 0;
    while n < 32 {
        m[0][n] = 64;
        n += 1;
    }
    let mut k = 1;
    while k < 32 {
        let mut n = 0;
        while n < 32 {
            let mut j = (k * (2 * n + 1)) % 128;
            if j > 64 {
                j = 128 - j;
            }
            m[k][n] = if j <= 32 { CTAB[j] } else { -CTAB[64 - j] };
            n += 1;
        }
        k += 1;
    }
    m
}

/// The 4x4 DST-VII matrix of clause 8.6.4.2, the single source for
/// [`DST_MATRIX`] and for the `i32` copy used by the transform itself.
const DST_DATA: [[i16; 4]; 4] = [
    [29, 55, 74, 84],
    [74, 74, 0, -74],
    [84, -29, -74, 55],
    [55, -84, 74, -29],
];

/// The 32-point HEVC DCT-II integer matrix `transMatrix` of clause 8.6.4.2; the
/// `nTbS`-point matrix is the row sub-sampling `DCT_MATRIX[m * (32 / nTbS)][n]`
/// for `m`, `n` < `nTbS`. Materialised only for the tests that validate it: the
/// transform itself uses the `i32` copies produced by [`sub`].
#[cfg(test)]
pub static DCT_MATRIX: [[i16; 32]; 32] = build_dct();

/// The 4x4 HEVC DST-VII integer matrix of clause 8.6.4.2, used for the
/// 4x4 luma intra residual. Materialised only for the tests; see
/// [`DCT_MATRIX`].
#[cfg(test)]
pub static DST_MATRIX: [[i16; 4]; 4] = DST_DATA;

/// Row sub-samples [`build_dct`] down to the `N`-point matrix and widens it to
/// `i32` so the hot loops never sign-extend.
const fn sub<const N: usize>() -> [[i32; N]; N] {
    let full = build_dct();
    let step = 32 / N;
    let mut m = [[0i32; N]; N];
    let mut k = 0;
    while k < N {
        let mut n = 0;
        while n < N {
            m[k][n] = full[k * step][n] as i32;
            n += 1;
        }
        k += 1;
    }
    m
}

/// Widens [`DST_DATA`] to `i32` for the hot loops.
const fn dst_i32() -> [[i32; 4]; 4] {
    let mut m = [[0i32; 4]; 4];
    let mut k = 0;
    while k < 4 {
        let mut n = 0;
        while n < 4 {
            m[k][n] = DST_DATA[k][n] as i32;
            n += 1;
        }
        k += 1;
    }
    m
}

/// 4-point DCT-II matrix.
const M4: [[i32; 4]; 4] = sub::<4>();
/// 8-point DCT-II matrix.
const M8: [[i32; 8]; 8] = sub::<8>();
/// 16-point DCT-II matrix.
const M16: [[i32; 16]; 16] = sub::<16>();
/// 32-point DCT-II matrix.
const M32: [[i32; 32]; 32] = sub::<32>();
/// 4-point DST-VII matrix.
const MDST: [[i32; 4]; 4] = dst_i32();

/// `Clip3(coeffMin, coeffMax, v)` of clause 5, written without `clamp` so that
/// the transform keeps no panicking path at all.
#[allow(clippy::manual_clamp)]
fn clip_coeff(v: i32) -> i32 {
    if v < COEFF_MIN {
        COEFF_MIN
    } else if v > COEFF_MAX {
        COEFF_MAX
    } else {
        v
    }
}

/// `bdShift` of clause 8.6.2, or `None` when the bit depth is out of range.
const fn bd_shift_of(bit_depth: u8) -> Option<u32> {
    if bit_depth >= MIN_BIT_DEPTH && bit_depth <= MAX_BIT_DEPTH {
        Some(20 - bit_depth as u32)
    } else {
        None
    }
}

/// One 1-D inverse transform: `out[i] = sum_j m[j][i] * src[j]`.
///
/// The sum index is the *row* of `m`, i.e. this multiplies by the transpose,
/// exactly as clause 8.6.4.2 writes it. Accumulating over `j` on the outside
/// keeps the inner loop a contiguous multiply-accumulate over two slices.
///
/// The largest attainable magnitude is `32 * 32768 * 90 = 94_371_840`, well
/// inside `i32`, because both stages take 16-bit-clipped inputs.
fn mul_1d<const N: usize>(m: &[[i32; N]; N], src: &[i32; N]) -> [i32; N] {
    let mut acc = [0i32; N];
    for j in 0..N {
        let c = src[j];
        let row = &m[j];
        for i in 0..N {
            acc[i] += row[i] * c;
        }
    }
    acc
}

/// The two-dimensional inverse transform for one `N`x`N` block.
///
/// `block` is row-major with `d[x][y]` at `block[y * N + x]`; it is overwritten
/// with the residual `r[x][y]` at the same position.
fn two_stage<const N: usize>(block: &mut [i32], m: &[[i32; N]; N], bd_shift: u32) {
    let b = &mut block[..N * N];
    // g[y][x], the clipped output of the column stage.
    let mut g = [[0i32; N]; N];
    let mut col = [0i32; N];
    for x in 0..N {
        for j in 0..N {
            col[j] = b[j * N + x];
        }
        let e = mul_1d(m, &col);
        for y in 0..N {
            g[y][x] = clip_coeff((e[y] + (1 << (STAGE1_SHIFT - 1))) >> STAGE1_SHIFT);
        }
    }
    let rnd = 1i32 << (bd_shift - 1);
    for y in 0..N {
        let r = mul_1d(m, &g[y]);
        for i in 0..N {
            b[y * N + i] = (r[i] + rnd) >> bd_shift;
        }
    }
}

/// Applies the two-dimensional inverse transform in place (clause 8.6.4.2)
/// followed by the final `bdShift` of clause 8.6.2.
///
/// `block` holds the scaled transform coefficients d[x][y] at
/// `block[y * n + x]` and receives the residual samples r[x][y].
/// `tr_type` is 0 for DCT-II and 1 for the 4x4 DST-VII.
///
/// `block` is left untouched when `n` is not 4, 8, 16 or 32, when `tr_type` is
/// 1 and `n` is not 4, when `bit_depth` is outside 8..=16, or when `block` is
/// shorter than `n * n`.
pub fn inverse_transform(block: &mut [i32], n: usize, tr_type: u8, bit_depth: u8) {
    let bd_shift = match bd_shift_of(bit_depth) {
        Some(s) => s,
        None => return,
    };
    match (tr_type, n) {
        (1, 4) if block.len() >= 16 => two_stage::<4>(block, &MDST, bd_shift),
        (0, 4) if block.len() >= 16 => two_stage::<4>(block, &M4, bd_shift),
        (0, 8) if block.len() >= 64 => two_stage::<8>(block, &M8, bd_shift),
        (0, 16) if block.len() >= 256 => two_stage::<16>(block, &M16, bd_shift),
        (0, 32) if block.len() >= 1024 => two_stage::<32>(block, &M32, bd_shift),
        _ => {}
    }
}

/// Transform-skip residual derivation (clause 8.6.2) for one block.
///
/// `block` holds d[x][y] and receives r[x][y]. `rotate` is
/// `transform_skip_rotation_enabled_flag` (always false for Main profiles).
///
/// `block` is left untouched when `n` is not 4, 8, 16 or 32, when `bit_depth`
/// is outside 8..=16, or when `block` is shorter than `n * n`.
pub fn transform_skip(block: &mut [i32], n: usize, bit_depth: u8, rotate: bool) {
    let log2_n: u32 = match n {
        4 => 2,
        8 => 3,
        16 => 4,
        32 => 5,
        _ => return,
    };
    let bd_shift = match bd_shift_of(bit_depth) {
        Some(s) => s,
        None => return,
    };
    if block.len() < n * n {
        return;
    }
    let b = &mut block[..n * n];
    // Reading d[n-1-x][n-1-y] into r[x][y] is exactly a reversal of the
    // row-major block, since (n-1-y) * n + (n-1-x) == n * n - 1 - (y * n + x).
    if rotate {
        b.reverse();
    }
    let ts_shift = 5 + log2_n;
    let rnd = 1i32 << (bd_shift - 1);
    for v in b.iter_mut() {
        *v = ((*v << ts_shift) + rnd) >> bd_shift;
    }
}

#[cfg(test)]
#[path = "dct_tests.rs"]
mod tests;
