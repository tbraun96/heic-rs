//! `iinf`, `infe`, `pitm`, `iref`, `iloc`, `ipma` and `ipco`, against bytes
//! written out here in the test.

mod common;

use common::*;
use heic_rs::boxes::BoxIter;
use heic_rs::error::Error;
use heic_rs::meta::iinf;
use heic_rs::meta::iloc::{self, Construction};
use heic_rs::meta::iprp;
use heic_rs::meta::iref::{self, RefKind};

fn one_box(data: &[u8]) -> heic_rs::boxes::BoxHeader<'_> {
    BoxIter::new(data).next().expect("a box").expect("parses")
}

#[test]
fn iinf_lists_items_with_their_types() {
    let data = iinf(&[
        infe(1, b"hvc1", true),
        infe(2, b"grid", false),
        infe(3, b"Exif", true),
    ]);
    let items = iinf::parse(&one_box(&data)).expect("parses");
    assert_eq!(items.len(), 3);
    assert_eq!(
        (items[0].id, items[0].item_type, items[0].hidden),
        (1, *b"hvc1", true)
    );
    assert!(items[0].is_hevc());
    assert_eq!((items[1].id, items[1].item_type), (2, *b"grid"));
    assert!(items[1].is_grid());
    assert_eq!(items[2].item_type, *b"Exif");
}

#[test]
fn infe_reads_a_mime_content_type() {
    let mut body = 9u16.to_be_bytes().to_vec();
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(b"mime");
    body.extend_from_slice(&cstr(""));
    body.extend_from_slice(&cstr("application/rdf+xml"));
    let data = full(b"infe", 2, 0, &body);
    let item = iinf::parse_infe(&one_box(&data)).expect("parses");
    assert_eq!(item.id, 9);
    assert_eq!(item.content_type, Some("application/rdf+xml"));
}

#[test]
fn infe_version_3_uses_a_32_bit_id() {
    let mut body = 70_000u32.to_be_bytes().to_vec();
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(b"hvc1");
    body.push(0);
    let data = full(b"infe", 3, 0, &body);
    assert_eq!(
        iinf::parse_infe(&one_box(&data)).expect("parses").id,
        70_000
    );
}

#[test]
fn iinf_refuses_an_impossible_entry_count() {
    let data = full(b"iinf", 0, 0, &u16::MAX.to_be_bytes());
    assert!(matches!(
        iinf::parse(&one_box(&data)),
        Err(Error::Malformed(_))
    ));
}

#[test]
fn pitm_reads_both_widths() {
    let v0 = full(b"pitm", 0, 0, &7u16.to_be_bytes());
    assert_eq!(iinf::parse_pitm(&one_box(&v0)).expect("parses"), 7);
    let v1 = full(b"pitm", 1, 0, &70_000u32.to_be_bytes());
    assert_eq!(iinf::parse_pitm(&one_box(&v1)).expect("parses"), 70_000);
}

#[test]
fn iref_keeps_dimg_order() {
    let data = iref(&[
        iref_entry(b"dimg", 13, &[3, 1, 2]),
        iref_entry(b"cdsc", 14, &[13]),
        iref_entry(b"auxl", 15, &[13]),
    ]);
    let refs = iref::parse(&one_box(&data)).expect("parses");
    assert_eq!(refs.len(), 3);
    assert_eq!(iref::targets(&refs, 13, RefKind::Dimg), &[3, 1, 2]);
    assert_eq!(iref::targets(&refs, 14, RefKind::Cdsc), &[13]);
    assert_eq!(iref::sources(&refs, 13, RefKind::Auxl), vec![15]);
    assert!(iref::targets(&refs, 99, RefKind::Dimg).is_empty());
}

#[test]
fn iref_refuses_a_count_longer_than_its_body() {
    let mut body = 1u16.to_be_bytes().to_vec();
    body.extend_from_slice(&500u16.to_be_bytes());
    let data = full(b"iref", 0, 0, &bx(b"dimg", &body));
    assert!(matches!(
        iref::parse(&one_box(&data)),
        Err(Error::Malformed(_))
    ));
}

#[test]
fn iloc_reads_all_three_construction_methods() {
    let data = iloc(&[(1, 0, 100, 10), (2, 1, 0, 8), (3, 2, 4, 6)]);
    let locs = iloc::parse(&one_box(&data)).expect("parses");
    assert_eq!(locs.len(), 3);
    assert_eq!(locs[0].construction, Construction::File);
    assert_eq!(locs[0].extents[0].offset, 100);
    assert_eq!(locs[0].total_length(), Some(10));
    assert_eq!(locs[1].construction, Construction::Idat);
    assert_eq!(locs[2].construction, Construction::Item);
}

