//! Whole synthetic files: derivation cycles, disagreeing counts, item-offset
//! construction, and the limits that stop a small file asking for a large
//! allocation.

mod common;

use common::synth::*;
use common::*;
use heic_rs::context::Context;
use heic_rs::error::Error;

#[test]
fn a_single_item_file_resolves_end_to_end() {
    let hv = hvcc(1, 8, &[(32, b"v"), (33, b"s"), (34, b"p")]);
    let payload = cat(&[&4u32.to_be_bytes(), &[0x28, 0x01, 0x00, 0x00]]);
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false)]),
            iprp_of(&[ispe(32, 24), hv], &[(1, vec![(1, true), (2, true)])]),
            iloc(&[(1, 0, 0, payload.len() as u32)]),
        ],
        &payload,
    );
    // Patch the extent offset now that the layout is known.
    let bytes = with_offset(bytes, payload.len() as u32);
    let info = heic_rs::probe(&bytes).expect("probes");
    assert_eq!((info.width, info.height), (32, 24));
    assert_eq!(info.primary_item, 1);
    assert!(!info.is_grid);
}

#[test]
fn an_extent_past_the_end_of_the_file_is_a_truncation() {
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false)]),
            iloc(&[(1, 0, 900_000, 16)]),
        ],
        &[0; 4],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert!(matches!(ctx.item_data(1), Err(Error::Truncated(_))));
}

#[test]
fn an_idat_extent_without_an_idat_box_is_reported() {
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"grid", false)]),
            iloc(&[(1, 1, 0, 8)]),
        ],
        &[],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.item_data(1).unwrap_err(), Error::MissingBox("idat"));
}

#[test]
fn a_missing_primary_item_is_reported_by_id() {
    let bytes = file(
        &[
            pitm(9),
            iinf(&[infe(1, b"hvc1", false)]),
            iloc(&[(1, 0, 0, 4)]),
        ],
        &[0; 4],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.meta.primary, 9);
    assert_eq!(ctx.meta.primary_item().unwrap_err(), Error::MissingItem(9));
    assert_eq!(heic_rs::probe(&bytes).unwrap_err(), Error::MissingItem(9));
}

#[test]
fn a_single_item_file_without_pitm_is_unambiguous() {
    let hv = hvcc(1, 8, &[(33, b"s")]);
    let bytes = file(
        &[
            iinf(&[infe(4, b"hvc1", false)]),
            iprp_of(&[ispe(8, 8), hv], &[(4, vec![(1, true), (2, true)])]),
            iloc(&[(4, 0, 0, 4)]),
        ],
        &[0; 4],
    );
    assert_eq!(heic_rs::probe(&bytes).expect("probes").primary_item, 4);
}

#[test]
fn a_file_with_several_items_and_no_pitm_is_refused() {
    let bytes = file(
        &[
            iinf(&[infe(1, b"hvc1", false), infe(2, b"hvc1", false)]),
            iloc(&[(1, 0, 0, 4)]),
        ],
        &[0; 4],
    );
    assert_eq!(
        heic_rs::probe(&bytes).unwrap_err(),
        Error::MissingBox("pitm")
    );
}

#[test]
fn a_declared_size_over_the_limit_is_refused_before_any_decoding() {
    let hv = hvcc(1, 8, &[(33, b"s")]);
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false)]),
            iprp_of(
                &[ispe(65_535, 65_535), hv],
                &[(1, vec![(1, true), (2, true)])],
            ),
            iloc(&[(1, 0, 0, 4)]),
        ],
        &[0; 4],
    );
    // Probing still works: it is only the allocation that is refused.
    assert_eq!(heic_rs::probe(&bytes).expect("probes").width, 65_535);
    let e = heic_rs::decode(&bytes, &heic_rs::DecodeOptions::default())
        .expect_err("4.29 gigapixels is over the default limit");
    assert!(matches!(e, Error::PixelLimit { .. }), "got {e}");
}

#[test]
fn strict_mode_refuses_an_unknown_essential_property() {
    let hv = hvcc(1, 8, &[(33, b"s")]);
    // A real one-NAL item, so that the lenient path gets all the way to the
    // codec rather than tripping over nonsense bytes first.
    let payload = cat(&[&1u32.to_be_bytes(), &[0x28]]);
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false)]),
            iprp_of(
                &[ispe(8, 8), hv, bx(b"zzzz", b"")],
                &[(1, vec![(1, true), (2, true), (3, true)])],
            ),
            iloc(&[(1, 0, 0, payload.len() as u32)]),
        ],
        &payload,
    );
    let bytes = with_offset(bytes, payload.len() as u32);
    let lenient = heic_rs::DecodeOptions::default();
    let strict = heic_rs::DecodeOptions::default().with_strict(true);
    // Lenient mode gets as far as the codec seam; strict mode stops earlier.
    let e = heic_rs::decode(&bytes, &lenient).expect_err("no decoder");
    assert!(heic_rs::hevc::is_not_linked(&e), "got {e}");
    let e = heic_rs::decode(&bytes, &strict).expect_err("an unknown essential property");
    assert!(
        matches!(e, Error::Unsupported(m) if m.contains("essential")),
        "got {e}"
    );
}

#[test]
fn an_avif_file_is_refused_before_anything_else() {
    let bytes = cat(&[&ftyp(b"avif", &[b"mif1", b"avif"]), &meta(&[pitm(1)])]);
    let e = heic_rs::probe(&bytes).expect_err("avif is not decodable");
    assert!(
        matches!(e, Error::Unsupported(m) if m.contains("AVIF")),
        "got {e}"
    );
}

#[test]
fn an_empty_or_tiny_input_is_an_error_not_a_panic() {
    for bytes in [
        &[][..],
        &[0][..],
        &[0, 0, 0, 0][..],
        b"not a heif file at all",
    ] {
        assert!(heic_rs::probe(bytes).is_err());
        assert!(heic_rs::decode(bytes, &heic_rs::DecodeOptions::default()).is_err());
    }
}
