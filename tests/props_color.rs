//! `colr` in both flavours, the matrix table, and how `gather` assembles a
//! whole item's properties.

mod common;

use common::*;
use heic_rs::boxes::BoxIter;
use heic_rs::props::colr::{self, ColorInfo, MatrixCoefficients, Range};
use heic_rs::props::simple::{Mirror, Rotation};
use heic_rs::props::{self, Transform};

fn one_box(data: &[u8]) -> heic_rs::boxes::BoxHeader<'_> {
    BoxIter::new(data).next().expect("a box").expect("parses")
}

#[test]
fn colr_nclx_reads_matrix_and_range() {
    // Primaries 2, transfer 2, matrix 6 (BT.601), full range: what sips writes.
    let body = cat(&[
        b"nclx",
        &2u16.to_be_bytes(),
        &2u16.to_be_bytes(),
        &6u16.to_be_bytes(),
        &[0x80],
    ]);
    let d = bx(b"colr", &body);
    let ColorInfo::Nclx(n) = colr::parse(&one_box(&d)).expect("parses") else {
        panic!("expected nclx");
    };
    assert_eq!(n.primaries, 2);
    assert_eq!(n.matrix, MatrixCoefficients::Bt601);
    assert_eq!(n.matrix_code, 6);
    assert_eq!(n.range, Range::Full);
    assert_eq!(n.matrix.luma_weights(), (0.299, 0.114));
}

#[test]
fn colr_carries_icc_profiles() {
    for (kind, restricted) in [(b"prof", false), (b"rICC", true)] {
        let d = bx(b"colr", &cat(&[kind, b"icc-bytes"]));
        let ColorInfo::Icc {
            restricted: r,
            data,
        } = colr::parse(&one_box(&d)).expect("parses")
        else {
            panic!("expected icc");
        };
        assert_eq!(r, restricted);
        assert_eq!(data, b"icc-bytes");
    }
}

#[test]
fn matrix_codes_map_to_the_right_weights() {
    assert_eq!(
        MatrixCoefficients::from_code(0),
        MatrixCoefficients::Identity
    );
    assert_eq!(MatrixCoefficients::from_code(1), MatrixCoefficients::Bt709);
    assert_eq!(MatrixCoefficients::from_code(5), MatrixCoefficients::Bt601);
    assert_eq!(
        MatrixCoefficients::from_code(9),
        MatrixCoefficients::Bt2020Ncl
    );
    assert_eq!(MatrixCoefficients::Bt709.luma_weights(), (0.2126, 0.0722));
    assert_eq!(
        MatrixCoefficients::Bt2020Ncl.luma_weights(),
        (0.2627, 0.0593)
    );
    // Unspecified falls back to BT.709, which is documented behaviour.
    assert_eq!(
        MatrixCoefficients::Unspecified.luma_weights(),
        (0.2126, 0.0722)
    );
}

#[test]
fn gather_keeps_transform_order_and_both_colr_flavours() {
    let nclx = bx(
        b"colr",
        &cat(&[
            b"nclx",
            &1u16.to_be_bytes(),
            &13u16.to_be_bytes(),
            &1u16.to_be_bytes(),
            &[0],
        ]),
    );
    let icc = bx(b"colr", &cat(&[b"prof", b"profile"]));
    let ipco = bx(
        b"ipco",
        &cat(&[
            &ispe(8, 8),
            &bx(b"imir", &[1]),
            &bx(b"irot", &[1]),
            &nclx,
            &icc,
        ]),
    );
    // Associated as mirror-then-rotate, which is the order they must apply in.
    let assoc = ipma(&[(
        1,
        vec![(1, false), (2, true), (3, true), (4, true), (5, false)],
    )]);
    let d = bx(b"iprp", &cat(&[&ipco, &assoc]));
    let properties = heic_rs::meta::iprp::parse(&one_box(&d)).expect("parses");
    let p = props::gather(&properties, 1).expect("gathers");
    assert_eq!(
        p.transforms,
        vec![
            Transform::Mirror(Mirror::TopBottom),
            Transform::Rotate(Rotation::Ccw90)
        ]
    );
    assert_eq!(p.rotation(), Rotation::Ccw90);
    assert_eq!(p.mirror(), Some(Mirror::TopBottom));
    assert!(p.clap().is_none());
    assert_eq!(p.nclx.map(|n| n.range), Some(Range::Limited));
    assert_eq!(p.icc, Some(&b"profile"[..]));
    assert!(!p.icc_restricted);
    assert_eq!(p.ispe.map(|i| (i.width, i.height)), Some((8, 8)));
}
