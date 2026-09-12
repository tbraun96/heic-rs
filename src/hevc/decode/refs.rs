//! Reference sample gathering and substitution (clauses 8.4.4.2.1 and .2.2).

use super::state::Dec;
use crate::hevc::intra::Refs;

/// Collects the neighbouring samples of a transform block and substitutes
/// the unavailable ones.
///
/// `x` and `y` are in the coordinates of component `c_idx`, `n` is `nTbS`.
pub fn gather_refs(d: &Dec<'_>, x: usize, y: usize, n: usize, c_idx: usize) -> Refs {
    let (sw, sh) = if c_idx == 0 {
        (1usize, 1usize)
    } else {
        (d.sps.sub_w, d.sps.sub_h)
    };
    let plane = match c_idx {
        0 => &d.pic.y,
        1 => &d.pic.cb,
        _ => &d.pic.cr,
    };
    let bit_depth = if c_idx == 0 {
        d.sps.bit_depth_y
    } else {
        d.sps.bit_depth_c
    };
    let (xc, yc) = (x * sw, y * sh);
    let total = 4 * n + 1;
    let mut r = Refs::new(n);
    let mut avail = [false; 129];
    // Index 0 is p[-1][2n-1]; index 2n the corner; index 4n is p[2n-1][-1].
    for (i, a) in avail.iter_mut().enumerate().take(total) {
        let (cx, cy) = if i <= 2 * n {
            (-1isize, 2 * n as isize - 1 - i as isize)
        } else {
            ((i - 2 * n - 1) as isize, -1isize)
        };
        let nx = x as isize + cx;
        let ny = y as isize + cy;
        if nx < 0 || ny < 0 || nx as usize >= plane.width || ny as usize >= plane.height {
            continue;
        }
        let lx = (x as isize + cx) * sw as isize;
        let ly = (y as isize + cy) * sh as isize;
        if !d.available(xc, yc, lx, ly) {
            continue;
        }
        if d.pps.constrained_intra_pred {
            // Every coding unit in an intra-only picture is an intra CU, so the
            // constrained intra prediction check can never remove a sample here.
        }
        *a = true;
        r.buf[i] = plane.at(nx as usize, ny as usize);
    }
    substitute(&mut r, &avail, total, bit_depth);
    r
}

/// Reference sample substitution (clause 8.4.4.2.2).
fn substitute(r: &mut Refs, avail: &[bool; 129], total: usize, bit_depth: u8) {
    if avail[..total].iter().all(|&a| !a) {
        let mid = 1u16 << (bit_depth - 1);
        r.buf[..total].fill(mid);
        return;
    }
    if !avail[0] {
        // Search from p[-1][2n-1] upwards, then along the top row.
        if let Some(i) = avail[..total].iter().position(|&a| a) {
            r.buf[0] = r.buf[i];
        }
    }
    let mut last = r.buf[0];
    for (v, &a) in r.buf[..total].iter_mut().zip(avail[..total].iter()) {
        if a {
            last = *v;
        } else {
            *v = last;
        }
    }
}
