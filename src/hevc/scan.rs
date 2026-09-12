//! Residual scan orders (clause 6.5.3) generated at compile time.
//!
//! `scanIdx` follows the specification: 0 = up-right diagonal, 1 = horizontal,
//! 2 = vertical. Entries are `[xC, yC]` pairs in scan order.

/// Up-right diagonal scan for an `s` x `s` block, padded to 64 entries.
const fn diag(s: usize) -> [[u8; 2]; 64] {
    let mut out = [[0u8; 2]; 64];
    let mut i = 0usize;
    let mut x: isize = 0;
    let mut y: isize = 0;
    let si = s as isize;
    loop {
        while y >= 0 {
            if x < si && y < si {
                out[i] = [x as u8, y as u8];
                i += 1;
            }
            y -= 1;
            x += 1;
        }
        y = x;
        x = 0;
        if i >= s * s {
            break;
        }
    }
    out
}

/// Horizontal scan for an `s` x `s` block, padded to 64 entries.
const fn horiz(s: usize) -> [[u8; 2]; 64] {
    let mut out = [[0u8; 2]; 64];
    let mut y = 0usize;
    while y < s {
        let mut x = 0usize;
        while x < s {
            out[y * s + x] = [x as u8, y as u8];
            x += 1;
        }
        y += 1;
    }
    out
}

/// Vertical scan for an `s` x `s` block, padded to 64 entries.
const fn vert(s: usize) -> [[u8; 2]; 64] {
    let mut out = [[0u8; 2]; 64];
    let mut x = 0usize;
    while x < s {
        let mut y = 0usize;
        while y < s {
            out[x * s + y] = [x as u8, y as u8];
            y += 1;
        }
        x += 1;
    }
    out
}

/// `SCANS[log2BlockSize][scanIdx]`, for block sizes 1, 2, 4 and 8.
pub static SCANS: [[[[u8; 2]; 64]; 3]; 4] = [
    [diag(1), horiz(1), vert(1)],
    [diag(2), horiz(2), vert(2)],
    [diag(4), horiz(4), vert(4)],
    [diag(8), horiz(8), vert(8)],
];

/// Returns the scan order table for a block of `1 << log2_size` samples a side.
#[inline]
pub fn scan_order(log2_size: usize, scan_idx: usize) -> &'static [[u8; 2]; 64] {
    &SCANS[log2_size][scan_idx]
}

/// The 4x4 sub-block scan (`log2_size == 2`) used inside every transform block.
#[inline]
pub fn sub_scan(scan_idx: usize) -> &'static [[u8; 2]; 64] {
    &SCANS[2][scan_idx]
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
