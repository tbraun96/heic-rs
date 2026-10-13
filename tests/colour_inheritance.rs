//! An item that carries no `colr` of its own: a `grid` whose tiles do.
//!
//! Canon writes the colour description on the tiles and leaves the grid bare,
//! which is the shape that used to be converted with the fallback: duller than
//! the file's own thumbnail, and never tone mapped.

mod common;

use common::synth::*;
use common::*;
use heic_rs::context::Context;
use heic_rs::props::colr::{MatrixCoefficients, Nclx, Range};

/// A `colr` box carrying an `nclx`.
fn colr(primaries: u16, transfer: u16, matrix: u16, full: bool) -> Vec<u8> {
    bx(
        b"colr",
        &cat(&[
            b"nclx",
            &primaries.to_be_bytes(),
            &transfer.to_be_bytes(),
            &matrix.to_be_bytes(),
            &[u8::from(full) << 7],
        ]),
    )
}

/// BT.2020 primaries, PQ transfer, BT.2020 non-constant matrix, full range:
/// what a Canon HDR still declares.
fn pq() -> Vec<u8> {
    colr(9, 16, 9, true)
}

/// The `nclx` [`pq`] describes.
fn pq_expected() -> Option<Nclx> {
    Some(Nclx {
        primaries: 9,
        transfer: 16,
        matrix: MatrixCoefficients::Bt2020Ncl,
        matrix_code: 9,
        range: Range::Full,
    })
}

/// A file whose primary item is a 1x2 `grid` over two `hvc1` tiles. The grid
/// declares `ispe` and nothing else; each tile declares `colr` when its entry
/// says so.
fn grid_over_two_tiles(tile_colr: [bool; 2]) -> Vec<u8> {
    let payload = grid_payload(1, 2, 4, 2);
    let mut ipco = vec![ispe(4, 2)];
    if tile_colr.iter().any(|declares| *declares) {
        ipco.push(pq());
    }
    let assoc = |declares: bool| {
        if declares {
            vec![(1, true), (2, true)]
        } else {
            vec![(1, true)]
        }
    };
    file(
        &[
            pitm(1),
            iinf(&[
                infe(1, b"grid", false),
                infe(2, b"hvc1", true),
                infe(3, b"hvc1", true),
            ]),
            iref(&[iref_entry(b"dimg", 1, &[2, 3])]),
            iprp_of(
                &ipco,
                &[
                    (1, vec![(1, true)]),
                    (2, assoc(tile_colr[0])),
                    (3, assoc(tile_colr[1])),
                ],
            ),
            bx(b"idat", &payload),
            iloc(&[(1, 1, 0, payload.len() as u32), (2, 0, 0, 4), (3, 0, 0, 4)]),
        ],
        &[0; 4],
    )
}

#[test]
fn a_grid_without_its_own_colr_inherits_the_tiles_colour() {
    let bytes = grid_over_two_tiles([true, true]);
    let ctx = Context::open(&bytes).expect("opens");
    // The shape under test: the grid itself declares nothing.
    assert!(ctx.props(1).expect("gathers").nclx.is_none());
    assert_eq!(ctx.declared_nclx(1).expect("resolves"), pq_expected());
}

#[test]
fn the_first_tile_that_declares_a_colour_speaks_for_the_grid() {
    // Tiles of one picture agree in practice; a tile that says nothing does not
    // stop the search.
    let bytes = grid_over_two_tiles([false, true]);
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.declared_nclx(1).expect("resolves"), pq_expected());
}

#[test]
fn a_grid_whose_tiles_say_nothing_declares_nothing() {
    let bytes = grid_over_two_tiles([false, false]);
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.declared_nclx(1).expect("resolves"), None);
}

#[test]
fn a_coded_picture_without_colr_declares_nothing() {
    let bytes = file(
        &[
            pitm(1),
            iinf(&[infe(1, b"hvc1", false)]),
            iprp_of(&[ispe(4, 2)], &[(1, vec![(1, true)])]),
            iloc(&[(1, 0, 0, 4)]),
        ],
        &[0; 4],
    );
    let ctx = Context::open(&bytes).expect("opens");
    assert_eq!(ctx.declared_nclx(1).expect("resolves"), None);
}
