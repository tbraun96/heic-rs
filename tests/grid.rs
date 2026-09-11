//! `grid` parsing, tile-list checking and compositing.

mod common;

use common::*;
use heic_rs::error::Error;
use heic_rs::grid::{self, Grid};
use heic_rs::hevc::ChromaFormat;

#[test]
fn grid_payload_16_bit() {
    // The bytes sips writes for a 2x2 grid of 512x512 tiles.
    let data = [0x00, 0x00, 0x01, 0x01, 0x04, 0x00, 0x04, 0x00];
    let g = grid::parse(&data).expect("parses");
    assert_eq!(
        g,
        Grid {
            rows: 2,
            columns: 2,
            output_width: 1024,
            output_height: 1024
        }
    );
    assert_eq!(g.tile_count(), 4);

    // And for a 4-column, 3-row 2048x1536.
    let data = [0x00, 0x00, 0x02, 0x03, 0x08, 0x00, 0x06, 0x00];
    let g = grid::parse(&data).expect("parses");
    assert_eq!(
        g,
        Grid {
            rows: 3,
            columns: 4,
            output_width: 2048,
            output_height: 1536
        }
    );
    assert_eq!(g.tile_count(), 12);
}

#[test]
fn grid_payload_32_bit_when_the_flag_is_set() {
    let data = cat(&[
        &[0u8, 1, 5, 7],
        &4032u32.to_be_bytes(),
        &3024u32.to_be_bytes(),
    ]);
    let g = grid::parse(&data).expect("parses");
    assert_eq!(
        g,
        Grid {
            rows: 6,
            columns: 8,
            output_width: 4032,
            output_height: 3024
        }
    );
    assert_eq!(g.tile_count(), 48);
}

#[test]
fn grid_refuses_a_future_version_and_a_zero_size() {
    assert!(matches!(
        grid::parse(&[1, 0, 0, 0, 0, 1, 0, 1]),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        grid::parse(&[0, 0, 0, 0, 0, 0, 0, 1]),
        Err(Error::Malformed(_))
    ));
}

#[test]
fn grid_refuses_a_truncated_payload() {
    assert!(matches!(
        grid::parse(&[0, 0, 1, 1, 0x04]),
        Err(Error::Truncated(_))
    ));
    assert!(matches!(grid::parse(&[]), Err(Error::Truncated(_))));
}

#[test]
fn a_tile_count_that_disagrees_with_dimg_is_refused() {
    let g = Grid {
        rows: 2,
        columns: 2,
        output_width: 4,
        output_height: 4,
    };
    assert!(grid::check_tiles(&g, &[1, 2, 3, 4]).is_ok());
    let e = grid::check_tiles(&g, &[1, 2, 3]).expect_err("three tiles for a 2x2 grid");
    assert!(matches!(e, Error::Malformed(m) if m.contains("dimg")));
    assert!(grid::check_tiles(&g, &[1, 2, 3, 4, 5]).is_err());
}

#[test]
fn compose_lays_tiles_out_row_major() {
    let g = Grid {
        rows: 2,
        columns: 2,
        output_width: 4,
        output_height: 4,
    };
    let tiles: Vec<_> = [10u16, 20, 30, 40]
        .iter()
        .map(|v| mono_frame(2, 2, |_, _| *v))
        .collect();
    let out = grid::compose(&g, &tiles, u64::MAX).expect("composes");
    assert_eq!((out.width, out.height), (4, 4));
    // Row 0 is the top-left and top-right tiles side by side.
    assert_eq!(&out.y[0..4], &[10, 10, 20, 20]);
    assert_eq!(&out.y[4..8], &[10, 10, 20, 20]);
    assert_eq!(&out.y[8..12], &[30, 30, 40, 40]);
    assert_eq!(&out.y[12..16], &[30, 30, 40, 40]);
}

