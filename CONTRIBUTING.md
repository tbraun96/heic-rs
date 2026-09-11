# Contributing to heic-rs

Thanks for considering it. This is a small crate with a narrow purpose, so the rules below are short
and CI enforces most of them.

## Building and testing

Requires Rust 1.85 or newer (the crate is edition 2024, and 1.85 is the declared MSRV, tested in CI).

```
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo check --target wasm32-unknown-unknown --no-default-features
```

The last one needs the target installed once:

```
rustup target add wasm32-unknown-unknown
```

Run all five before opening a pull request. CI runs exactly these, on Linux, macOS and Windows,
against stable and 1.85.

## House rules

These are not style preferences; a pull request that breaks one will fail CI or be sent back.

- **250 lines per file, maximum.** Every tracked `*.rs` file. The `line-guard` CI job counts lines
  and fails with the offenders listed. When a file outgrows the limit, split it along a real seam
  (one box type, one property, one stage) rather than slicing it at line 250.
- **No `unsafe`.** The crate root carries `#![forbid(unsafe_code)]`. There is no exception process.
- **No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or indexing that can panic in library
  code.** HEIF files are untrusted input; a malformed file must produce an `Error`, not a panic.
  Tests, examples and benches may use `unwrap` freely.
- **No I/O in business logic (SBIO).** The decoding path takes `&[u8]` and returns pixels. It must
  not open files, resolve paths, or touch the network. All filesystem access lives in the `io`
  module behind the `std` feature.
- **No implicit defaults in the parsing path.** If the container does not say, either the spec gives
  the default and you cite it in a comment, or you return `Error::Malformed`.
- **Bounds-check everything.** Reads go through the reader primitives, which return
  `Error::Truncated` rather than slicing past the end.

## Commit style

Conventional commits, lowercase, with a scope naming the module:

```
feat(boxes): parse iref version 1 large item ids
fix(grid): reject grids whose tile count does not match rows * columns
docs(readme): state the decoder status up front
test(props): cover clap with a non-integer aperture offset
perf(color): hoist the limited-range coefficients out of the inner loop
```

Pull request titles follow the same form; CI does not check this, but reviewers do.

## Fixtures

Test fixtures are generated, never committed by hand:

```
bash scripts/make-fixtures.sh
```

That script uses macOS `sips` only, so it runs on macOS and nowhere else. It encodes the synthetic
PNGs produced by `scripts/gen-pngs.py` into HEIC with `sips -s format heic`, and writes the
ground-truth pixels back out with `sips -s format png`, using Apple's own decoder as the reference.
If you are not on a Mac, do not hand-edit fixtures; ask in the pull request and someone with a Mac
will regenerate them.

Note that `sips` tiles anything larger than 512 px on a side into a `grid` derivation of 512x512
tiles, which is why the grid path is exercised by nearly every non-trivial fixture.

## Documentation

Public items need doc comments, and `cargo doc` runs with `-D warnings` in CI, so a broken intra-doc
link fails the build. If your change alters behaviour a user can observe, add a line to the
`[Unreleased]` section of [CHANGELOG.md](CHANGELOG.md) in the same pull request.

## Licensing of contributions

There is no CLA and no DCO sign-off requirement.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this
crate by you, as defined in the Apache-2.0 license, shall be dual licensed as MIT OR Apache-2.0,
without any additional terms or conditions. Do not contribute code you are not entitled to license
this way, and in particular do not port code from AGPL or LGPL HEIC implementations.

## Releasing

Releases are cut by the `release.yml` workflow, which triggers on a `v*` tag:

1. Land the version bump in `Cargo.toml` and move the `[Unreleased]` section of the changelog under
   the new version heading.
2. Tag the merge commit as `vX.Y.Z` and push the tag.
3. The workflow's `verify` job fails unless the tag, stripped of its leading `v`, equals the
   `version` field in `Cargo.toml`. Then it runs `cargo publish --locked` using
   `secrets.CARGO_REGISTRY_TOKEN`, and a second job builds the examples on Linux, macOS and Windows
   and attaches them to the GitHub release.

The `CARGO_REGISTRY_TOKEN` secret is a stopgap. **crates.io Trusted Publishing (OIDC) is the intended
successor**, which lets the workflow authenticate to crates.io without a long-lived token in
repository secrets. We cannot configure it yet: Trusted Publishing is set up against an existing
crate, so the first publish has to happen with the token. We will migrate immediately after that
first release and delete the secret.
