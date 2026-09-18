# Prebuilt packages

| File | Widget | Built against | Notes |
| --- | --- | --- | --- |
| `party-quiz-0.1.0.nar` | Party Quiz 0.1.0 | Deck firmware 26.09, bmc-main `9019fd35`, SDK 0.7.0 | Silent on a stock Deck; sounds need the runtime patch in `patches/` |

A `.nar` is a Nix store export of the built widget package: its manifest, the wasm module, and assets, about 300 KB.
Install with `scripts/install-prebuilt.sh <file> <deck-ip>`; it uses the Deck's own Nix and `bmc-nix-cli`, so nothing
is installed on your computer beyond ssh. Rebuild and re-export after any widget change:

```shell
nix build .#deck-packages.widget-party-quiz.pkg -o result-pq      # in a bmc-main checkout after syncing
nix-store --export $(nix-store -qR ./result-pq) > dist/party-quiz-<version>.nar
```
