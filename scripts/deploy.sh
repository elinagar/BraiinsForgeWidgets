#!/usr/bin/env bash
# Sync a widget into a bmc-main checkout and deploy it to a Deck.
#
#   scripts/deploy.sh <widget-dir> <path-to-bmc-main> <deck-ip> [--first]
#
# --first ships every package built from the checkout (needed once per Deck
# and after bmc-main changes); without it only the widget package goes.
# Requires: nix with flakes, git-lfs content pulled in bmc-main, and an SSH
# key on the Deck (ssh-copy-id root@<deck-ip>).
set -euo pipefail
widget="${1:?widget directory, e.g. party-quiz}"
bmc="${2:?path to a bmc-main checkout}"
ip="${3:?Deck IP address}"
first="${4:-}"
here="$(cd "$(dirname "$0")" && pwd)"
"$here/sync-to-bmc-main.sh" "$widget" "$bmc"
cd "$bmc"
# Nix only sees files Git tracks; staging is enough.
git add "widgets-wasm/$widget" widgets-wasm/Cargo.toml widgets-wasm/Cargo.lock
if [ "$first" = "--first" ]; then
  nix run .#deck -- deploy --device "$ip"
else
  nix run .#deck -- deploy --device "$ip" --packages "widget-$widget"
fi
cat <<MSG

Deployed $widget to $ip. Add it to a fullscreen scene in the Deck's web app.
deck deploy clears the Deck's upgrade servers; when you are done iterating run:
  nix run .#deck -- register-server --device $ip
MSG
