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

pub mod error;
pub mod hevc;
pub mod reader;

pub use crate::error::{Error, Result};
pub use crate::hevc::ChromaFormat;
