//! Byte-array builders, so every parser test states the bytes it parses.

#![allow(dead_code)]

/// A plain box: size, type, body.
pub fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(kind);
    v.extend_from_slice(body);
    v
}

/// A FullBox: size, type, version, flags, body.
pub fn full(kind: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut inner = vec![version];
    inner.extend_from_slice(&flags.to_be_bytes()[1..]);
    inner.extend_from_slice(body);
    bx(kind, &inner)
}

/// Concatenate byte runs.
pub fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// A NUL-terminated string.
pub fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// An `ftyp` box with the given major brand and compatible brands.
pub fn ftyp(major: &[u8; 4], compatible: &[&[u8; 4]]) -> Vec<u8> {
    let mut body = major.to_vec();
    body.extend_from_slice(&0u32.to_be_bytes());
    for c in compatible {
        body.extend_from_slice(*c);
    }
    bx(b"ftyp", &body)
}

/// An `infe` version 2 entry.
pub fn infe(id: u16, kind: &[u8; 4], hidden: bool) -> Vec<u8> {
    let mut body = id.to_be_bytes().to_vec();
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(kind);
    body.push(0);
    full(b"infe", 2, u32::from(hidden), &body)
}

/// An `iinf` version 0 box wrapping the given `infe` entries.
pub fn iinf(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut body = (entries.len() as u16).to_be_bytes().to_vec();
    for e in entries {
        body.extend_from_slice(e);
    }
    full(b"iinf", 0, 0, &body)
}

/// One `iref` entry of a given kind: `from` points at `to`.
pub fn iref_entry(kind: &[u8; 4], from: u16, to: &[u16]) -> Vec<u8> {
    let mut body = from.to_be_bytes().to_vec();
    body.extend_from_slice(&(to.len() as u16).to_be_bytes());
    for t in to {
        body.extend_from_slice(&t.to_be_bytes());
    }
    bx(kind, &body)
}

/// An `iref` version 0 box.
pub fn iref(entries: &[Vec<u8>]) -> Vec<u8> {
    full(
        b"iref",
        0,
        0,
        &cat(&entries.iter().map(|e| e.as_slice()).collect::<Vec<_>>()),
    )
}

/// An `iloc` version 1 box with one single-extent entry per item.
///
/// Each entry is `(item id, construction method, offset, length)`.
pub fn iloc(items: &[(u16, u8, u32, u32)]) -> Vec<u8> {
    // offset_size 4, length_size 4, base_offset_size 0, index_size 0.
    let mut body = vec![0x44, 0x00];
    body.extend_from_slice(&(items.len() as u16).to_be_bytes());
    for (id, method, offset, length) in items {
        body.extend_from_slice(&id.to_be_bytes());
        body.extend_from_slice(&u16::from(*method).to_be_bytes());
        body.extend_from_slice(&0u16.to_be_bytes());
        body.extend_from_slice(&1u16.to_be_bytes());
        body.extend_from_slice(&offset.to_be_bytes());
        body.extend_from_slice(&length.to_be_bytes());
    }
    full(b"iloc", 1, 0, &body)
}

/// An `ipma` version 0 box: `(item id, [(index, essential)])`.
pub fn ipma(entries: &[(u16, Vec<(u8, bool)>)]) -> Vec<u8> {
    let mut body = (entries.len() as u32).to_be_bytes().to_vec();
    for (id, assoc) in entries {
        body.extend_from_slice(&id.to_be_bytes());
        body.push(assoc.len() as u8);
        for (index, essential) in assoc {
            body.push(index | if *essential { 0x80 } else { 0 });
        }
    }
    full(b"ipma", 0, 0, &body)
}

/// An `ispe` box.
pub fn ispe(w: u32, h: u32) -> Vec<u8> {
    full(b"ispe", 0, 0, &cat(&[&w.to_be_bytes(), &h.to_be_bytes()]))
}

/// A minimal `hvcC` with the given chroma format, depth and NAL arrays.
pub fn hvcc(chroma: u8, depth: u8, nals: &[(u8, &[u8])]) -> Vec<u8> {
    let mut body = vec![1, 0x03];
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    body.push(90);
    body.extend_from_slice(&0xf000u16.to_be_bytes());
    body.push(0xfc);
    body.push(0xfc | chroma);
    body.push(0xf8 | (depth - 8));
    body.push(0xf8 | (depth - 8));
    body.extend_from_slice(&0u16.to_be_bytes());
    // constantFrameRate 0, numTemporalLayers 1, nested 0, lengthSizeMinusOne 3
    body.push(0b0000_1011);
    body.push(nals.len() as u8);
    for (kind, payload) in nals {
        body.push(0x80 | kind);
        body.extend_from_slice(&1u16.to_be_bytes());
        body.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        body.extend_from_slice(payload);
    }
    bx(b"hvcC", &body)
}

/// A `grid` item payload with 16-bit output dimensions.
pub fn grid_payload(rows: u8, cols: u8, w: u16, h: u16) -> Vec<u8> {
    cat(&[
        &[0, 0, rows - 1, cols - 1],
        &w.to_be_bytes(),
        &h.to_be_bytes(),
    ])
}

/// Wrap `meta` children into a `meta` box.
pub fn meta(children: &[Vec<u8>]) -> Vec<u8> {
    let hdlr = {
        let mut body = vec![0u8; 4];
        body.extend_from_slice(b"pict");
        body.extend_from_slice(&[0u8; 12]);
        body.push(0);
        full(b"hdlr", 0, 0, &body)
    };
    let mut body = hdlr;
    for c in children {
        body.extend_from_slice(c);
    }
    full(b"meta", 0, 0, &body)
}

/// A frame of one flat colour, 4:2:0 at 8 bits.
pub fn solid_frame(w: u32, h: u32, y: u16, cb: u16, cr: u16) -> heic_rs::hevc::Frame {
    use heic_rs::hevc::{ChromaFormat, Frame};
    let (cw, ch) = ChromaFormat::Yuv420.chroma_size(w, h);
    Frame {
        width: w,
        height: h,
        bit_depth: 8,
        chroma: ChromaFormat::Yuv420,
        y: vec![y; (w * h) as usize],
        cb: vec![cb; (cw * ch) as usize],
        cr: vec![cr; (cw * ch) as usize],
        y_stride: w,
        c_stride: cw,
    }
}

/// A frame whose luma is `f(x, y)`, monochrome so there is no chroma to reason
/// about.
pub fn mono_frame(w: u32, h: u32, f: impl Fn(u32, u32) -> u16) -> heic_rs::hevc::Frame {
    use heic_rs::hevc::{ChromaFormat, Frame};
    let mut y = Vec::with_capacity((w * h) as usize);
    for row in 0..h {
        for col in 0..w {
            y.push(f(col, row));
        }
    }
    Frame {
        width: w,
        height: h,
        bit_depth: 8,
        chroma: ChromaFormat::Monochrome,
        y,
        cb: Vec::new(),
        cr: Vec::new(),
        y_stride: w,
        c_stride: 0,
    }
}

pub mod inflate;
pub mod png;
pub mod synth;