#[test]
fn compose_crops_the_overhang_to_the_declared_output_size() {
    // Three 2-wide tiles across cover 6 columns, but the grid declares 5.
    let g = Grid {
        rows: 1,
        columns: 3,
        output_width: 5,
        output_height: 2,
    };
    let tiles: Vec<_> = [1u16, 2, 3]
        .iter()
        .map(|v| mono_frame(2, 2, |_, _| *v))
        .collect();
    let out = grid::compose(&g, &tiles, u64::MAX).expect("composes");
    assert_eq!((out.width, out.height), (5, 2));
    assert_eq!(&out.y[0..5], &[1, 1, 2, 2, 3]);
    assert_eq!(&out.y[5..10], &[1, 1, 2, 2, 3]);
}

#[test]
fn compose_carries_chroma_planes() {
    let g = Grid {
        rows: 1,
        columns: 2,
        output_width: 4,
        output_height: 2,
    };
    let tiles = vec![
        solid_frame(2, 2, 100, 60, 200),
        solid_frame(2, 2, 150, 80, 210),
    ];
    let out = grid::compose(&g, &tiles, u64::MAX).expect("composes");
    assert_eq!(out.chroma, ChromaFormat::Yuv420);
    assert_eq!(out.y, vec![100, 100, 150, 150, 100, 100, 150, 150]);
    assert_eq!(out.c_stride, 2);
    assert_eq!(out.cb, vec![60, 80]);
    assert_eq!(out.cr, vec![200, 210]);
}

#[test]
fn compose_refuses_tiles_that_disagree() {
    let g = Grid {
        rows: 1,
        columns: 2,
        output_width: 4,
        output_height: 2,
    };
    let mismatched = vec![mono_frame(2, 2, |_, _| 1), mono_frame(3, 2, |_, _| 2)];
    assert!(matches!(
        grid::compose(&g, &mismatched, u64::MAX),
        Err(Error::Malformed(_))
    ));

    let wrong_format = vec![mono_frame(2, 2, |_, _| 1), solid_frame(2, 2, 1, 1, 1)];
    assert!(matches!(
        grid::compose(&g, &wrong_format, u64::MAX),
        Err(Error::Malformed(_))
    ));

    let too_few = vec![mono_frame(2, 2, |_, _| 1)];
    assert!(matches!(
        grid::compose(&g, &too_few, u64::MAX),
        Err(Error::Malformed(_))
    ));
    assert!(matches!(
        grid::compose(&g, &[], u64::MAX),
        Err(Error::Malformed(_))
    ));
}

#[test]
fn compose_refuses_tiles_that_do_not_cover_the_output() {
    // Two 2-wide tiles cover 4 columns; the grid claims 9.
    let g = Grid {
        rows: 1,
        columns: 2,
        output_width: 9,
        output_height: 2,
    };
    let tiles = vec![mono_frame(2, 2, |_, _| 1), mono_frame(2, 2, |_, _| 2)];
    let e = grid::compose(&g, &tiles, u64::MAX).expect_err("tiles are too small");
    assert!(matches!(e, Error::Malformed(m) if m.contains("cover")));
}

#[test]
fn compose_respects_the_pixel_limit() {
    let g = Grid {
        rows: 1,
        columns: 2,
        output_width: 4,
        output_height: 2,
    };
    let tiles = vec![mono_frame(2, 2, |_, _| 1), mono_frame(2, 2, |_, _| 2)];
    let e = grid::compose(&g, &tiles, 4).expect_err("8 pixels is over a 4 pixel limit");
    assert_eq!(
        e,
        Error::PixelLimit {
            pixels: 8,
            max_pixels: 4
        }
    );
}

#[test]
fn a_frame_shorter_than_its_geometry_is_refused() {
    let mut f = mono_frame(4, 4, |_, _| 1);
    f.y.truncate(8);
    let g = Grid {
        rows: 1,
        columns: 1,
        output_width: 4,
        output_height: 4,
    };
    assert!(matches!(
        grid::compose(&g, &[f], u64::MAX),
        Err(Error::Malformed(_))
    ));
}
