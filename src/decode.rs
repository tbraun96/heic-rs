//! [`decode`]: container to pixels.
//!
//! Every step but one is implemented here and tested: resolving the primary
//! item, splitting NAL units, composing a grid, converting colour, applying
//! transforms. The one step that is not is the call into
//! [`crate::hevc::decode_still`], which is still a placeholder.

use alloc::vec::Vec;

use crate::color;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::grid;
use crate::hevc::{self, Frame};
use crate::image::{DecodeOptions, Image, check_pixels};
use crate::props::ItemProps;
use crate::props::colr::Nclx;
use crate::transform;

/// Decode the primary image of a HEIF file.
///
/// # Errors
///
/// Returns [`Error::Unsupported`] at the codec seam until the HEVC decoder
/// lands. Every container-level failure is reported before that point, so a
/// malformed file still gets a precise error today.
pub fn decode(bytes: &[u8], options: &DecodeOptions) -> Result<Image> {
    let ctx = Context::open(bytes)?;
    let id = ctx.meta.primary;
    let p = ctx.props(id)?;
    if options.strict && ctx.meta.props.unknown_essential(id).is_some() {
        return Err(Error::Unsupported(
            "the primary item carries an essential property this build does not implement",
        ));
    }
    let limit = options.pixel_limit();
    let (cw, ch) = ctx.coded_size(id, &p)?;
    // Refuse the declared size before decoding anything, so that a small file
    // claiming an enormous image costs nothing.
    check_pixels(cw, ch, limit)?;
    let (tw, th) = transform::transformed_size(cw, ch, &p.transforms)?;
    check_pixels(tw, th, limit)?;

    let frame = decode_item(&ctx, id, limit)?;
    let alpha = alpha_frame(&ctx, id, options, limit)?;
    let nclx = p.nclx.unwrap_or_default();
    let image = color::convert(&frame, alpha.as_ref(), nclx, options.layout, limit)?;
    if options.apply_transforms {
        transform::apply_all(image, &p.transforms)
    } else {
        Ok(image)
    }
}

/// Decode one item, following a `grid` derivation when there is one.
fn decode_item(ctx: &Context<'_>, id: u32, limit: u64) -> Result<Frame> {
    match ctx.grid(id)? {
        Some((g, tiles)) => {
            let mut frames = Vec::with_capacity(tiles.len());
            for t in &tiles {
                frames.push(decode_coded(ctx, *t)?);
            }
            grid::compose(&g, &frames, limit)
        }
        None => decode_coded(ctx, id),
    }
}

/// Decode a single coded picture item through the codec seam.
fn decode_coded(ctx: &Context<'_>, id: u32) -> Result<Frame> {
    let p = ctx.props(id)?;
    let hvcc = p.hvcc.as_ref().ok_or(Error::MissingBox("hvcC"))?;
    let data = ctx.item_data(id)?;
    let nals = hvcc.split_nals(&data)?;
    let params = hvcc.parameter_sets();
    if params.is_empty() {
        return Err(Error::Malformed("hvcC carries no parameter sets"));
    }
    let frame = hevc::decode_still(&params, &nals)?;
    frame.validate()?;
    Ok(frame)
}

/// Decode the alpha auxiliary item, when the caller asked for alpha and the
/// requested layout has somewhere to put it.
fn alpha_frame(
    ctx: &Context<'_>,
    id: u32,
    options: &DecodeOptions,
    limit: u64,
) -> Result<Option<Frame>> {
    if !options.decode_alpha || !options.layout.has_alpha() {
        return Ok(None);
    }
    let Some(aux) = ctx.alpha_item(id)? else {
        return Ok(None);
    };
    Ok(Some(decode_item(ctx, aux, limit)?))
}

/// The colour description that will be used for an item, including the
/// documented fallback for files that carry no `colr`.
pub fn effective_nclx(p: &ItemProps<'_>) -> Nclx {
    p.nclx.unwrap_or_default()
}
