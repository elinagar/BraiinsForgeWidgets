# BraiinsForgeWidgets

Widgets for the [Braiins Deck](https://braiinsforge.com/hardware/braiins-deck), one Rust crate per widget, built to
WebAssembly against the Deck SDK from [BraiinsForge/bmc-main](https://github.com/BraiinsForge/bmc-main).

## Widgets

| Widget | Folder | Status |
| --- | --- | --- |
| Party Quiz, phones as buzzers | [`party-quiz/`](party-quiz/) | Playable in the dev server; awaiting a testbed and on-device check |

Each widget has a user story in [`docs/USER_STORIES.md`](docs/USER_STORIES.md), a player guide in
[`docs/USER_DOC.md`](docs/USER_DOC.md), and its design in [`docs/TECHNICAL_DOC.md`](docs/TECHNICAL_DOC.md). Work in
progress is tracked in [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md).

## Build

The Rust toolchain is pinned in `rust-toolchain.toml` (it includes the `wasm32-unknown-unknown` target). The SDK is a
git dependency pinned to the bmc-main commit matching the current Deck firmware, so a plain checkout builds without
Nix:

```shell
cargo test -p party-quiz                                        # host-side unit tests
cargo clippy -p party-quiz --target wasm32-unknown-unknown -- -D warnings
cargo build -p party-quiz --release --target wasm32-unknown-unknown
```

Run the phone controller page in a browser without a Deck:

```shell
cargo run -p party-quiz --example dev-server -- 8080
# open http://localhost:8080/ in two tabs
```

## Deploy to a Deck

Deployment uses bmc-main's Nix tooling, so the crate is copied into a bmc-main checkout first:

```shell
scripts/deploy.sh party-quiz /path/to/bmc-main "$DEVICE_IP" --first   # first time: every package
scripts/deploy.sh party-quiz /path/to/bmc-main "$DEVICE_IP"           # later: only the widget
```

Or step by step:

```shell
scripts/sync-to-bmc-main.sh party-quiz /path/to/bmc-main
cd /path/to/bmc-main
just wasm::dev party-quiz                                        # desktop testbed, hot reload
nix run .#deck -- deploy --device "$DEVICE_IP" --packages widget-party-quiz
```

The script rewrites the SDK dependency to the in-tree path and registers the crate in bmc-main's widget workspace.
Follow bmc-main's `README.md` for the development shell, `just validate`, and the visual-regression workflow.

## Conventions

- Widgets follow bmc-main's [widget best practices](https://github.com/BraiinsForge/bmc-main/blob/master/docs/devel/wasm-widgets/best-practices.md):
  pure logic in host-testable modules, host imports only behind `#[cfg(target_arch = "wasm32")]`, `fmt!` instead of
  `format!`, numbers through `format_number!`, fixed font sizes per viewport.
- After changing a `manifest.json`, regenerate the typed params with bmc-main's codegen
  (`just wasm::gen <widget>` inside bmc-main after syncing, then copy `src/manifest_params.rs` back).
- Sound assets are small PCM WAV files committed directly; bmc-main keeps them in Git LFS, so run `git lfs install`
  before syncing a widget with sounds into bmc-main.
- GPL-3.0-or-later, matching the SDK. Every Rust source carries the bmc-main license header.
