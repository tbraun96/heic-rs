//! Level, sign and remainder decoding for one 4x4 residual sub-block.

use super::state::Dec;
use crate::hevc::cabac::off;
use crate::hevc::error::Result;
use crate::hevc::scan::sub_scan;

use super::residual_ctx::read_remaining;

/// Decodes the greater1, greater2, sign and remainder bins of one sub-block.
#[allow(clippy::too_many_arguments)]
pub fn decode_levels(
    d: &mut Dec<'_>,
    sig: &[bool; 16],
    i: usize,
    c_idx: usize,
    c1: &mut i32,
    sb_type: usize,
    log2_size: usize,
    scan_idx: usize,
    xs: usize,
    ys: usize,
) -> Result<()> {
    let mut ctx_set = if i > 0 && c_idx == 0 { 2 } else { 0 };
    if *c1 == 0 {
        ctx_set += 1;
    }
    *c1 = 1;
    let base = off::GT1 + ctx_set * 4 + if c_idx > 0 { 16 } else { 0 };
    let mut gt1 = [false; 16];
    let mut gt2 = [false; 16];
    let mut num_gt1 = 0usize;
    let mut last_gt1: i32 = -1;
    let mut first_sig: i32 = -1;
    let mut last_sig: i32 = -1;
    for m in (0..16).rev() {
        if !sig[m] {
            continue;
        }
        if num_gt1 < 8 {
            let bin = d.cab.decision(base + (*c1).min(3) as usize) != 0;
            gt1[m] = bin;
            num_gt1 += 1;
            if bin {
                *c1 = 0;
                if last_gt1 < 0 {
                    last_gt1 = m as i32;
                }
            } else if *c1 > 0 && *c1 < 3 {
                *c1 += 1;
            }
        }
        if last_sig < 0 {
            last_sig = m as i32;
        }
        first_sig = m as i32;
    }
    let sign_hidden = last_sig - first_sig > 3 && !d.tq_bypass;
    if last_gt1 >= 0 {
        let ctx = off::GT2 + ctx_set + if c_idx > 0 { 4 } else { 0 };
        gt2[last_gt1 as usize] = d.cab.decision(ctx) != 0;
    }
    let mut signs = [false; 16];
    for m in (0..16).rev() {
        if sig[m] && !(d.pps.sign_data_hiding && sign_hidden && m as i32 == first_sig) {
            signs[m] = d.cab.bypass() != 0;
        }
    }
    let mut rice = if d.sps.persistent_rice {
        (d.stat_coeff[sb_type] / 4) as u32
    } else {
        0
    };
    let mut first_rem = true;
    let mut num_sig = 0usize;
    let mut sum_abs = 0i64;
    let pos_scan = sub_scan(scan_idx);
    let n = 1usize << log2_size;
    for m in (0..16).rev() {
        if !sig[m] {
            continue;
        }
        let base_level = 1 + i32::from(gt1[m]) + i32::from(gt2[m]);
        let threshold = if num_sig < 8 {
            if m as i32 == last_gt1 { 3 } else { 2 }
        } else {
            1
        };
        let mut level = base_level as i64;
        if base_level == threshold {
            let rem = read_remaining(d, rice)?;
            if first_rem && d.sps.persistent_rice {
                update_stat(d, sb_type, rem);
            }
            first_rem = false;
            level += rem as i64;
            if level > 3i64 << rice {
                rice = (rice + 1).min(4);
            }
        } else if level > 3i64 << rice {
            rice = (rice + 1).min(4);
        }
        sum_abs += level;
        let mut v = level;
        if signs[m] {
            v = -v;
        }
        if d.pps.sign_data_hiding && sign_hidden && m as i32 == first_sig && sum_abs % 2 == 1 {
            v = -v;
        }
        let xc = xs * 4 + pos_scan[m][0] as usize;
        let yc = ys * 4 + pos_scan[m][1] as usize;
        d.coeffs[yc * n + xc] = v.clamp(-32768, 32767) as i32;
        num_sig += 1;
    }
    Ok(())
}

/// Applies the `StatCoeff` update of clause 9.3.3.11.
fn update_stat(d: &mut Dec<'_>, sb_type: usize, rem: u32) {
    let s = d.stat_coeff[sb_type];
    if rem as i64 >= 3i64 << (s / 4) {
        d.stat_coeff[sb_type] = s + 1;
    } else if (2 * rem as i64) < (1i64 << (s / 4)) && s > 0 {
        d.stat_coeff[sb_type] = s - 1;
    }
}
