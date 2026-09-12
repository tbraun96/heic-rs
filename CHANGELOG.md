# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

**0.1.0 has not been released.** The container half of the crate is complete, but the HEVC
still-picture decoder is not linked in, so `decode()` cannot yet produce pixels. There will be no
release until it can. No release date is promised here because none has been set.

### Changed

- Colour conversion is now integer fixed point, and the chroma upsampler is fused into it: a
  conversion expands chroma one row at a time into a reused pair of row buffers instead of
  materialising two full-resolution planes, and applies the matrix through one bounds-check-free
  loop per pixel layout. 2048x1536 to `Rgb8` went from 22.0 ms to 2.24 ms on an Apple M3 Max, 143
  megapixels per second to 1.41 gigapixels. Output is within one least significant bit of the
  previous floating-point result, a bound `tests/color_fixed.rs` asserts against a float reference.
- `color::convert` refuses a frame whose `bit_depth` is outside 8 to 16 with `Error::Unsupported`,
  rather than scaling by a shift it cannot represent.

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
- Examples `decode`, `probe` and `to_png`; criterion benches with the `container_parse`,
  `grid_compose` and `color_convert` groups.

### Known limitations

- `decode()` reaches the codec seam and returns
  `Error::Unsupported("the HEVC decoder is not linked in this build")`. `probe()` is unaffected and
  works fully, because it never decodes pixels.
- AVIF, image sequences and animation, `iovl` overlay derivation, and encoding are all out of scope
  and reported as `Error::Unsupported`.
- Pixel comparison tests against the macOS reference decoder exist but are `#[ignore]`d until the
  HEVC decoder lands.
- No fuzzing has been run yet. See [SECURITY.md](SECURITY.md).

[Unreleased]: https://github.com/tbraun96/heic-rs/commits/main
