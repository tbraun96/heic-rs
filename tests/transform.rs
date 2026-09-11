//! Applying `irot`, `imir` and `clap`, and predicting the size they produce.

mod common;

use heic_rs::error::Error;
use heic_rs::image::{Image, PixelLayout};
use heic_rs::props::Transform;
use heic_rs::props::simple::{Clap, Mirror, Rotation};
use heic_rs::transform;

/// A 3x2 grey image whose pixels are 0, 1, 2 / 10, 11, 12.
fn steps() -> Image {
    Image {
        data: vec![0, 1, 2, 10, 11, 12],
        width: 3,
        height: 2,
        layout: PixelLayout::Gray8,
    }
}

#[test]
fn rotation_by_zero_is_the_identity() {
    let before = steps();
    let after = transform::rotate(before.clone(), Rotation::None);
    assert_eq!(after, before);
}

#[test]
fn rotate_90_counter_clockwise() {
    let out = transform::rotate(steps(), Rotation::Ccw90);
    assert_eq!((out.width, out.height), (2, 3));
    // The top-right pixel of the source becomes the top-left of the result.
    assert_eq!(out.data, vec![2, 12, 1, 11, 0, 10]);
}

#[test]
fn rotate_180_and_270() {
    let out = transform::rotate(steps(), Rotation::Ccw180);
    assert_eq!((out.width, out.height), (3, 2));
    assert_eq!(out.data, vec![12, 11, 10, 2, 1, 0]);

    let out = transform::rotate(steps(), Rotation::Ccw270);
    assert_eq!((out.width, out.height), (2, 3));
    assert_eq!(out.data, vec![10, 0, 11, 1, 12, 2]);
}

#[test]
fn four_quarter_turns_return_the_original() {
    let mut image = steps();
    for _ in 0..4 {
        image = transform::rotate(image, Rotation::Ccw90);
    }
    assert_eq!(image, steps());
}

#[test]
fn mirroring_exchanges_the_right_edges() {
    let out = transform::mirror(steps(), Mirror::LeftRight);
    assert_eq!(out.data, vec![2, 1, 0, 12, 11, 10]);
    let out = transform::mirror(steps(), Mirror::TopBottom);
    assert_eq!(out.data, vec![10, 11, 12, 0, 1, 2]);
}

#[test]
fn mirroring_twice_returns_the_original() {
    for m in [Mirror::LeftRight, Mirror::TopBottom] {
        assert_eq!(transform::mirror(transform::mirror(steps(), m), m), steps());
    }
}

#[test]
fn mirroring_works_on_multi_byte_pixels() {
    let image = Image {
        data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        width: 2,
        height: 2,
        layout: PixelLayout::Rgb8,
    };
    let out = transform::mirror(image, Mirror::LeftRight);
    assert_eq!(out.data, vec![4, 5, 6, 1, 2, 3, 10, 11, 12, 7, 8, 9]);
}

fn clap(w: u32, h: u32, hoff: i32, voff: i32) -> Clap {
    Clap {
        width: (w, 1),
        height: (h, 1),
        horiz_off: (hoff, 1),
        vert_off: (voff, 1),
    }
}

#[test]
fn a_centred_clean_aperture_crops_the_middle() {
    // A 4x4 ramp, cropped to the centre 2x2.
    let image = Image {
        data: (0u8..16).collect(),
        width: 4,
        height: 4,
        layout: PixelLayout::Gray8,
    };
    let rect = transform::crop_rect(4, 4, clap(2, 2, 0, 0)).expect("a valid rectangle");
    assert_eq!(rect, (1, 1, 2, 2));
    let out = transform::crop(image, clap(2, 2, 0, 0)).expect("crops");
    assert_eq!((out.width, out.height), (2, 2));
    assert_eq!(out.data, vec![5, 6, 9, 10]);
}

#[test]
fn a_clean_aperture_offset_moves_the_window() {
    let image = Image {
        data: (0u8..16).collect(),
        width: 4,
        height: 4,
        layout: PixelLayout::Gray8,
    };
    let out = transform::crop(image, clap(2, 2, 1, -1)).expect("crops");
    assert_eq!(out.data, vec![2, 3, 6, 7]);
}

#[test]
fn a_clean_aperture_larger_than_the_image_is_refused() {
    assert!(matches!(
        transform::crop_rect(4, 4, clap(8, 2, 0, 0)),
        Err(Error::Malformed(_))
    ));
    assert!(matches!(
        transform::crop_rect(4, 4, clap(2, 0, 0, 0)),
        Err(Error::Malformed(_))
    ));
    // An offset that pushes the window off the edge is refused too.
    assert!(matches!(
        transform::crop_rect(4, 4, clap(2, 2, -9, 0)),
        Err(Error::Malformed(_))
    ));
    assert!(matches!(
        transform::crop_rect(4, 4, clap(2, 2, 9, 0)),
        Err(Error::Malformed(_))
    ));
}

#[test]
fn transformed_size_is_computed_without_touching_pixels() {
    let rot = [Transform::Rotate(Rotation::Ccw90)];
    assert_eq!(
        transform::transformed_size(4032, 3024, &rot).expect("valid"),
        (3024, 4032)
    );

    let flat = [
        Transform::Rotate(Rotation::Ccw180),
        Transform::Mirror(Mirror::LeftRight),
    ];
    assert_eq!(
        transform::transformed_size(4032, 3024, &flat).expect("valid"),
        (4032, 3024)
    );

    let cropped = [
        Transform::Crop(clap(100, 50, 0, 0)),
        Transform::Rotate(Rotation::Ccw270),
    ];
    assert_eq!(
        transform::transformed_size(200, 200, &cropped).expect("valid"),
        (50, 100)
    );

    assert_eq!(
        transform::transformed_size(8, 8, &[]).expect("valid"),
        (8, 8)
    );
}

#[test]
fn transforms_apply_in_the_order_ipma_listed_them() {
    // Rotating then mirroring is not the same as mirroring then rotating, so
    // the order the file gives really is load-bearing.
    let rotate_then_mirror = [
        Transform::Rotate(Rotation::Ccw90),
        Transform::Mirror(Mirror::LeftRight),
    ];
    let mirror_then_rotate = [
        Transform::Mirror(Mirror::LeftRight),
        Transform::Rotate(Rotation::Ccw90),
    ];
    let a = transform::apply_all(steps(), &rotate_then_mirror).expect("applies");
    let b = transform::apply_all(steps(), &mirror_then_rotate).expect("applies");
    assert_eq!((a.width, a.height), (2, 3));
    assert_eq!((b.width, b.height), (2, 3));
    assert_ne!(a.data, b.data);
    assert_eq!(a.data, vec![12, 2, 11, 1, 10, 0]);
    assert_eq!(b.data, vec![0, 10, 1, 11, 2, 12]);
}

#[test]
fn an_empty_transform_list_leaves_the_image_alone() {
    assert_eq!(
        transform::apply_all(steps(), &[]).expect("applies"),
        steps()
    );
}
