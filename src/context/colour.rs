//! What colour a file declares for an item, looking through a derivation to
//! the coded pictures it is assembled from.

use super::Context;
use crate::error::Result;
use crate::props::colr::Nclx;

impl<'a> Context<'a> {
    /// The colour an item declares: its own `colr`, or the one its coded tiles
    /// declare when the item is a derivation that carries none.
    ///
    /// `colr` is optional (ISO/IEC 23008-12, 6.5.5), and a derived item does not
    /// repeat the properties of the coded pictures it is assembled from, so the
    /// description that applies to a `grid` is the one written on its tiles. A
    /// grid is assembled from pictures of one kind, so the first tile that
    /// declares a colour speaks for all of them.
    ///
    /// `Ok(None)` means the file declares nothing, which is not the same as
    /// declaring the fallback: what an unlabelled file means is the caller's
    /// decision. Decoding then asks the coded picture, see
    /// [`pick_nclx`](crate::decode::pick_nclx).
    pub fn declared_nclx(&self, item: u32) -> Result<Option<Nclx>> {
        if let Some(own) = self.props(item)?.nclx {
            return Ok(Some(own));
        }
        let Some((_, tiles)) = self.grid(item)? else {
            return Ok(None);
        };
        for tile in tiles {
            if let Some(n) = self.props(tile)?.nclx {
                return Ok(Some(n));
            }
        }
        Ok(None)
    }

    /// The ICC profile an item declares, resolved the same way as
    /// [`declared_nclx`](Context::declared_nclx).
    ///
    /// `colr` carries either flavour, and a writer may choose the profile and
    /// still put it only on the tiles: a OnePlus 11 photograph declares one
    /// `prof` on each of its 48 tiles and nothing on the grid, with no `nclx`
    /// anywhere in the file. Asking the grid alone loses the profile, and the
    /// photograph is taken for sRGB.
    pub fn declared_icc(&self, item: u32) -> Result<Option<&'a [u8]>> {
        if let Some(own) = self.props(item)?.icc {
            return Ok(Some(own));
        }
        let Some((_, tiles)) = self.grid(item)? else {
            return Ok(None);
        };
        for tile in tiles {
            if let Some(icc) = self.props(tile)?.icc {
                return Ok(Some(icc));
            }
        }
        Ok(None)
    }
}
