//! `Mosaic`: a grid's tiles read as rows, and the proof that reading them in
//! place gives the same picture as composing them onto a canvas.

mod common;

use common::*;
use heic_rs::error::Error;
use heic_rs::grid::{self, Grid, Mosaic};
use heic_rs::hevc::{ChromaFormat, Frame};
use heic_rs::{DecodeOptions, PixelLayout};

/// A 4:2:0 frame whose every sample differs, seeded so tiles differ too.
fn textured(w: u32, h: u32, seed: u16) -> Frame {
    let (cw, ch) = ChromaFormat::Yuv420.chroma_size(w, h);
    let fill = |n: u32, k: u16| (0..n).map(|i| seed.wrapping_mul(k) + i as u16).collect();
    Frame {
        width: w,
        height: h,
        bit_depth: 8,
        chroma: ChromaFormat::Yuv420,
        y: fill(w * h, 7),
        cb: fill(cw * ch, 11),
        cr: fill(cw * ch, 13),
        y_stride: w,
        c_stride: cw,
    }
}

#[test]
fn mosaic_rows_are_the_rows_compose_writes() {
    // Three 4x4 tiles across and two down cover 12x8; the grid declares 11x7,
    // so the last column and row of tiles overhang and are clipped.
    let g = Grid {
        rows: 2,
        columns: 3,
        output_width: 11,
        output_height: 7,
    };
    let tiles: Vec<_> = (1..=6).map(|s| textured(4, 4, s)).collect();
    let composed = grid::compose(&g, &tiles, u64::MAX).expect("composes");
    let m = Mosaic::new(&g, &tiles).expect("a valid mosaic");
    assert_eq!((m.width(), m.height()), (11, 7));
    assert_eq!(m.chroma_size(), (6, 4));

    let mut row = vec![0u16; 11];
    for y in 0..7 {
        m.luma_row(y, &mut row).expect("a luma row");
        assert_eq!(&row[..], &composed.y[y * 11..(y + 1) * 11], "luma row {y}");
    }
    let mut row = vec![0u16; 6];
    for r in 0..4 {
        m.chroma_row(false, r, &mut row).expect("a cb row");
        assert_eq!(&row[..], &composed.cb[r * 6..(r + 1) * 6], "cb row {r}");
        m.chroma_row(true, r, &mut row).expect("a cr row");
        assert_eq!(&row[..], &composed.cr[r * 6..(r + 1) * 6], "cr row {r}");
    }
    // Past the picture there is nothing to read.
    assert!(m.luma_row(7, &mut [0u16; 11]).is_none());
}

#[test]
fn mosaic_refuses_what_compose_refuses() {
    let g = Grid {
        rows: 1,
        columns: 2,
        output_width: 6,
        output_height: 2,
    };
    // 4:2:0 tiles three samples wide put the second tile's origin between
    // two chroma samples.
    let unsited = vec![textured(3, 2, 1), textured(3, 2, 2)];
    assert!(matches!(
        Mosaic::new(&g, &unsited),
        Err(Error::Unsupported(_))
    ));
    let too_small = vec![textured(2, 2, 1), textured(2, 2, 2)];
    assert!(matches!(
        Mosaic::new(&g, &too_small),
        Err(Error::Malformed(_))
    ));
    let mismatched = vec![textured(4, 2, 1), mono_frame(4, 2, |_, _| 1)];
    assert!(matches!(
        Mosaic::new(&g, &mismatched),
        Err(Error::Malformed(_))
    ));
}

/// The decoder reads a grid's tiles in place. This is the same picture the
/// canvas path produces, byte for byte, on a real four-tile file.
#[test]
fn a_decoded_grid_matches_its_composed_frame() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/checker-1024.heic"
    );
    let bytes = std::fs::read(path).expect("the fixture");
    let ctx = heic_rs::context::Context::open(&bytes).expect("opens");
    let id = ctx.meta.primary;
    let (g, ids) = ctx.grid(id).expect("resolves").expect("is a grid");
    let tiles: Vec<Frame> = ids
        .iter()
        .map(|t| {
            let p = ctx.props(*t).expect("tile props");
            let hvcc = p.hvcc.as_ref().expect("hvcC");
            let data = ctx.item_data(*t).expect("tile data");
            let nals = hvcc.split_nals(&data).expect("nals");
            heic_rs::hevc::decode_still(&hvcc.parameter_sets(), &nals).expect("decodes")
        })
        .collect();
    let composed = grid::compose(&g, &tiles, u64::MAX).expect("composes");
    let nclx = ctx.props(id).expect("props").nclx.unwrap_or_default();
    for layout in [PixelLayout::Rgb8, PixelLayout::Bgra8, PixelLayout::Rgb16] {
        let canvas = heic_rs::color::convert(&composed, None, nclx, layout, u64::MAX, Some(1))
            .expect("converts");
        for threads in [Some(1), None] {
            let options = DecodeOptions::default()
                .with_layout(layout)
                .with_threads(threads);
            let direct = heic_rs::decode(&bytes, &options).expect("decodes");
            assert_eq!(direct, canvas, "{layout:?} on {threads:?} threads");
        }
    }
}
