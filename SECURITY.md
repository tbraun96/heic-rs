# Security Policy

## Supported versions

`heic-rs` has not had a release yet. Until 0.1.0 ships, the supported version is the `main` branch.

| Version | Supported |
|---|---|
| `main` (unreleased) | yes |
| 0.1.x | yes, once released |

After the first release, security fixes land on the latest published minor version. Older minor
versions are not backported.

## Reporting a vulnerability

Please report privately, not in a public issue.

- Preferred: open a private security advisory at
  <https://github.com/tbraun96/heic-rs/security/advisories/new>.
- Alternatively, email tbraun96@gmail.com.

Include the input file if you can share it, or a generator for it if you cannot, plus the crate
version or commit, the `DecodeOptions` in use, and what you observed (panic, hang, allocation
blow-up, wrong pixels). A minimised reproducer is worth more than a description.

Expect an acknowledgement within a few days. Fixes are developed in a private fork and disclosed
alongside the patched release, with credit to the reporter unless you prefer otherwise.

## Threat model

**HEIF files are untrusted input.** That is the whole premise: this crate exists to be pointed at
files that arrived over a network, from a phone, or from a user upload, and it is written on the
assumption that any of them may be hostile or simply corrupt.

What that assumption buys:

- **`#![forbid(unsafe_code)]`** at the crate root. There is no `unsafe` block to audit, and the
  compiler enforces that. A parser bug therefore degrades into a panic or wrong output, not a
  memory-safety vulnerability: no out-of-bounds read, no use-after-free, no type confusion.
- **No `unwrap`, `expect`, or `panic!` in library code.** Malformed input returns an `Error`. A panic
  reachable from library code with any byte sequence is a bug worth reporting, even without a
  memory-safety consequence, because callers decoding untrusted uploads should not have to catch
  unwinds.
- **Every parser is bounds-checked.** Reads go through reader primitives that return
  `Error::Truncated` when the input ends early and `Error::Malformed` when a structure is internally
  inconsistent (a box that claims to extend past its parent, an `iloc` extent outside the file, a
  grid whose tile count does not match its declared rows and columns).
- **No I/O in the decoding path (SBIO).** The core takes `&[u8]` and returns pixels. It does not open
  files and never touches the network, so there is no path-traversal or SSRF surface. The only
  filesystem access in the crate is the `io` module behind the `std` feature, which reads the file
  the caller named and does nothing else.
- **Zero required dependencies**, so the supply-chain surface of a production build is this crate and
  the standard library. `cargo-deny` runs in CI against the advisory database.

### Resource exhaustion

The realistic attack against a safe decoder is making it allocate or compute too much.

- **`DecodeOptions::max_pixels`** caps the output pixel count, checked before any pixel buffer is
  allocated. It defaults to `Some(DEFAULT_MAX_PIXELS)`, where
  `pub const DEFAULT_MAX_PIXELS: u64 = 268_435_456;` (256 megapixels). If a file declares more, the
  decode is refused with an error rather than attempted.
- Lower it for hostile input. A service accepting uploads has no reason to allow 256 Mpx; set it to
  whatever your product actually displays. Setting it to `None` disables the check entirely and is
  appropriate only for files you control.
- Box sizes and item extents are validated against the actual input length before anything is
  allocated, so a header claiming a four-gigabyte box does not cause a four-gigabyte allocation.
- Grid geometry is validated against `max_pixels` too, so a small file describing an enormous grid is
  rejected at the derivation step rather than while compositing.

### Out of scope

- Encoding. This crate does not encode.
- Correctness of pixel output as a security property. Wrong pixels are a bug, not a vulnerability.
- Denial of service by a caller who explicitly set `max_pixels` to `None`.

## Fuzzing

**No fuzzing has been run on this crate yet.** Saying otherwise would be a false assurance, so: it
has not happened. It is on the roadmap, and it will happen before 0.1.0 is published.

`cargo-fuzz` requires a nightly toolchain, which is why fuzz targets are not part of the stable CI
matrix: the whole point of the matrix is to prove the crate builds and passes on stable and on the
1.85 MSRV. Fuzzing will run out of band instead.

The intended targets:

- `fuzz_parse` — drives `probe` over arbitrary bytes. Any panic, hang, or unbounded allocation is a
  finding. This is the highest-value target, because `probe` covers the entire container parser and
  is the code path most likely to be pointed at hostile input.
- `fuzz_decode` — drives `decode` over arbitrary bytes with a deliberately small `max_pixels`, so
  that a legitimately large image does not masquerade as a resource-exhaustion finding. This target
  only becomes interesting once the HEVC decoder lands; today it stops at the codec seam.

To run one, once the targets exist:

```
cargo +nightly fuzz run fuzz_parse
```

Because the decoding path takes `&[u8]` and performs no I/O, a fuzz target is a two-line harness with
no filesystem setup and no mocking. That is one of the concrete payoffs of the SBIO rule.
