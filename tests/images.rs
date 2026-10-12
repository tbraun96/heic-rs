//! Files with more than one image to show, and the items that are not one of them.
//!
//! `three-images.heic` holds three images, each with a thumbnail, and an alpha
//! item for the third; see `tests/fixtures/README.md`.

use heic_rs::{DecodeOptions, PixelLayout, decode_image, images, probe_image};

fn load(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} is missing: {e}"))
}

#[test]
fn lists_the_images_but_not_thumbnails_or_alpha() {
    assert_eq!(images(&load("three-images.heic")).unwrap(), [1, 3, 5]);
    // a file with one image lists its primary item
    let flat = load("flat-64.heic");
    assert_eq!(
        images(&flat).unwrap(),
        [heic_rs::probe(&flat).unwrap().primary_item]
    );
    // the tiles of a grid are not images of their own
    assert_eq!(images(&load("checker-1024.heic")).unwrap().len(), 1);
}

#[test]
fn probes_and_decodes_each_image() {
    let file = load("three-images.heic");
    let options = DecodeOptions {
        layout: PixelLayout::Rgba8,
        ..Default::default()
    };
    // HEVC is lossy and 4:2:0, the colours come back a few values off
    let close = |a: &[u8], b: [u8; 4]| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 8);
    for (id, size, colour, alpha) in [
        (1, (64, 48), [220, 30, 30, 255], false),
        (3, (64, 48), [30, 200, 40, 255], false),
        (5, (32, 80), [40, 60, 230, 128], true),
    ] {
        let info = probe_image(&file, id).unwrap();
        assert_eq!(
            (info.width, info.height, info.has_alpha),
            (size.0, size.1, alpha),
            "item {id}"
        );
        let image = decode_image(&file, id, &options).unwrap();
        assert_eq!((image.width, image.height), size, "item {id}");
        assert!(
            image.data.chunks_exact(4).all(|p| close(p, colour)),
            "item {id}: {:?} instead of {colour:?}",
            &image.data[..4]
        );
    }
}

#[test]
fn refuses_items_that_are_not_images_to_show() {
    let file = load("three-images.heic");
    // item 2 is the thumbnail of the first image, 99 does not exist
    for id in [2, 99] {
        assert!(
            decode_image(&file, id, &DecodeOptions::default()).is_err(),
            "item {id}"
        );
        assert!(probe_image(&file, id).is_err(), "item {id}");
    }
}
