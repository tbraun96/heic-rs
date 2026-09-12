# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

**0.1.0 has not been released.** The crate now decodes HEIC files to pixels end to end. No release
date is promised here because none has been set.

### Changed

- Colour conversion is now integer fixed point, and the chroma upsampler is fused into it: a
  conversion expands chroma one row at a time into a reused pair of row buffers instead of
  materialising two full-resolution planes, and applies the matrix through one bounds-check-free
  loop per pixel layout. 2048x1536 to `Rgb8` went from 22.0 ms to 2.24 ms on an Apple M3 Max, 143
  megapixels per second to 1.41 gigapixels. Output is within one least significant bit of the
  previous floating-point result, a bound `tests/color_fixed.rs` asserts against a float reference.
- `color::convert` refuses a frame whose `bit_depth` is outside 8 to 16 with `Error::Unsupported`,
  rather than scaling by a shift it cannot represent.
- `color::convert` takes a thread count, and `DecodeOptions` grows a `threads` field. Both default
  to `None`, which is rayon's pool.
- The inverse transform bounds both of its one-dimensional stages by the smallest top-left rectangle
  that holds a non-zero coefficient, which residual coding already guarantees is small. Output is
  bit-identical; a single-tile decode is about 16% faster for it.
- Intra reference availability (clause 6.4.1) is derived once per minimum transform block rather
  than once per reference sample, which is exact because the derivation reads its argument only at
  that granularity.

### Added

- ISOBMFF box reader: box walking, size and version/flags handling, 64-bit large sizes, and
  bounds-checked cursor primitives that report `Error::Truncated` instead of reading past a buffer.
- `ftyp` parsing and brand recognition for `heic`, `heix`, `heim`, `heis`, `hevc`, `mif1`, `msf1`.
- `meta` box and its children: `hdlr`, `pitm`, `iinf`/`infe`, `iref`, `iprp`/`ipco`/`ipma`, `iloc`
  (construction methods including `idat`), and `idat`.
- Item property parsing: `hvcC`, `ispe`, `pixi`, `colr` (both ICC and NCLX forms), `irot`, `imir`,
  `clap`, `auxC`, `pasp`.
- Grid (`grid`) derivation: tile enumeration via `dimg` references, grid geometry validation, and
  compositing of tiles into a single output surface.
- Transform application for `irot` and `imir`, with `DecodeOptions::apply_transforms` to disable it.
- EXIF and ICC profile extraction from their respective items.
- YUV to RGB colour conversion covering BT.601, BT.709 and BT.2020 matrices in both full and limited
  range, with chroma upsampling for 4:2:0 and 4:2:2.
- Public API: `decode`, `probe`, `DecodeOptions`, `PixelLayout`, `Image`, `ImageInfo`, `GridInfo`,
  `Error`, and `DEFAULT_MAX_PIXELS` (268435456, i.e. 256 megapixels).
- `io` module behind the default-on `std` feature, providing `decode_file` and `probe_file`. It is
  the only part of the crate that touches the filesystem.
- `no_std` + `alloc` core with zero required dependencies; `wasm32-unknown-unknown` builds with
  `--no-default-features`.
- `#![forbid(unsafe_code)]` at the crate root.
- HEVC still-picture intra decoder as the `hevc` module: NAL and parameter set parsing, CABAC, the
  coding quadtree, intra prediction, inverse DCT-II and DST-VII, dequantisation with scaling lists,
  deblocking and SAO. Main, Main 10 and Main Still Picture; 4:2:0, 4:2:2, 4:4:4 and monochrome;
  8-bit and 10-bit. Written from the specification text; see [NOTICE](NOTICE).
- `Error::MissingParameterSet`, for a slice that names a VPS, SPS or PPS the `hvcC` record does not
  carry. The decoder's own error type is converted to `Error` at the seam, carrying its message, so
  a caller still sees exactly one error type.
- `parallel` feature, on by default and implying `std`: grid tiles are decoded on a rayon pool and
  colour conversion is split into row bands. `DecodeOptions::threads` selects the pool, the serial
  path, or a private pool of a given size. Output is byte-identical in every case, which
  `tests/parallel.rs` asserts across thread counts and every pixel layout.
- Examples `decode`, `probe`, `to_png` and `throughput`; criterion benches with the
  `container_parse`, `grid_compose`, `color_convert`, `color_threads` and `decode_file` groups, and
  a `bench`-gated `hevc` bench for the decoder's internal stages.

### Known limitations

- Per thread the codec is slower than the AGPL `heic` crate on low-residual content; parallelism is
  what puts `heic-rs` ahead on anything stored as a grid. A 64x64 single-tile file is still about
  30% slower than that alternative. See the README's performance section.
- AVIF, image sequences and animation, `iovl` overlay derivation, and encoding are all out of scope
  and reported as `Error::Unsupported`.
- Inter prediction, P and B slices, dependent slice segments and the multilayer, 3D, screen-content
  and range extensions are refused by the decoder with a message naming the tool.
- No fuzzing has been run yet. See [SECURITY.md](SECURITY.md).

[Unreleased]: https://github.com/tbraun96/heic-rs/commits/main
