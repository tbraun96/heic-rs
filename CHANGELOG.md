# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-09-11

Performance. Output is bit-identical to 0.1.0 on every fixture, the API is unchanged, and the MSRV
is unchanged: this release is worth taking and cannot break you.

Measured on an Apple M3 Max against `heic` 0.1.6 (AGPL-3.0), whole file in and RGB8 out, both
decoders alternating, ten cycles, lowest run:

| file | 0.1.0 | 0.1.1 | `heic` (AGPL) |
|---|---|---|---|
| `flat-64.heic` | 0.030 ms | **0.023 ms** | 0.025 ms |
| `gradient-512.heic` | 2.07 ms | **1.42 ms** | 2.30 ms |
| `checker-1024.heic` | 3.35 ms | **2.60 ms** | 4.94 ms |
| `photo-2048.heic` | 2.87 ms | **1.77 ms** | 7.58 ms |

`flat-64.heic` is the one this crate used to lose, and it was the first item on the roadmap. It is
now a win, and not because of threads: serial `photo-2048.heic` went from 23.8 ms to 17.0 ms, so
roughly a third of the margin survives with the `parallel` feature switched off — which is the
configuration WebAssembly runs in.

### Changed

- **Entropy path.** The last significant coefficient is found through a compile-time inverse scan
  instead of walking the scan order backwards, which was up to 1024 steps for a 32x32 block whose
  energy sits near DC — what a photograph looks like. The CABAC engine fetches a byte at a time,
  renormalises with a shift, and its bin decoders are infallible; `sig_coeff_flag` contexts settle
  once per sub-block; bypass bins decode without a data-dependent branch. 109 to 140.5 Mbin/s.
- **Reconstruction.** Dequantisation is bounded by the extent of the coefficients that exist, 4x4
  blocks run as a branch-free matrix product, blocks whose coefficients lie in one row or column
  collapse, intra predictors are const-generic over the block size, reference samples are gathered
  per minimum transform block rather than per sample, and SAO snapshots a plane only when some CTB
  actually applies it.
- **Container and scheduling.** A grid's tiles are read in place: `decode` no longer composes a
  canvas the size of the picture only for the colour pass to read it straight back, which on
  `photo-2048.heic` was 14 MB and 256 us. Peak allocation for that picture fell from 49.2 to
  39.8 MB. The primary item's properties are gathered once instead of twice.
- **Colour is no longer spread below 768x768.** On a single-tile decode the pool's workers have
  slept through the whole codec run, and waking them cost 90-150 us more than the conversion saved.
  Measured alone with a warm pool the old floor looked like a 2.4x win; measured inside a real
  decode it was a loss. Nothing above the floor changed.

### Fixed

- Nothing. No defect was found in 0.1.0's output: every change here was checked bit-identical
  against it across all nine fixtures, seven pixel layouts and three thread counts.

**0.1.0 was the first release.** The crate decodes HEIC files to pixels end to end.

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
  what puts `heic-rs` ahead on anything stored as a grid. Whole-file, RGB8 out, on an Apple M3 Max:
  2.87 ms against 7.78 ms at 2048x1536, 3.35 against 5.04 at 1024x1024, 2.07 against 2.30 at
  512x512, and 0.030 against 0.023 at 64x64 — the last of those a third slower, and the only case
  parallelism cannot reach. See the README's performance section.
- AVIF, image sequences and animation, `iovl` overlay derivation, and encoding are all out of scope
  and reported as `Error::Unsupported`.
- Inter prediction, P and B slices, dependent slice segments and the multilayer, 3D, screen-content
  and range extensions are refused by the decoder with a message naming the tool.
- No fuzzing has been run yet. See [SECURITY.md](SECURITY.md).

[Unreleased]: https://github.com/tbraun96/heic-rs/commits/main
