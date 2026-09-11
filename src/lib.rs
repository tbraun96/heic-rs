//! A pure-Rust decoder for HEIC and HEIF still images.
//!
//! No C, no `unsafe`, no required dependencies, and a `no_std` core that
//! builds for `wasm32-unknown-unknown`.
//!
//! Business logic performs no I/O: the decoding path takes `&[u8]` and returns
//! values. Filesystem convenience lives in the `io` module, behind the
//! default-on `std` feature, and nowhere else.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod boxes;
pub mod error;
pub mod ftyp;
pub mod grid;
pub mod hevc;
pub mod image;
pub mod meta;
pub mod props;
pub mod reader;

pub use crate::error::{Error, Result};
pub use crate::ftyp::Brand;
pub use crate::grid::Grid;
pub use crate::hevc::ChromaFormat;
pub use crate::image::{DEFAULT_MAX_PIXELS, DecodeOptions, Image, PixelLayout};
pub use crate::props::Transform;
pub use crate::props::colr::{MatrixCoefficients, Nclx, Range};
pub use crate::props::simple::{Mirror, Rotation};
