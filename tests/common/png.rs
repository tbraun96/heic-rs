//! A minimal PNG reader, for comparing our output against the reference
//! decodes in `tests/fixtures/*.ref.png`.
//!
//! It handles exactly what those files are: 8-bit truecolour (type 2),
//! non-interlaced, deflate-compressed. Anything else is rejected. The deflate
//! half lives in `inflate.rs`, so that the comparison needs no dependency and
//! no external tool.

use super::inflate::inflate;

/// A decoded 8-bit RGB image.
pub struct Png {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

pub fn decode(bytes: &[u8]) -> Result<Png, String> {
    if bytes.len() < 8 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("not a PNG".into());
    }
    let (mut pos, mut idat, mut dims) = (8usize, Vec::new(), None);
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let body = bytes
            .get(pos + 8..pos + 8 + len)
            .ok_or_else(|| "truncated chunk".to_string())?;
        match kind {
            b"IHDR" => {
                let w = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
                let h = u32::from_be_bytes([body[4], body[5], body[6], body[7]]);
                if (body[8], body[9], body[12]) != (8, 2, 0) {
                    return Err(format!(
                        "unsupported PNG: depth {} type {}",
                        body[8], body[9]
                    ));
                }
                dims = Some((w, h));
            }
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        pos += 12 + len;
    }
    let (width, height) = dims.ok_or_else(|| "no IHDR".to_string())?;
    if idat.len() < 2 {
        return Err("no IDAT".into());
    }
    let raw = inflate(&idat[2..])?;
    let stride = width as usize * 3;
    let mut rgb = vec![0u8; stride * height as usize];
    let mut src = 0usize;
    for y in 0..height as usize {
        let filter = *raw.get(src).ok_or_else(|| "short scanline".to_string())?;
        src += 1;
        let line = raw
            .get(src..src + stride)
            .ok_or_else(|| "short scanline".to_string())?
            .to_vec();
        src += stride;
        for x in 0..stride {
            let a = if x >= 3 { rgb[y * stride + x - 3] } else { 0 };
            let b = if y > 0 { rgb[(y - 1) * stride + x] } else { 0 };
            let c = if x >= 3 && y > 0 {
                rgb[(y - 1) * stride + x - 3]
            } else {
                0
            };
            let v = line[x];
            rgb[y * stride + x] = match filter {
                0 => v,
                1 => v.wrapping_add(a),
                2 => v.wrapping_add(b),
                3 => v.wrapping_add(((u16::from(a) + u16::from(b)) / 2) as u8),
                4 => v.wrapping_add(paeth(a, b, c)),
                f => return Err(format!("unknown PNG filter {f}")),
            };
        }
    }
    Ok(Png { width, height, rgb })
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i32::from(a) + i32::from(b) - i32::from(c);
    let (pa, pb, pc) = (
        (p - i32::from(a)).abs(),
        (p - i32::from(b)).abs(),
        (p - i32::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}
