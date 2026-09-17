#!/usr/bin/env bash
# Copy one widget crate into a bmc-main checkout so it can be built and
# deployed with bmc-main's Nix tooling.
#
#   scripts/sync-to-bmc-main.sh <widget-dir> <path-to-bmc-main>
#   nix run .#deck -- deploy --device "$DEVICE_IP" --packages widget-<widget-dir>
set -euo pipefail
widget="${1:?widget directory, e.g. party-quiz}"
bmc="${2:?path to a bmc-main checkout}"
src="$(cd "$(dirname "$0")/.." && pwd)/$widget"
dst="$bmc/widgets-wasm/$widget"
[ -f "$src/Cargo.toml" ] || { echo "no crate at $src" >&2; exit 1; }
[ -f "$bmc/widgets-wasm/Cargo.toml" ] || { echo "$bmc is not a bmc-main checkout" >&2; exit 1; }
rsync -a --delete --exclude target --exclude Cargo.lock "$src/" "$dst/"
# In-tree the SDK is a path dependency.
sed -i -E 's#^bmc-wasm-sdk = \{ git = .*#bmc-wasm-sdk = { path = "../../bmc-wasm-runtime/sdk" }#' "$dst/Cargo.toml"
sed -i 's#"\$schema": "https://raw.githubusercontent.com/BraiinsForge/bmc-main/master/bmc-widget-manifest/manifest.schema.json"#"$schema": "../../bmc-widget-manifest/manifest.schema.json"#' "$dst/manifest.json"
# Register the crate in the widget workspace if it is not there yet.
if ! grep -q "\"$widget\"" "$bmc/widgets-wasm/Cargo.toml"; then
  sed -i "0,/^members = \[/s//members = [\n    \"$widget\",/" "$bmc/widgets-wasm/Cargo.toml"
  echo "added \"$widget\" to $bmc/widgets-wasm/Cargo.toml members"
fi
echo "synced $widget into $dst"
