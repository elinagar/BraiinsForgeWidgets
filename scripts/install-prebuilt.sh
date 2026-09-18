#!/usr/bin/env bash
# Install a prebuilt widget package on a Braiins Deck without Nix, Rust, or
# bmc-main on your machine. Needs only ssh/scp and the Deck's root password
# (the one set in the Deck's web app).
#
#   scripts/install-prebuilt.sh dist/party-quiz-0.1.0.nar <deck-ip>
#
# The .nar is a Nix store export of the widget package. The Deck's own Nix
# imports it and bmc-nix-cli adds it to the application profile as a new
# generation; the running widget reloads without a restart. The package was
# built against the Deck firmware named in dist/README.md.
set -euo pipefail
nar="${1:?path to the .nar file}"
ip="${2:?Deck IP address}"
[ -f "$nar" ] || { echo "no such file: $nar" >&2; exit 1; }
base="$(basename "$nar" .nar)"          # e.g. party-quiz-0.1.0
widget="${base%-*}"                     # party-quiz
version="${base##*-}"                   # 0.1.0
echo "Copying $nar to the Deck at $ip ..."
scp -q "$nar" "root@$ip:/tmp/$base.nar"
ssh "root@$ip" "set -e
  N=\$(ls -d /nix/store/*-nix-armv7l*/bin | head -1)
  \$N/nix-store --import < /tmp/$base.nar >/tmp/$base.paths
  P=\$(grep -o '/nix/store/[a-z0-9]*-bmc-widget-$widget' /tmp/$base.paths | tail -1)
  [ -n \"\$P\" ] || P=\$(ls -td /nix/store/*-bmc-widget-$widget | head -1)
  echo \"Installing \$P as widget-$widget $version ...\"
  # bmc-nix-cli shells out to nix-store, which is not on PATH in a plain ssh session.
  PATH=\"\$N:\$PATH\" bmc-nix-cli add-packages --name widget-$widget --version $version --store-path \"\$P\"
  bmc-nix-cli list-packages | grep widget-$widget
  rm -f /tmp/$base.nar /tmp/$base.paths"
cat <<MSG

Installed. In the Deck's web app add the widget to a fullscreen scene (or, if it
was already there, it has reloaded in place). Turn off scene cycling while
playing. Sounds need the patched runtime described in the README; without it
the game is silent but complete.
MSG
