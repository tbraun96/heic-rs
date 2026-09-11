//! Builders for whole synthetic HEIF files.

#![allow(dead_code)]

use super::{bx, cat, ftyp, full, ipma, ispe, meta};

/// Assemble a file from a `meta` body and an `mdat` payload.
pub fn file(meta_children: &[Vec<u8>], mdat: &[u8]) -> Vec<u8> {
    cat(&[
        &ftyp(b"heic", &[b"mif1", b"heic"]),
        &meta(meta_children),
        &bx(b"mdat", mdat),
    ])
}

/// The byte offset at which an `mdat` payload begins in such a file.
pub fn mdat_offset(bytes: &[u8]) -> u32 {
    let pos = bytes
        .windows(4)
        .position(|w| w == b"mdat")
        .expect("an mdat box");
    (pos + 4) as u32
}

/// A `pitm` box naming the primary item.
pub fn pitm(id: u16) -> Vec<u8> {
    full(b"pitm", 0, 0, &id.to_be_bytes())
}

/// An `iprp` wrapping an `ipco` of the given properties and one `ipma`.
pub fn iprp_of(ipco_children: &[Vec<u8>], assoc: &[(u16, Vec<(u8, bool)>)]) -> Vec<u8> {
    let ipco = bx(
        b"ipco",
        &cat(&ipco_children
            .iter()
            .map(|c| c.as_slice())
            .collect::<Vec<_>>()),
    );
    bx(b"iprp", &cat(&[&ipco, &ipma(assoc)]))
}

/// Rewrite a single-extent `iloc` offset so that it points at the `mdat` body.
///
/// The builders lay the file out before the offset is known, so the extent is
/// written as zero and patched here once the size of everything before `mdat`
/// has settled.
pub fn with_offset(mut bytes: Vec<u8>, length: u32) -> Vec<u8> {
    let offset = mdat_offset(&bytes);
    let needle = cat(&[&0u32.to_be_bytes(), &length.to_be_bytes()]);
    let pos = bytes
        .windows(8)
        .position(|w| w == needle.as_slice())
        .expect("the extent");
    bytes[pos..pos + 4].copy_from_slice(&offset.to_be_bytes());
    bytes
}

/// An `ispe` and `hvcC` pair associated with one item, both essential.
pub fn picture_props(item: u16, w: u32, h: u32, hvcc: Vec<u8>) -> Vec<u8> {
    iprp_of(&[ispe(w, h), hvcc], &[(item, vec![(1, true), (2, true)])])
}
