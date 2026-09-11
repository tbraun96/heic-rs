//! Box traversal, `ftyp`, and what happens to hostile sizes.

mod common;

use common::*;
use heic_rs::boxes::{BoxIter, MAX_BOX_SIZE};
use heic_rs::error::Error;
use heic_rs::ftyp::Brand;

#[test]
fn walks_a_sequence_of_boxes() {
    let data = cat(&[
        &bx(b"ftyp", b"heic"),
        &bx(b"free", b""),
        &bx(b"mdat", &[1, 2, 3]),
    ]);
    let seen: Vec<_> = BoxIter::new(&data)
        .map(|b| b.map(|b| (b.boxtype, b.payload.len())))
        .collect();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[0].as_ref().map(|x| *x).ok(), Some((*b"ftyp", 4)));
    assert_eq!(seen[1].as_ref().map(|x| *x).ok(), Some((*b"free", 0)));
    assert_eq!(seen[2].as_ref().map(|x| *x).ok(), Some((*b"mdat", 3)));
}

#[test]
fn size_zero_runs_to_the_end() {
    let mut data = 0u32.to_be_bytes().to_vec();
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&[9, 9, 9, 9]);
    let b = BoxIter::new(&data)
        .next()
        .expect("one box")
        .expect("parses");
    assert_eq!(b.payload, &[9, 9, 9, 9]);
}

#[test]
fn largesize_is_honoured() {
    let mut data = 1u32.to_be_bytes().to_vec();
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&24u64.to_be_bytes());
    data.extend_from_slice(&[7; 8]);
    let b = BoxIter::new(&data)
        .next()
        .expect("one box")
        .expect("parses");
    assert_eq!(b.payload, &[7; 8]);
}

#[test]
fn uuid_boxes_expose_their_extended_type() {
    let uuid = [0xaa; 16];
    let data = bx(b"uuid", &cat(&[&uuid, b"body"]));
    let b = BoxIter::new(&data)
        .next()
        .expect("one box")
        .expect("parses");
    assert_eq!(b.uuid, Some(uuid));
    assert_eq!(b.payload, b"body");
}

#[test]
fn a_truncated_box_is_an_error_not_a_panic() {
    let mut data = bx(b"mdat", &[0; 32]);
    data.truncate(20);
    let first = BoxIter::new(&data).next().expect("an item");
    assert_eq!(first.unwrap_err(), Error::Truncated("box body"));
}

#[test]
fn an_absurd_size_is_refused_before_allocating() {
    let mut data = 1u32.to_be_bytes().to_vec();
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&(MAX_BOX_SIZE + 1).to_be_bytes());
    let first = BoxIter::new(&data).next().expect("an item");
    assert_eq!(
        first.unwrap_err(),
        Error::BoxTooLarge {
            boxtype: *b"mdat",
            size: MAX_BOX_SIZE + 1
        }
    );
}

#[test]
fn a_size_smaller_than_the_header_is_refused() {
    let mut data = 4u32.to_be_bytes().to_vec();
    data.extend_from_slice(b"mdat");
    let first = BoxIter::new(&data).next().expect("an item");
    assert!(matches!(first.unwrap_err(), Error::Malformed(_)));
}

#[test]
fn iteration_stops_after_the_first_error() {
    let mut data = bx(b"free", b"");
    data.extend_from_slice(&[0, 0, 0, 4]);
    data.extend_from_slice(b"mdat");
    let results: Vec<_> = BoxIter::new(&data).collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
}

#[test]
fn ftyp_reads_brands() {
    let data = ftyp(b"heic", &[b"mif1", b"heic"]);
    let f = heic_rs::ftyp::parse(&data).expect("parses");
    assert_eq!(f.major, Brand::Heic);
    assert_eq!(f.effective, Brand::Heic);
    assert_eq!(f.minor_version, 0);
    assert_eq!(f.compatible_count, 2);
}

#[test]
fn ftyp_falls_back_to_a_compatible_brand() {
    let data = ftyp(b"XXXX", &[b"mif1"]);
    let f = heic_rs::ftyp::parse(&data).expect("parses");
    assert_eq!(f.major, Brand::Mif1);
}

#[test]
fn avif_is_named_in_the_refusal() {
    for data in [ftyp(b"avif", &[b"mif1"]), ftyp(b"mif1", &[b"avif"])] {
        let e = heic_rs::ftyp::parse(&data).expect_err("avif is refused");
        let Error::Unsupported(msg) = e else {
            panic!("wrong error kind")
        };
        assert!(
            msg.contains("AVIF"),
            "message should name AVIF, got {msg:?}"
        );
    }
}

#[test]
fn an_unknown_brand_is_refused() {
    let data = ftyp(b"XXXX", &[b"YYYY"]);
    assert!(matches!(
        heic_rs::ftyp::parse(&data),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn a_file_with_no_ftyp_is_refused() {
    let data = bx(b"mdat", b"");
    assert_eq!(
        heic_rs::ftyp::parse(&data).unwrap_err(),
        Error::MissingBox("ftyp")
    );
}

#[test]
fn every_brand_round_trips_through_its_fourcc() {
    for b in [
        Brand::Heic,
        Brand::Heix,
        Brand::Heim,
        Brand::Heis,
        Brand::Hevc,
        Brand::Mif1,
    ] {
        assert_eq!(Brand::from_fourcc(b.fourcc()), Some(b));
        assert!(b.is_decodable());
    }
    assert!(!Brand::Avif.is_decodable());
}
