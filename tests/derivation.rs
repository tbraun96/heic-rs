//! Derivation: `grid` cycles, tile counts that disagree with `dimg`, the
//! overlay derivation this crate declines, and `iloc` construction method 2.

mod common;

use common::synth::*;
use common::*;
use heic_rs::context::Context;
use heic_rs::error::Error;

#[test]
fn a_grid_that_lists_itself_is_a_cycle() {
    let payload = grid_payload(1, 2, 4, 2);
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"grid", false), infe(2, b"hvc1", true)]),
            iref(&[iref_entry(b"dimg", 1, &[1, 2])]),
            bx(b"idat", &payload),
            iloc(&[(1, 1, 0, payload.len() as u32), (2, 0, 0, 4)]),
        ],
        &[0; 4],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.grid(1).unwrap_err(), Error::CyclicDerivation(1));
}

#[test]
fn a_grid_whose_tile_is_itself_a_grid_is_a_cycle() {
    let payload = grid_payload(1, 1, 4, 2);
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"grid", false), infe(2, b"grid", true)]),
            iref(&[iref_entry(b"dimg", 1, &[2]), iref_entry(b"dimg", 2, &[1])]),
            bx(b"idat", &payload),
            iloc(&[
                (1, 1, 0, payload.len() as u32),
                (2, 1, 0, payload.len() as u32),
            ]),
        ],
        &[0; 4],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.grid(1).unwrap_err(), Error::CyclicDerivation(2));
}

#[test]
fn a_grid_whose_tile_count_disagrees_with_dimg_is_refused() {
    // The payload says two columns, so two tiles; the dimg lists three.
    let payload = grid_payload(1, 2, 4, 2);
    let bytes = file(
        &[
            pitm(1),
            iinf(&[
                infe(1, b"grid", false),
                infe(2, b"hvc1", true),
                infe(3, b"hvc1", true),
                infe(4, b"hvc1", true),
            ]),
            iref(&[iref_entry(b"dimg", 1, &[2, 3, 4])]),
            bx(b"idat", &payload),
            iloc(&[(1, 1, 0, payload.len() as u32)]),
        ],
        &[0; 4],
    );
    let ctx = Context::open(&bytes).expect("opens");
    let e = ctx.grid(1).expect_err("the counts disagree");
    assert!(
        matches!(e, Error::Malformed(m) if m.contains("dimg")),
        "got {e}"
    );
}

#[test]
fn an_overlay_derivation_is_refused_by_name() {
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"iovl", false)]),
            iloc(&[(1, 1, 0, 4)]),
            bx(b"idat", &[0; 4]),
        ],
        &[],
    );
    let ctx = Context::open(&bytes).expect("opens");
    let e = ctx.grid(1).expect_err("iovl is not implemented");
    assert!(
        matches!(e, Error::Unsupported(m) if m.contains("iovl")),
        "got {e}"
    );
}

#[test]
fn an_item_that_constructs_from_itself_is_a_cycle() {
    // Construction method 2 with an extent index naming the item itself.
    let mut body = vec![0x44, 0x44];
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&2u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&4u32.to_be_bytes());
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false)]),
            full(b"iloc", 1, 0, &body),
        ],
        &[],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.item_data(1).unwrap_err(), Error::CyclicDerivation(1));
}

#[test]
fn an_item_offset_extent_reads_from_another_item() {
    let mdat = b"0123456789abcdef".to_vec();
    let mut body = vec![0x44, 0x44];
    body.extend_from_slice(&2u16.to_be_bytes());
    // Item 1: a plain file extent over the whole mdat body.
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&(mdat.len() as u32).to_be_bytes());
    // Item 2: four bytes taken from inside item 1, starting at offset 4.
    body.extend_from_slice(&2u16.to_be_bytes());
    body.extend_from_slice(&2u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&4u32.to_be_bytes());
    body.extend_from_slice(&4u32.to_be_bytes());
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false), infe(2, b"hvc1", true)]),
            full(b"iloc", 1, 0, &body),
        ],
        &mdat,
    );
    // Point item 1 at the mdat body.
    let offset = mdat_offset(&bytes);
    let needle = cat(&[&0u32.to_be_bytes(), &(mdat.len() as u32).to_be_bytes()]);
    let pos = bytes
        .windows(8)
        .position(|w| w == needle.as_slice())
        .expect("the extent");
    let mut bytes = bytes;
    bytes[pos..pos + 4].copy_from_slice(&offset.to_be_bytes());

    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(&ctx.item_data(1).expect("reads")[..], mdat.as_slice());
    assert_eq!(&ctx.item_data(2).expect("reads")[..], b"4567");
}
