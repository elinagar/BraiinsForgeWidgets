# Prebuilt packages

| File | Widget | Built against | Notes |
| --- | --- | --- | --- |
| `party-quiz-0.1.0.nar` | Party Quiz 0.1.0 | Deck firmware 26.09, bmc-main `9019fd35`, SDK 0.7.0 | Silent on a stock Deck; sounds need the runtime patch in `patches/` |

A `.nar` is a Nix store export of the built widget package: its manifest, the wasm module, and assets including the
sounds, about 600 KB. Install with `scripts/install-prebuilt.sh <file> <deck-ip>`; it uses the Deck's own Nix and
`bmc-nix-cli`, so nothing is installed on your computer beyond ssh.

Re-export after every deploy, from the exact package the deploy shipped (several builds can sit in the local store, so
never pick one by hand). The deploy log names it in its `copying path '…-bmc-widget-party-quiz'` line:

```shell
scripts/deploy.sh party-quiz /path/to/bmc-main <deck-ip> | tee deploy.log
P=$(grep -o "copying path '/nix/store/[a-z0-9]*-bmc-widget-party-quiz'" deploy.log | tail -1 | grep -o "/nix/store/[^']*")
nix-store --export "$P" > dist/party-quiz-<version>.nar
```
