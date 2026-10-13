# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **4:4:4 pictures** failed with "ran out of bytes reading an HEVC bitstream" or "picture is not
  fully covered by slices" as soon as the encoder used 4x4 transform blocks. 4:4:4 signals
  `cbf_cb` and `cbf_cr` on them, 4:2:0 and 4:2:2 do not, and both were skipped, so CABAC lost
  sync (`decode/tu.rs`). A 4:4:4 golden vector now covers it, made with FFmpeg and x265 because
  `sips` cannot write 4:4:4 (`scripts/make-444-vector.sh`).

## [0.1.4] - 2026-10-11

Security fixes: five ways a malformed HEVC stream could panic, all found by fuzzing and reported
privately, with fixes, by **s17sh70x2jfl031ag** -- thank you again. Take it if you decode untrusted
files. Output, API and MSRV are unchanged: every fixture still decodes bit for bit as before.

### Security

Panics in release builds:

- **A block starting right of the picture.** Its width clips to nothing, but its row offsets were
  still computed and on the last rows pointed past the end of the plane (`decode/recon.rs`). Such a
  block now writes nothing, as blocks partly outside the picture already wrote only what is inside.
- **A conformance window as large as the picture.** The crop then started outside the decoded plane
  (`decode/mod.rs`). The SPS now refuses a window whose offsets leave no sample in either direction,
  as FFmpeg's strict mode does, with the offsets scaled and summed in checked arithmetic.
- **A reference row or column outside the plane.** For a block starting below or right of the
  picture, the row above and the corner were read from past the end of the plane (`decode/refs.rs`).
  They, and the column to the left, are now unavailable there, as a column below the plane already
  was.

Panics with overflow checks on (debug builds, tests, fuzzing); release builds returned an error:

- **A PCM bit depth above the sample bit depth** underflowed `bit_depth - pcm_bit_depth`
  (`decode/ctu.rs`). The SPS refuses it, as FFmpeg does, and also refuses a PCM block size outside
  the 8 to 32 samples the standard allows: those sizes are shift amounts.
- **A slice QP outside `-QpBdOffsetY..=51`** shifted the dequantiser by 64 or more
  (`transform/dequant.rs`). The slice header refuses it, as FFmpeg does, in checked arithmetic; and
  the PPS refuses an `init_qp` that no bit depth allows, which overflowed on its own.

No memory-safety impact: the crate is `#![forbid(unsafe_code)]`.

### Added

- The five reported files as regression tests (`tests/hevc_hostile.rs`), each run in debug and in
  release builds.
- CI fuzzing runs with debug assertions, so arithmetic overflow fails the job; every input that has
  found a bug is part of the seed corpus.

## [0.1.3] - 2026-10-11

A security fix, the first finding of the fuzzing 0.1.2 added. Take it if you decode untrusted files:
a 690-byte input could make `probe` or `decode` ask for gigabytes. Output, API and MSRV are unchanged.

### Security

- **Items built from other items could outgrow any bound.** An item stored by construction method
  2 is assembled from the data of other items, and each may list several extents; with items built
  from items, eight deep, every level multiplied the one beneath it. The `probe` fuzz target found a
  690-byte file that asked for 2.2 GB on its first CI run. An assembled item may now be no larger
  than the file that holds it -- it is made of that file's bytes -- and a larger one is
  `Error::Malformed`. No memory-safety impact; a resource-exhaustion bug reachable from untrusted
  input, which `DecodeOptions::max_pixels` did not cover because it caps pixels, not item data.

### Added

- The fuzzer's input as a regression test, with a counting allocator: before the fix the peak grew by
  52.9 GB; now by less than a megabyte.

### Fixed

- CI's publish dry run regenerated the committed lockfile and failed on the dirty tree.

## [0.1.2] - 2026-10-11

A security fix. Take it if you decode untrusted files: a 24-byte input could make `decode` panic.
Output, API and MSRV are unchanged.

### Security

- **A `uuid` box too small for its own extended type panicked instead of being refused.** The check
  that a box's stated size covers its header was made before the 16-byte extended type of a `uuid`
  box was read, so a `uuid` box stating 20 bytes passed it, and its payload was sliced from byte 24
  of a 20-byte box: `slice index starts at 24 but ends at 20` (`src/boxes.rs`), with the default
  options and in release builds. The header is now measured after the extended type, and the
  payload is sliced from that measurement. No memory-safety impact: the crate is
  `#![forbid(unsafe_code)]`, so the bug was a panic, which a caller decoding uploads should never
  have to catch. Found by fuzzing and reported privately under `SECURITY.md` by
  **s17sh70x2jfl031ag**, with the fix -- thank you.

### Added

- Regression tests: the reported input, through `BoxIter` and `decode`; the same shape with a 64-bit
  size; and every 32- and 64-bit header of a `uuid`, `mdat` or `meta` box with any stated size below
  64 over every buffer length below 64 -- an error or a box, never a panic.
- Fuzzing. `fuzz/` holds three `cargo-fuzz` targets -- `decode`, `probe` and the box walk -- and CI
  runs each for two minutes on every push and pull request, seeded with the test fixtures, keeping
  any crashing input as an artifact. With the fix reverted, the box-walk target finds this release's
  panic from an empty corpus. GitHub's private vulnerability reporting is switched on, so the route
  `SECURITY.md` prefers now works.

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
