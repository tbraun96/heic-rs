//! The `grid` derived item: a picture stored as a mosaic of coded tiles.
//!
//! macOS begins tiling above 512 pixels on a side, so an ordinary iPhone
//! photograph is a grid, not a single coded picture. A 4032x3024 photograph is
//! 8 columns by 6 rows of 512x512 tiles; the last column and row overhang the
//! declared output size and are cropped away here.

use alloc::vec::Vec;

use crate::error::{Error, Result};
use crate::hevc::{ChromaFormat, Frame};
use crate::image::check_pixels;
use crate::reader::Reader;

/// The parsed payload of a `grid` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// Tiles down.
    pub rows: u32,
    /// Tiles across.
    pub columns: u32,
    /// Width of the composed image after cropping.
    pub output_width: u32,
    /// Height of the composed image after cropping.
    pub output_height: u32,
}

impl Grid {
    /// How many tiles the `dimg` reference must list.
    pub const fn tile_count(&self) -> u64 {
        self.rows as u64 * self.columns as u64
    }
}

/// Parse a `grid` item payload.
///
/// The two dimensions are 16-bit unless bit 0 of `flags` is set, in which case
/// they are 32-bit.
pub fn parse(data: &[u8]) -> Result<Grid> {
    let mut r = Reader::new(data);
    let version = r.u8("grid version")?;
    if version != 0 {
        return Err(Error::Unsupported("grid version above 0"));
    }
    let flags = r.u8("grid flags")?;
    let rows = u32::from(r.u8("grid rows")?) + 1;
    let columns = u32::from(r.u8("grid columns")?) + 1;
    let (output_width, output_height) = if flags & 1 != 0 {
        (r.u32("grid output width")?, r.u32("grid output height")?)
    } else {
        (
            u32::from(r.u16("grid output width")?),
            u32::from(r.u16("grid output height")?),
        )
    };
    if output_width == 0 || output_height == 0 {
        return Err(Error::Malformed("grid declares a zero output dimension"));
    }
    Ok(Grid {
        rows,
        columns,
        output_width,
        output_height,
    })
}

/// Check that a `dimg` tile list matches what the grid asked for.
pub fn check_tiles(grid: &Grid, tiles: &[u32]) -> Result<()> {
    if tiles.len() as u64 != grid.tile_count() {
        return Err(Error::Malformed(
            "the grid's tile count disagrees with the length of its dimg reference",
        ));
    }
    Ok(())
}

/// Compose decoded tiles into one frame, cropped to the grid's output size.
///
/// Tiles are laid out row-major in the order the `dimg` reference listed them,
/// which is the order the caller must pass them in.
pub fn compose(grid: &Grid, tiles: &[Frame], max_pixels: u64) -> Result<Frame> {
    if tiles.len() as u64 != grid.tile_count() {
        return Err(Error::Malformed(
            "the grid's tile count disagrees with the number of decoded tiles",
        ));
    }
    let first = tiles.first().ok_or(Error::Malformed("grid has no tiles"))?;
    first.validate()?;
    let (tw, th) = (first.width, first.height);
    let (chroma, depth) = (first.chroma, first.bit_depth);
    for t in tiles {
        t.validate()?;
        if t.width != tw || t.height != th {
            return Err(Error::Malformed("grid tiles are not all the same size"));
        }
        if t.chroma != chroma || t.bit_depth != depth {
            return Err(Error::Malformed("grid tiles do not share a pixel format"));
        }
    }
    // The mosaic must be at least as large as the crop it promises.
    let canvas_w = u64::from(tw) * u64::from(grid.columns);
    let canvas_h = u64::from(th) * u64::from(grid.rows);
    if canvas_w < u64::from(grid.output_width) || canvas_h < u64::from(grid.output_height) {
        return Err(Error::Malformed(
            "grid tiles do not cover the declared output size",
        ));
    }
    check_pixels(grid.output_width, grid.output_height, max_pixels)?;

    let (out_cw, out_ch) = chroma.chroma_size(grid.output_width, grid.output_height);
    let mut out = Frame {
        width: grid.output_width,
        height: grid.output_height,
        bit_depth: depth,
        chroma,
        y: alloc::vec![0u16; grid.output_width as usize * grid.output_height as usize],
        cb: alloc::vec![0u16; out_cw as usize * out_ch as usize],
        cr: alloc::vec![0u16; out_cw as usize * out_ch as usize],
        y_stride: grid.output_width,
        c_stride: out_cw,
    };
    let (xs, ys) = (chroma.x_shift(), chroma.y_shift());
    for (i, tile) in tiles.iter().enumerate() {
        let row = i as u32 / grid.columns;
        let col = i as u32 % grid.columns;
        let (dx, dy) = (col * tw, row * th);
        blit(
            &mut out.y,
            out.y_stride,
            grid.output_width,
            grid.output_height,
            &tile.y,
            tile.y_stride,
            tw,
            th,
            dx,
            dy,
        );
        if chroma == ChromaFormat::Monochrome {
            continue;
        }
        // Tiles must start on a chroma sample or the mosaic cannot be
        // reassembled without resampling, which would be a silent quality loss.
        if (dx >> xs) << xs != dx || (dy >> ys) << ys != dy {
            return Err(Error::Unsupported(
                "grid tile origin is not on a chroma sample",
            ));
        }
        let (tcw, tch) = chroma.chroma_size(tw, th);
        for (plane, src) in [(&mut out.cb, &tile.cb), (&mut out.cr, &tile.cr)] {
            blit(
                plane,
                out.c_stride,
                out_cw,
                out_ch,
                src,
                tile.c_stride,
                tcw,
                tch,
                dx >> xs,
                dy >> ys,
            );
        }
    }
    Ok(out)
}

/// Copy a rectangle into a plane, clipping anything that overhangs.
#[allow(clippy::too_many_arguments)]
fn blit(
    dst: &mut [u16],
    dst_stride: u32,
    dst_w: u32,
    dst_h: u32,
    src: &[u16],
    src_stride: u32,
    src_w: u32,
    src_h: u32,
    dx: u32,
    dy: u32,
) {
    if dx >= dst_w || dy >= dst_h {
        return;
    }
    let copy_w = core::cmp::min(src_w, dst_w - dx) as usize;
    let copy_h = core::cmp::min(src_h, dst_h - dy) as usize;
    for row in 0..copy_h {
        let s = row * src_stride as usize;
        let d = (dy as usize + row) * dst_stride as usize + dx as usize;
        let (Some(srow), Some(drow)) = (src.get(s..s + copy_w), dst.get_mut(d..d + copy_w)) else {
            return;
        };
        drow.copy_from_slice(srow);
    }
}

/// Collect the tile item ids a grid derives from, in order.
pub fn tile_ids(refs: &[crate::meta::iref::ItemReference], grid_item: u32) -> Vec<u32> {
    crate::meta::iref::targets(refs, grid_item, crate::meta::iref::RefKind::Dimg).to_vec()
}
