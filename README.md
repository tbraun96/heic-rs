# heic-rs

A pure-Rust HEIC/HEIF image decoder: no C toolchain, no `unsafe`, `no_std`-friendly, wasm-ready.

[![crates.io](https://img.shields.io/crates/v/heic-rs.svg)](https://crates.io/crates/heic-rs)
[![docs.rs](https://img.shields.io/docsrs/heic-rs)](https://docs.rs/heic-rs)
[![CI](https://github.com/tbraun96/heic-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/tbraun96/heic-rs/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## Status

**This crate does not decode pixels yet. Read this section before you depend on it.**

The container half is complete:

- ISOBMFF box parsing, `ftyp` brand handling.
- `meta` and its children: `hdlr`, `pitm`, `iinf`/`infe`, `iref`, `iprp`/`ipco`/`ipma`, `iloc`, `idat`.
- Item properties: `hvcC`, `ispe`, `pixi`, `colr`, `irot`, `imir`, `clap`, `auxC`, `pasp`.
- Grid (`grid`) derivation, tile compositing, and rotation/mirror transforms.
- EXIF and ICC profile extraction.
- YUV to RGB colour conversion (BT.601, BT.709, BT.2020; full and limited range) with chroma upsampling.

The HEVC still-picture decoder is **not linked in**. It is being written as a separate crate and will
land here as the `hevc` module. Until it does:

- `probe()` works fully today. It reads the container and never decodes pixels.
- `decode()` walks the whole container, resolves the item, assembles the grid plan, reaches the codec
  seam, and returns `Error::Unsupported("the HEVC decoder is not linked in this build")`.

Nothing in this README should be read as a claim that the crate turns a HEIC file into pixels today.
It does not. When the decoder lands, this section and the [CHANGELOG](CHANGELOG.md) will say so.

## Why pure Rust

- **No C toolchain.** No `cmake`, no `pkg-config`, no `libheif`, no vendored C sources, no build
  script that shells out. `cargo build` is the whole story, which means the crate cross-compiles and
  vendors trivially.
- **`#![forbid(unsafe_code)]`.** Enforced at the crate root, not by convention. A parser bug is a
  wrong answer or a panic, not a memory-safety vulnerability.
- **Wasm-ready.** The core is `no_std` + `alloc`; `wasm32-unknown-unknown` is a checked CI target.
- **Permissive licence.** MIT OR Apache-2.0, the ordinary Rust dual licence, with no copyleft
  obligations flowing into your binary.

### How it compares

| Crate | Language | Licence | unsafe | wasm | C toolchain needed |
|---|---|---|---|---|---|
| `heic-rs` | Rust | MIT OR Apache-2.0 | none (`#![forbid(unsafe_code)]`) | yes | no |
| `libheif-rs` | Rust bindings to C `libheif` | LGPL-3.0 for the library, and in practice GPL-encumbered codec plugins | yes, FFI throughout | no | yes |
| `heic` (imazen) | pure Rust | AGPL-3.0 | — | — | no |
| Browser (`<img>`, `createImageBitmap`) | — | — | — | — | — |

Notes on that table:

- `libheif-rs` wraps the C library `libheif`. The library itself is LGPL-3.0, and the HEVC codec
  plugins usually shipped alongside it (`libde265`, `x265`) carry their own stronger terms, so in
  practice a deployment built this way is GPL-encumbered. You also inherit a C build dependency.
- `heic` by imazen is genuinely pure Rust, but it is AGPL-3.0. For most commercial use that is a
  non-starter, and no amount of engineering quality changes that.
- Browsers: as of writing, Safari decodes HEIC in `<img>` and `createImageBitmap`, while Chrome and
  Firefox do not. Browser support moves; check current data before relying on either answer.

**The licence difference is the point.** There is no shortage of ways to decode HEIC. There is a
shortage of permissively licensed ones that do not drag a C toolchain along. `heic-rs` is meant to be
the permissive option.

## Quick start

```toml
[dependencies]
heic-rs = "0.1"
```

Inspect a file without decoding it. This path works today:

```rust
let bytes = std::fs::read("photo.heic")?;
let info = heic_rs::probe(&bytes)?;
println!("{}x{} bit_depth={} grid={}", info.width, info.height, info.bit_depth, info.is_grid);
```

Decode to pixels. This path reaches the codec seam and currently returns `Error::Unsupported`:

```rust
use heic_rs::{decode, DecodeOptions, PixelLayout};

let bytes = std::fs::read("photo.heic")?;
let options = DecodeOptions { layout: PixelLayout::Rgba8, ..DecodeOptions::default() };
match decode(&bytes, &options) {
    Ok(image) => println!("{}x{} -> {} bytes", image.width, image.height, image.data.len()),
    // Today: "the HEVC decoder is not linked in this build".
    Err(e) => eprintln!("decode failed: {e}"),
}
```

With the `std` feature (on by default) the `io` module saves you the `read` call:

```rust
let info = heic_rs::io::probe_file("photo.heic")?;
let image = heic_rs::io::decode_file("photo.heic", &DecodeOptions::default())?;
```

Runnable versions live in `examples/`: `decode.rs`, `probe.rs`, `to_png.rs`.

```
cargo run --example probe -- photo.heic
cargo run --example decode -- photo.heic
cargo run --example to_png -- photo.heic out.png
```

## API tour

```rust
pub fn decode(bytes: &[u8], options: &DecodeOptions) -> Result<Image, Error>;
pub fn probe(bytes: &[u8]) -> Result<ImageInfo, Error>;

pub struct DecodeOptions {
    pub layout: PixelLayout,
    pub max_pixels: Option<u64>,
    pub apply_transforms: bool,
    pub decode_alpha: bool,
    pub strict: bool,
}

pub enum PixelLayout { Rgb8, Rgba8, Bgr8, Bgra8, Gray8, Rgb16, Rgba16 }

pub struct Image {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub layout: PixelLayout,
}

pub struct ImageInfo {
    // width, height, coded_width, coded_height, bit_depth, chroma,
    // rotation, mirror, has_alpha, is_grid, grid: Option<GridInfo>,
    // has_exif, has_icc, brand
}

// std feature only:
pub mod io {
    pub fn decode_file<P: AsRef<Path>>(path: P, options: &DecodeOptions) -> Result<Image, Error>;
    pub fn probe_file<P: AsRef<Path>>(path: P) -> Result<ImageInfo, Error>;
}
```

`DecodeOptions::default()` decodes to `PixelLayout::Rgb8` with
`max_pixels = Some(DEFAULT_MAX_PIXELS)`, where:

```rust
pub const DEFAULT_MAX_PIXELS: u64 = 268_435_456; // 256 megapixels
```

`ImageInfo` reports both the displayed size (`width`, `height`, after transforms) and the coded size
(`coded_width`, `coded_height`, before them), so a caller can tell an `irot` apart from a genuinely
tall image.

## The SBIO rule

**Business logic performs no I/O.** The decoding path takes `&[u8]` and returns pixels. It never
opens a file, never resolves a path, never allocates a buffer from a filename, and never touches the
network. Every filesystem convenience lives in the `io` module behind the `std` feature, and that
module does exactly one thing: read bytes, then call the same pure function you could have called.

This is not stylistic. It is what makes the core `no_std` and wasm-ready, it is what lets a fuzzer
drive the parser directly with a byte slice, and it is what keeps the interesting code testable
without a filesystem.

## `no_std` and wasm

The core is `no_std` + `alloc`. Turn off default features to drop `std` and the `io` module:

```toml
[dependencies]
heic-rs = { version = "0.1", default-features = false }
```

CI checks the wasm target on every push:

```
cargo check --target wasm32-unknown-unknown --no-default-features
```

There are zero required dependencies. `criterion` is a dev-dependency for benches only and never
reaches your build.

## Security

HEIF files are untrusted input; treat them that way. The crate is `#![forbid(unsafe_code)]`, library
code contains no `unwrap`/`expect`/`panic!`, and every parser is bounds-checked and reports
`Error::Truncated` or `Error::Malformed` rather than reading past a buffer.

The main resource-exhaustion lever is yours: `DecodeOptions::max_pixels` caps the pixel count before
anything is allocated, defaulting to `DEFAULT_MAX_PIXELS` (268435456, i.e. 256 Mpx). Lower it for
hostile input; set it to `None` only when you control the files.

To report a vulnerability, see [SECURITY.md](SECURITY.md).

## Performance

**`heic-rs` has no end-to-end decode numbers of its own yet**, because the HEVC decoder has not
landed. The container, compositing and colour figures below are real and measured; only the
end-to-end number is missing. Publishing a throughput figure for a decoder that returns `Error::Unsupported` would be
dishonest. Our own numbers go here once the decoder lands.

For context on the target, here is a measured baseline from the AGPL alternative (`heic` crate by
imazen 0.1.6), measured on an Apple M-series Mac, release build, best of five after a warm-up,
decoding to RGB8, on a corpus of four HEICs generated from our own synthetic PNGs with `sips`:

| file | pixels | best | throughput |
|---|---|---|---|
| flat-64.heic | 64x64 | 0.03 ms | 149 Mpx/s |
| gradient-512.heic | 512x512 | 2.37 ms | 110 Mpx/s |
| checker-1024.heic | 1024x1024 | 5.13 ms | 204 Mpx/s |
| photo-2048.heic | 2048x1536 | 8.54 ms | 369 Mpx/s |

That crate is **not** a dependency of `heic-rs` and is not used by it in any way. It was measured in
a separate harness outside this repository, and the numbers are reproduced here only as a reference
point for what a pure-Rust HEIC decoder achieves on this class of hardware. Treat them as
context, not as a benchmark of this crate.

What we can measure today is the container half. The benches are criterion:

```
cargo bench
```

Three groups, all of which exercise code that is finished. Measured on an Apple M-series Mac,
release build, criterion's median of a three-second measurement window:

| group | case | median |
|---|---|---|
| `container_parse` | `flat-64.heic` (single item) | 544 ns |
| `container_parse` | `checker-1024.heic` (2x2 grid) | 870 ns |
| `container_parse` | `photo-2048.heic` (3x4 grid, 13 items) | 1.47 us |
| `grid_compose` | 2x2 tiles into 1024x1024 | 89.6 us |
| `grid_compose` | 3x4 tiles into 2048x1536 | 348 us |
| `grid_compose` | 6x8 tiles into 4032x3024 (the iPhone shape) | 2.71 ms |
| `color_convert` | 512x512 to Rgb8 | 1.84 ms |
| `color_convert` | 512x512 to Rgba8 | 1.94 ms |
| `color_convert` | 2048x1536 to Rgb8 | 22.1 ms |
| `color_convert` | 2048x1536 to Rgba8 | 23.3 ms |

Reading the container is free: under two microseconds for a thirteen-item file, so probing a
directory of photographs costs nothing. Compositing runs at about 4.5 gigapixels per second.

**Colour conversion is the slow part and we are not going to pretend otherwise.** At 2048x1536 it
takes 22 ms, about 140 megapixels per second, which is more than the AGPL crate spends decoding the
entire file. Two reasons, both fixable and neither yet fixed: `upsample::plane` materialises two
full-resolution chroma planes before the matrix runs, and the matrix itself is scalar `f32` with one
bounds-checked index per sample. A fused per-pixel chroma fetch and an integer fixed-point matrix
are the obvious next steps, and they are on the roadmap rather than in the code. The filter lives in
one small module precisely so that it can be replaced without disturbing anything else.

## Correctness

`heic-rs` measures its correctness against the **platform decoder**, not against another Rust crate.

Fixtures are encoded with macOS `sips -s format heic`, and the ground truth is whatever Apple's own
HEIC decoder produces for them:

```
sips -s format png <file>.heic --out <file>.ref.png
```

The pixel tests compare our output against those reference PNGs by PSNR. They are `#[ignore]`d until
the HEVC decoder lands, at which point they become the gate on it.

Comparing against the OS decoder is deliberate. Differential-testing against another implementation
means inheriting that implementation's bugs as "expected" output, and in this case it would also mean
running AGPL or LGPL code in our test harness. The platform decoder is independent of both problems.

## Supported and not supported

Supported:

- HEIC/HEIF brands: `heic`, `heix`, `heim`, `heis`, `hevc`, `mif1`, `msf1`.
- Single-item images and grid-derived images (`grid`), including tile compositing.
- Transform properties: `irot` (rotation), `imir` (mirror), `clap` (clean aperture).
- EXIF and ICC profile extraction.
- 8-bit and 10-bit samples; 4:2:0, 4:2:2, 4:4:4, and monochrome chroma formats.

Grid support is not optional for reading real files. macOS `sips` begins tiling above 512 px on a
side, so anything larger is stored as a `grid` derivation of 512x512 tiles: a 1024x1024 image becomes
2x2, and 2048x1536 becomes 4x3. The common iPhone shape, 4032x3024, arrives as 8 columns by 6 rows.
A decoder that handles only single-item images will fail on most photographs an iPhone or a Mac
actually produces.

Not supported, and reported as `Error::Unsupported` with a message naming the reason:

- AVIF. The error message names AVIF explicitly; it is a different codec in the same container
  family, and this crate does not decode it.
- Image sequences and animation.
- `iovl` overlay derivation.
- Encoding. This is a decoder.
- For now, actual HEVC pixel decoding, per the [Status](#status) section.

## Roadmap

1. Land the HEVC still-picture decoder as the `hevc` module, replacing the `Unsupported` seam in
   `decode()`.
2. Publish end-to-end decode benchmarks for this crate, on the same corpus as the baseline above.
3. Make colour conversion fast: fuse the chroma fetch into the matrix loop instead of
   materialising two full-resolution planes, and move the matrix to integer fixed point. The
   benchmark above says this is the crate's slowest finished code by a wide margin.
4. Run the fuzz targets described in [SECURITY.md](SECURITY.md) and fix whatever they find.
5. Alpha (`auxC`) decoding end to end, once the codec is in place.
6. Consider `iovl` overlay derivation and 12-bit samples, in that order.

Encoding is not on the roadmap.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this
crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
