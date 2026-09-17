# Repository AI Instructions

This repository collects Braiins Deck widgets, one crate per folder, built against the SDK in
[BraiinsForge/bmc-main](https://github.com/BraiinsForge/bmc-main). Read `README.md` for the build and deploy flow and
`IMPLEMENTATION_PLAN.md` for work in progress before changing anything.

## Layout

- `<widget>/` is a Rust `cdylib` + `lib` crate with `manifest.json`, `assets/`, `src/`, optional `examples/`.
- `docs/USER_STORIES.md`, `docs/USER_DOC.md`, `docs/TECHNICAL_DOC.md` have one section per widget. Keep them in step
  with the code; a behavior change lands in the story first.
- `scripts/sync-to-bmc-main.sh` copies a crate into a bmc-main checkout for the testbed and device deployment.

## Rules

- The SDK dependency is a pinned git revision of bmc-main. Bump it deliberately, together with the firmware version it
  matches, and note it in the plan.
- Pure logic (models, parsers, request routing, screen data) lives in modules that compile natively and have unit
  tests. Host calls stay in a `#[cfg(target_arch = "wasm32")]` glue module.
- No `format!`, `println!`, or `dbg!` under `src/`; use the SDK `fmt!` macro. Examples may use std formatting.
- Validate before handing over: `cargo test -p <widget>`, `cargo clippy -p <widget> --target wasm32-unknown-unknown
  -- -D warnings`, `cargo clippy -p <widget> --all-targets -- -D warnings`, `cargo fmt -- --check`.
- Work on a branch; do not commit or push unless asked. Commit subjects in the imperative, body lines starting with
  `-`.
- Widget `manifest.json` follows bmc-main's schema; a new widget needs a fresh UUIDv4 `uid` that is never reused.