#[test]
fn iloc_version_0_has_no_construction_method() {
    // offset_size 4, length_size 4, base_offset_size 4, one item, one extent.
    let mut body = vec![0x44, 0x40];
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&5u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&1000u32.to_be_bytes());
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&24u32.to_be_bytes());
    body.extend_from_slice(&16u32.to_be_bytes());
    let data = full(b"iloc", 0, 0, &body);
    let locs = iloc::parse(&one_box(&data)).expect("parses");
    assert_eq!(locs[0].construction, Construction::File);
    assert_eq!(locs[0].base_offset, 1000);
    assert_eq!(locs[0].extents[0].offset, 24);
}

#[test]
fn iloc_refuses_an_unknown_construction_method() {
    let data = iloc(&[(1, 7, 0, 1)]);
    assert!(matches!(
        iloc::parse(&one_box(&data)),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn iloc_refuses_an_impossible_item_count() {
    let data = full(
        b"iloc",
        1,
        0,
        &cat(&[&[0x44, 0x00], &u16::MAX.to_be_bytes()]),
    );
    assert!(matches!(
        iloc::parse(&one_box(&data)),
        Err(Error::Malformed(_))
    ));
}

#[test]
fn ipma_resolves_property_indices_and_essential_flags() {
    let ipco = bx(
        b"ipco",
        &cat(&[&ispe(64, 48), &hvcc(1, 8, &[(32, b"v"), (33, b"s")])]),
    );
    let assoc = ipma(&[(1, vec![(1, false), (2, true)])]);
    let data = bx(b"iprp", &cat(&[&ipco, &assoc]));
    let props = iprp::parse(&one_box(&data)).expect("parses");
    assert_eq!(props.boxes.len(), 2);
    let a = props.associations(1);
    assert_eq!(a.len(), 2);
    assert_eq!((a[0].index, a[0].essential), (1, false));
    assert_eq!((a[1].index, a[1].essential), (2, true));
    assert!(props.find(1, b"ispe").is_some());
    assert!(props.find(1, b"hvcC").is_some());
    assert!(props.find(1, b"colr").is_none());
    assert!(props.find(2, b"ispe").is_none());
}

#[test]
fn ipma_with_the_wide_index_flag_reads_15_bit_indices() {
    let mut body = 1u32.to_be_bytes().to_vec();
    body.extend_from_slice(&1u16.to_be_bytes());
    body.push(1);
    body.extend_from_slice(&(0x8000u16 | 300).to_be_bytes());
    let assoc = full(b"ipma", 0, 1, &body);
    let data = bx(b"iprp", &cat(&[&bx(b"ipco", b""), &assoc]));
    let props = iprp::parse(&one_box(&data)).expect("parses");
    let a = props.associations(1);
    assert_eq!((a[0].index, a[0].essential), (300, true));
    // Index 300 does not exist in an empty ipco, so it resolves to nothing
    // rather than indexing out of bounds.
    assert!(props.resolve(a[0]).is_none());
}

#[test]
fn a_property_index_of_zero_resolves_to_nothing() {
    let data = bx(
        b"iprp",
        &cat(&[&bx(b"ipco", &ispe(1, 1)), &ipma(&[(1, vec![(0, false)])])]),
    );
    let props = iprp::parse(&one_box(&data)).expect("parses");
    assert!(props.resolve(props.associations(1)[0]).is_none());
}

#[test]
fn an_unknown_essential_property_is_reported() {
    let ipco = bx(b"ipco", &cat(&[&ispe(8, 8), &bx(b"zzzz", b"")]));
    let data = bx(
        b"iprp",
        &cat(&[&ipco, &ipma(&[(1, vec![(1, true), (2, true)])])]),
    );
    let props = iprp::parse(&one_box(&data)).expect("parses");
    assert_eq!(props.unknown_essential(1), Some(*b"zzzz"));
    // The same property, not marked essential, is not reported.
    let data = bx(
        b"iprp",
        &cat(&[&ipco, &ipma(&[(1, vec![(1, true), (2, false)])])]),
    );
    let props = iprp::parse(&one_box(&data)).expect("parses");
    assert_eq!(props.unknown_essential(1), None);
}

#[test]
fn a_meta_box_without_a_pict_handler_is_refused() {
    let mut body = vec![0u8; 4];
    body.extend_from_slice(b"vide");
    body.extend_from_slice(&[0u8; 13]);
    let data = full(b"meta", 0, 0, &full(b"hdlr", 0, 0, &body));
    assert!(matches!(
        heic_rs::meta::parse(&data),
        Err(Error::Unsupported(_))
    ));
}
