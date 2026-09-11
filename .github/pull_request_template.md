## What this changes

<!-- One or two sentences. Link the issue if there is one. -->

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all-features` and `cargo test --no-default-features` pass
- [ ] `cargo doc --no-deps --all-features` is warning-free, and new public items have doc comments
- [ ] Every tracked `.rs` file is still 250 lines or fewer
- [ ] No `unwrap`, `expect`, or `panic!` in library code (tests and examples are exempt)
- [ ] No `unsafe`, and no I/O outside the `io` module
- [ ] `CHANGELOG.md` updated under `[Unreleased]` if this changes observable behaviour
- [ ] Title follows conventional commit style, e.g. `feat(boxes): parse iref version 1`

## Notes for the reviewer

<!-- Anything non-obvious: a spec citation, a tradeoff, a fixture that had to be regenerated. -->
