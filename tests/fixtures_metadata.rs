//! EXIF, XMP, colour and codec configuration, read out of the real fixtures.
//!
//! Every value asserted here was read out of the files with a separate box
//! walker before being written down. See `tests/fixtures/README.md` for how
//! the fixtures are generated.

use heic_rs::context::Context;
use heic_rs::props::colr::{MatrixCoefficients, Range};

fn load(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} is missing: {e}"))
}

const ALL: [&str; 9] = [
    "flat-white-16.heic",
    "flat-64.heic",
    "checker-64.heic",
    "rgb-strips-96.heic",
    "gradient-512.heic",
    "checker-1024.heic",
    "photo-2048.heic",
    "rotated-90.heic",
    "with-exif.heic",
];

#[test]
fn exif_is_found_and_unwrapped_to_its_tiff_header() {
    for (name, entries) in [("rotated-90.heic", 3u16), ("with-exif.heic", 6)] {
        let bytes = load(name);
        let info = heic_rs::probe(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(info.has_exif, "{name}");
        let ctx = Context::open(&bytes).expect("opens");
        assert_eq!(ctx.exif_item(13), Some(14), "{name}");
        let tiff = ctx.exif(13).expect("reads").expect("is present");
        // Big-endian TIFF: `MM`, 42, then the offset of the first IFD.
        assert_eq!(&tiff[..2], b"MM", "{name}");
        assert_eq!(u16::from_be_bytes([tiff[2], tiff[3]]), 42, "{name}");
        let ifd = u32::from_be_bytes([tiff[4], tiff[5], tiff[6], tiff[7]]) as usize;
        assert_eq!(ifd, 8, "{name}");
        assert_eq!(
            u16::from_be_bytes([tiff[ifd], tiff[ifd + 1]]),
            entries,
            "{name}"
        );
    }
    // The files without an Exif item say so rather than inventing one.
    let bytes = load("flat-64.heic");
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.exif_item(1), None);
    assert_eq!(ctx.exif(1).expect("reads"), None);
}

#[test]
fn xmp_is_recognised_only_where_it_exists() {
    let bytes = load("with-exif.heic");
    assert!(heic_rs::probe(&bytes).expect("probes").has_xmp);
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.xmp_item(13), Some(15));
    let xmp = ctx.item_data(15).expect("reads");
    assert!(xmp.starts_with(b"<x:xmpmeta"), "expected an XMP packet");

    let bytes = load("photo-2048.heic");
    assert!(!heic_rs::probe(&bytes).expect("probes").has_xmp);
}

#[test]
fn the_colour_description_is_read_exactly() {
    for name in ALL {
        let bytes = load(name);
        let ctx = Context::open(&bytes).expect("opens");
        let p = ctx.props(ctx.meta.primary).expect("gathers");
        let n = p.nclx.unwrap_or_else(|| panic!("{name} has no nclx"));
        assert_eq!(
            (n.primaries, n.transfer, n.matrix_code),
            (2, 2, 6),
            "{name}"
        );
        assert_eq!(n.matrix, MatrixCoefficients::Bt601, "{name}");
        assert_eq!(n.range, Range::Full, "{name}");
        assert_eq!(p.icc, None, "{name}");
        assert_eq!(
            p.pixi.map(|p| p.bits_per_channel),
            Some(vec![8, 8, 8]),
            "{name}"
        );
    }
}

#[test]
fn the_codec_configuration_is_read_exactly() {
    for name in ALL {
        let bytes = load(name);
        let ctx = Context::open(&bytes).expect("opens");
        // A grid keeps its codec configuration on the tiles, not on itself.
        let coded = match ctx.grid(ctx.meta.primary).expect("resolves") {
            Some((_, tiles)) => tiles[0],
            None => ctx.meta.primary,
        };
        let p = ctx.props(coded).expect("gathers");
        let c = p
            .hvcc
            .as_ref()
            .unwrap_or_else(|| panic!("{name} has no hvcC"));
        assert_eq!(c.configuration_version, 1, "{name}");
        assert_eq!(c.general_profile_idc, 3, "{name}");
        assert_eq!(c.chroma_format, 1, "{name}");
        assert_eq!((c.bit_depth_luma, c.bit_depth_chroma), (8, 8), "{name}");
        assert_eq!(c.length_size, 4, "{name}");
        assert_eq!(c.arrays.len(), 3, "{name}");
        assert_eq!(c.parameter_sets().len(), 3, "{name}");
        // The item data really is a sequence of 4-byte length-prefixed NALs.
        let data = ctx.item_data(coded).expect("reads");
        let nals = c
            .split_nals(&data)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!nals.is_empty(), "{name}");
        // NAL type 20 is CRA_NUT, an intra random access picture.
        assert_eq!(nals[0][0] >> 1 & 0x3f, 20, "{name}");
    }
}
