//! Intra sample prediction: reference filtering (8.4.4.2.3) and the planar,
//! DC and angular predictors (8.4.4.2.5 and 8.4.4.2.6).

use super::Refs;

/// `intraPredAngle` indexed by `predModeIntra - 2`, for modes 2 to 34.
const ANG: [i32; 33] = [
    32, 26, 21, 17, 13, 9, 5, 2, 0, -2, -5, -9, -13, -17, -21, -26, -32, -26, -21, -17, -13, -9,
    -5, -2, 0, 2, 5, 9, 13, 17, 21, 26, 32,
];

/// `invAngle` indexed by `predModeIntra - 11`, for the modes 11 to 25 that
/// have a negative `intraPredAngle`.
const INV: [i32; 15] = [
    -4096, -1638, -910, -630, -482, -390, -315, -256, -315, -390, -482, -630, -910, -1638, -4096,
];

/// Filters the neighbouring reference samples (clause 8.4.4.2.3).
///
/// `mode` is predModeIntra (0 planar, 1 DC, 2..=34 angular), `c_idx` 0 for luma,
/// `chroma_array_type` is 0..=3, `strong_smoothing` is
/// `strong_intra_smoothing_enabled_flag`, `disable_smoothing` is
/// `intra_smoothing_disabled_flag` from the SPS range extension.
pub fn filter_refs(
    r: &Refs,
    mode: u8,
    c_idx: usize,
    bit_depth: u8,
    chroma_array_type: u8,
    strong_smoothing: bool,
    disable_smoothing: bool,
) -> Refs {
    let n = r.n;
    let nn = 2 * n as isize;
    if disable_smoothing || (c_idx != 0 && chroma_array_type != 3) {
        return r.clone();
    }
    let m = mode as i32;
    let thresh = match n {
        8 => 7,
        16 => 1,
        _ => 0,
    };
    let dist = core::cmp::min((m - 26).abs(), (m - 10).abs());
    if mode == 1 || mode > 34 || n == 4 || dist <= thresh {
        return r.clone();
    }
    let c = r.top(-1) as i32;
    let (te, le) = (r.top(nn - 1) as i32, r.left(nn - 1) as i32);
    // `1 << (bit_depth - 5)`, written so a nonsensical small bit depth cannot
    // underflow the shift amount.
    let lim = (1i32 << bit_depth) >> 5;
    let bilin = strong_smoothing
        && c_idx == 0
        && n == 32
        && (c + te - 2 * r.top(nn / 2 - 1) as i32).abs() < lim
        && (c + le - 2 * r.left(nn / 2 - 1) as i32).abs() < lim;
    let mut o = Refs::new(n);
    if bilin {
        o.set_top(-1, c as u16);
        for i in 0..nn - 1 {
            let (w0, w1) = (nn - 1 - i, i + 1);
            o.set_left(i, (((w0 as i32) * c + (w1 as i32) * le + 32) >> 6) as u16);
            o.set_top(i, (((w0 as i32) * c + (w1 as i32) * te + 32) >> 6) as u16);
        }
    } else {
        o.set_top(
            -1,
            ((r.left(0) as i32 + 2 * c + r.top(0) as i32 + 2) >> 2) as u16,
        );
        for i in 0..nn - 1 {
            let l = r.left(i + 1) as i32 + 2 * r.left(i) as i32 + r.left(i - 1) as i32;
            let t = r.top(i + 1) as i32 + 2 * r.top(i) as i32 + r.top(i - 1) as i32;
            o.set_left(i, ((l + 2) >> 2) as u16);
            o.set_top(i, ((t + 2) >> 2) as u16);
        }
    }
    o.set_left(nn - 1, le as u16);
    o.set_top(nn - 1, te as u16);
    o
}

/// Produces the nTbS x nTbS prediction block (clauses 8.4.4.2.5 to 8.4.4.2.6).
///
/// `out` receives predSamples[x][y] at `out[y * out_stride + x]`.
pub fn predict(
    r: &Refs,
    mode: u8,
    c_idx: usize,
    bit_depth: u8,
    out: &mut [u16],
    out_stride: usize,
) {
    match mode {
        0 => planar(r, out, out_stride),
        2..=34 => angular(r, mode, c_idx, bit_depth, out, out_stride),
        _ => dc(r, c_idx, out, out_stride),
    }
}

