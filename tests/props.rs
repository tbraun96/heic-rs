//! `hvcC`, `ispe`, `pixi`, `colr`, `irot`, `imir`, `clap`, `auxC`, `pasp`.

mod common;

use common::*;
use heic_rs::boxes::BoxIter;
use heic_rs::error::Error;
use heic_rs::props::hvcc;
use heic_rs::props::simple::{self, AuxKind, Mirror, Rotation};

fn one_box(data: &[u8]) -> heic_rs::boxes::BoxHeader<'_> {
    BoxIter::new(data).next().expect("a box").expect("parses")
}

#[test]
fn hvcc_reads_the_whole_configuration_record() {
    let data = hvcc(1, 8, &[(32, b"vps"), (33, b"sps-bytes"), (34, b"pps")]);
    let c = hvcc::parse(&one_box(&data)).expect("parses");
    assert_eq!(c.configuration_version, 1);
    assert_eq!(c.general_profile_space, 0);
    assert!(!c.general_tier_flag);
    assert_eq!(c.general_profile_idc, 3);
    assert_eq!(c.general_level_idc, 90);
    assert_eq!(c.chroma_format, 1);
    assert_eq!((c.bit_depth_luma, c.bit_depth_chroma), (8, 8));
    assert_eq!(c.length_size, 4);
    assert_eq!(c.num_temporal_layers, 1);
    assert_eq!(c.arrays.len(), 3);
    assert_eq!(
        c.parameter_sets(),
        vec![&b"vps"[..], &b"sps-bytes"[..], &b"pps"[..]]
    );
}

#[test]
fn hvcc_reports_ten_bit_depth() {
    let data = hvcc(1, 10, &[(33, b"sps")]);
    let c = hvcc::parse(&one_box(&data)).expect("parses");
    assert_eq!((c.bit_depth_luma, c.bit_depth_chroma), (10, 10));
}

#[test]
fn hvcc_refuses_more_nal_units_than_it_holds() {
    let mut data = hvcc(1, 8, &[(33, b"sps")]);
    // The tail of the record is: array header, NAL count, NAL length, payload.
    // Overwrite the count with something far larger than the body can hold.
    let len = data.len();
    data[len - 7] = 0xff;
    data[len - 6] = 0xff;
    assert!(matches!(
        hvcc::parse(&one_box(&data)),
        Err(Error::Malformed(_) | Error::Truncated(_))
    ));
}

#[test]
fn split_nals_honours_the_length_prefix() {
    let data = hvcc(1, 8, &[(33, b"sps")]);
    let c = hvcc::parse(&one_box(&data)).expect("parses");
    let item = cat(&[&3u32.to_be_bytes(), b"abc", &2u32.to_be_bytes(), b"de"]);
    assert_eq!(
        c.split_nals(&item).expect("splits"),
        vec![&b"abc"[..], &b"de"[..]]
    );
    // A length that runs past the end is a truncation, not a panic.
    let bad = cat(&[&99u32.to_be_bytes(), b"abc"]);
    assert!(matches!(c.split_nals(&bad), Err(Error::Truncated(_))));
    // A zero length would otherwise loop forever.
    let zero = cat(&[&0u32.to_be_bytes()]);
    assert!(matches!(c.split_nals(&zero), Err(Error::Malformed(_))));
}

#[test]
fn ispe_and_pixi_and_pasp() {
    let d = ispe(4032, 3024);
    assert_eq!(
        simple::parse_ispe(&one_box(&d)).expect("parses").width,
        4032
    );
    let d = ispe(0, 10);
    assert!(matches!(
        simple::parse_ispe(&one_box(&d)),
        Err(Error::Malformed(_))
    ));
    let d = full(b"pixi", 0, 0, &[3, 8, 8, 8]);
    assert_eq!(
        simple::parse_pixi(&one_box(&d))
            .expect("parses")
            .bits_per_channel,
        vec![8, 8, 8]
    );
    let d = full(b"pixi", 0, 0, &[9, 8]);
    assert!(matches!(
        simple::parse_pixi(&one_box(&d)),
        Err(Error::Malformed(_))
    ));
    let d = bx(b"pasp", &cat(&[&1u32.to_be_bytes(), &1u32.to_be_bytes()]));
    assert_eq!(
        simple::parse_pasp(&one_box(&d)).expect("parses").h_spacing,
        1
    );
}

#[test]
fn irot_and_imir_and_auxc() {
    for (code, want) in [
        (0u8, Rotation::None),
        (1, Rotation::Ccw90),
        (2, Rotation::Ccw180),
        (3, Rotation::Ccw270),
    ] {
        let d = bx(b"irot", &[code]);
        assert_eq!(simple::parse_irot(&one_box(&d)).expect("parses"), want);
    }
    assert!(Rotation::Ccw90.swaps_axes());
    assert!(!Rotation::Ccw180.swaps_axes());
    assert_eq!(Rotation::Ccw270.degrees(), 270);

    let d = bx(b"imir", &[0]);
    assert_eq!(
        simple::parse_imir(&one_box(&d)).expect("parses"),
        Mirror::LeftRight
    );
    let d = bx(b"imir", &[1]);
    assert_eq!(
        simple::parse_imir(&one_box(&d)).expect("parses"),
        Mirror::TopBottom
    );

    let d = full(
        b"auxC",
        0,
        0,
        &cstr("urn:mpeg:mpegB:cicp:systems:auxiliary:alpha"),
    );
    assert_eq!(
        simple::parse_auxc(&one_box(&d)).expect("parses").kind,
        AuxKind::Alpha
    );
    let d = full(b"auxC", 0, 0, &cstr("urn:mpeg:hevc:2015:auxid:2"));
    assert_eq!(
        simple::parse_auxc(&one_box(&d)).expect("parses").kind,
        AuxKind::Depth
    );
    let d = full(b"auxC", 0, 0, &cstr("urn:example:something-else"));
    assert_eq!(
        simple::parse_auxc(&one_box(&d)).expect("parses").kind,
        AuxKind::Other
    );
}

#[test]
fn clap_reads_its_rationals_and_refuses_zero_denominators() {
    let body = cat(&[
        &100u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        &50u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        &0u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        &0u32.to_be_bytes(),
        &1u32.to_be_bytes(),
    ]);
    let d = bx(b"clap", &body);
    let c = simple::parse_clap(&one_box(&d)).expect("parses");
    assert_eq!(c.width, (100, 1));
    assert_eq!(c.height, (50, 1));

    let mut bad = body.clone();
    bad[7] = 0;
    let d = bx(b"clap", &bad);
    assert!(matches!(
        simple::parse_clap(&one_box(&d)),
        Err(Error::Malformed(_))
    ));
}
