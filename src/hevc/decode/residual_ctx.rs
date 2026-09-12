//! Context index derivations for residual coding (clauses 9.3.4.2.3 to .2.7).

use super::state::Dec;
use crate::hevc::cabac::off;
use crate::hevc::error::{Error, Result};

/// `ctxIdxMap` for `sig_coeff_flag` in a 4x4 transform block (Table 9-39).
const SIG_MAP_4X4: [u8; 16] = [0, 1, 4, 5, 2, 3, 4, 5, 6, 6, 8, 8, 7, 7, 8, 8];

/// Derives `scanIdx` (clause 7.4.9.11): 0 diagonal, 1 horizontal, 2 vertical.
pub fn scan_index(d: &Dec<'_>, log2_size: usize, c_idx: usize, pred_mode: u8) -> usize {
    let mode_dependent =
        log2_size == 2 || (log2_size == 3 && c_idx == 0) || (log2_size == 3 && d.cat == 3);
    if !mode_dependent {
        return 0;
    }
    if (6..=14).contains(&pred_mode) {
        2
    } else if (22..=30).contains(&pred_mode) {
        1
    } else {
        0
    }
}

/// Decodes `last_sig_coeff_x/y_prefix` and their suffixes (clause 9.3.3.10).
pub fn last_position(d: &mut Dec<'_>, log2_size: usize, c_idx: usize) -> Result<(usize, usize)> {
    let (ctx_offset, ctx_shift) = if c_idx == 0 {
        (
            3 * (log2_size - 2) + ((log2_size - 1) >> 2),
            (log2_size + 1) >> 2,
        )
    } else {
        (15, log2_size - 2)
    };
    let c_max = (log2_size << 1) - 1;
    let mut prefix = [0usize; 2];
    for (k, base) in [off::LAST_X, off::LAST_Y].into_iter().enumerate() {
        let mut v = 0usize;
        while v < c_max {
            let ctx = base + ctx_offset + (v >> ctx_shift);
            if d.cab.decision(ctx)? == 0 {
                break;
            }
            v += 1;
        }
        prefix[k] = v;
    }
    let mut out = [0usize; 2];
    for k in 0..2 {
        out[k] = if prefix[k] > 3 {
            let bits = (prefix[k] >> 1) - 1;
            let suffix = d.cab.bypass_bits(bits as u32)? as usize;
            ((1usize << bits) * (2 + (prefix[k] & 1))) + suffix
        } else {
            prefix[k]
        };
    }
    Ok((out[0], out[1]))
}

/// Derives `ctxInc` for `sig_coeff_flag` (clause 9.3.4.2.5).
#[allow(clippy::too_many_arguments)]
pub fn sig_ctx(
    d: &Dec<'_>,
    log2_size: usize,
    c_idx: usize,
    xc: usize,
    yc: usize,
    xs: usize,
    ys: usize,
    sb_n: usize,
    scan_idx: usize,
    ts_ctx: bool,
) -> usize {
    if ts_ctx {
        return if c_idx == 0 { 42 } else { 43 };
    }
    let mut sig_ctx = if log2_size == 2 {
        SIG_MAP_4X4[(yc << 2) + xc] as usize
    } else if xc + yc == 0 {
        0
    } else {
        let mut prev = 0usize;
        if xs + 1 < sb_n {
            prev += d.csbf[ys * sb_n + xs + 1] as usize;
        }
        if ys + 1 < sb_n {
            prev += (d.csbf[(ys + 1) * sb_n + xs] as usize) << 1;
        }
        let xp = xc & 3;
        let yp = yc & 3;
        let base = match prev {
            0 => {
                if xp + yp == 0 {
                    2
                } else if xp + yp < 3 {
                    1
                } else {
                    0
                }
            }
            1 => {
                if yp == 0 {
                    2
                } else if yp == 1 {
                    1
                } else {
                    0
                }
            }
            2 => {
                if xp == 0 {
                    2
                } else if xp == 1 {
                    1
                } else {
                    0
                }
            }
            _ => 2,
        };
        let mut v = base;
        if c_idx == 0 {
            if xs + ys > 0 {
                v += 3;
            }
            v += if log2_size == 3 {
                if scan_idx == 0 { 9 } else { 15 }
            } else {
                21
            };
        } else {
            v += if log2_size == 3 { 9 } else { 12 };
        }
        v
    };
    if c_idx > 0 {
        sig_ctx += 27;
    }
    sig_ctx
}

/// Decodes `coeff_abs_level_remaining` (clause 9.3.3.11).
pub fn read_remaining(d: &mut Dec<'_>, rice: u32) -> Result<u32> {
    let mut prefix = 0u32;
    while d.cab.bypass()? == 1 {
        prefix += 1;
        if prefix > 24 {
            return Err(Error::InvalidData(
                "coeff_abs_level_remaining prefix too long",
            ));
        }
    }
    if prefix < 3 {
        let suffix = d.cab.bypass_bits(rice)?;
        Ok((prefix << rice) + suffix)
    } else {
        let extra = prefix - 3;
        let suffix = d.cab.bypass_bits(extra + rice)?;
        Ok(suffix + (((1u32 << extra) + 3 - 1) << rice))
    }
}