/// Planar prediction, clause 8.4.4.2.5.
fn planar(r: &Refs, out: &mut [u16], st: usize) {
    let (n, ni) = (r.n, r.n as i32);
    let sh = n.trailing_zeros() + 1;
    let (tn, ln) = (r.top(ni as isize) as i32, r.left(ni as isize) as i32);
    let mut top = [0i32; 32];
    for (x, v) in top[..n].iter_mut().enumerate() {
        *v = r.top(x as isize) as i32;
    }
    for y in 0..n {
        let (l, yi) = (r.left(y as isize) as i32, y as i32);
        let (a, b) = ((ni - 1 - yi), (yi + 1) * ln + ni);
        let row = &mut out[y * st..y * st + n];
        for (x, (o, &t)) in row.iter_mut().zip(top[..n].iter()).enumerate() {
            let xi = x as i32;
            *o = (((ni - 1 - xi) * l + (xi + 1) * tn + a * t + b) >> sh) as u16;
        }
    }
}

/// DC prediction with the luma boundary smoothing, clause 8.4.4.2.6.
fn dc(r: &Refs, c_idx: usize, out: &mut [u16], st: usize) {
    let (n, ni) = (r.n, r.n as i32);
    let sh = n.trailing_zeros() + 1;
    let mut sum = ni;
    for i in 0..n as isize {
        sum += r.top(i) as i32 + r.left(i) as i32;
    }
    let d = sum >> sh;
    for y in 0..n {
        for o in out[y * st..y * st + n].iter_mut() {
            *o = d as u16;
        }
    }
    if c_idx == 0 && n < 32 {
        for (x, o) in out[..n].iter_mut().enumerate() {
            *o = ((r.top(x as isize) as i32 + 3 * d + 2) >> 2) as u16;
        }
        for y in 1..n {
            out[y * st] = ((r.left(y as isize) as i32 + 3 * d + 2) >> 2) as u16;
        }
        out[0] = ((r.left(0) as i32 + 2 * d + r.top(0) as i32 + 2) >> 2) as u16;
    }
}

/// Angular prediction for modes 2 to 34, clause 8.4.4.2.6.
fn angular(r: &Refs, mode: u8, c_idx: usize, bd: u8, out: &mut [u16], st: usize) {
    let (n, ni) = (r.n, r.n as i32);
    let a = ANG[mode as usize - 2];
    let vert = mode >= 18;
    // `rf[32 + i]` holds ref[i] for i in -32 ..= 64.
    let mut rf = [0i32; 97];
    let main = |i: isize| -> i32 { (if vert { r.top(i - 1) } else { r.left(i - 1) }) as i32 };
    let side = |i: isize| -> i32 { (if vert { r.left(i - 1) } else { r.top(i - 1) }) as i32 };
    for (i, v) in rf[32..33 + n].iter_mut().enumerate() {
        *v = main(i as isize);
    }
    if a < 0 {
        let lim = (ni * a) >> 5;
        if lim < -1 {
            let inv = INV[mode as usize - 11];
            let mut x = -1i32;
            while x >= lim {
                rf[(32 + x) as usize] = side(((x * inv + 128) >> 8) as isize);
                x -= 1;
            }
        }
    } else {
        for (i, v) in rf[33 + n..33 + 2 * n].iter_mut().enumerate() {
            *v = main((n + 1 + i) as isize);
        }
    }
    for m in 0..n {
        let t = (m as i32 + 1) * a;
        let (idx, f) = (t >> 5, t & 31);
        let b = (33 + idx) as usize;
        if vert {
            let row = &mut out[m * st..m * st + n];
            if f == 0 {
                for (o, &v) in row.iter_mut().zip(rf[b..b + n].iter()) {
                    *o = v as u16;
                }
            } else {
                let (s0, s1) = (&rf[b..b + n], &rf[b + 1..b + 1 + n]);
                for (o, (&v0, &v1)) in row.iter_mut().zip(s0.iter().zip(s1.iter())) {
                    *o = (((32 - f) * v0 + f * v1 + 16) >> 5) as u16;
                }
            }
        } else {
            for k in 0..n {
                let v = if f == 0 {
                    rf[b + k]
                } else {
                    ((32 - f) * rf[b + k] + f * rf[b + k + 1] + 16) >> 5
                };
                out[k * st + m] = v as u16;
            }
        }
    }
    if c_idx == 0 && n < 32 && (mode == 26 || mode == 10) {
        let (c, hi) = (r.top(-1) as i32, (1i32 << bd) - 1);
        if mode == 26 {
            let p = r.top(0) as i32;
            for y in 0..n {
                let v = p + ((r.left(y as isize) as i32 - c) >> 1);
                out[y * st] = v.max(0).min(hi) as u16;
            }
        } else {
            let p = r.left(0) as i32;
            for (x, o) in out[..n].iter_mut().enumerate() {
                let v = p + ((r.top(x as isize) as i32 - c) >> 1);
                *o = v.max(0).min(hi) as u16;
            }
        }
    }
}

#[cfg(test)]
#[path = "pred_tests.rs"]
mod tests;
