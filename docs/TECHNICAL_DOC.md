# Technical Documentation

## Repository

A Cargo workspace with one crate per widget. Crates compile to `wasm32-unknown-unknown` for the Deck and natively for
their unit tests and examples. The SDK, `bmc-wasm-sdk`, is a git dependency on
[BraiinsForge/bmc-main](https://github.com/BraiinsForge/bmc-main) pinned to the revision matching the current Deck
firmware (26.09 at the time of writing, revision `9019fd35`). Deployment goes through a bmc-main checkout because the
Deck's package build and the `deck deploy` harness are Nix flake outputs of that repository;
`scripts/sync-to-bmc-main.sh` copies a crate in and rewrites the SDK dependency to the in-tree path.

## Party Quiz

### Modules

| Module | Target | Role |
| --- | --- | --- |
| `model.rs` | native + wasm | Game rules: players, phases, scoring, host succession. No SDK imports. |
| `server.rs` | native + wasm | Phone endpoint: routes, form parsing, versioned JSON snapshot. Pure. |
| `pack.rs` | native + wasm | Starter pack parser (tab-separated), filtering, deterministic shuffle. |
| `screens/` | native + wasm | Deck screens as functions from plain data to SDK trees: lobby, question, reveal, standings. |
| `theme.rs` | native + wasm | Colours and type sizes. |
| `effects.rs` | wasm only | LED effects and sounds on phase transitions. |
| `lib.rs` | glue is wasm only | Lifecycle exports, HTTP listener, mDNS, params, KV, screen dispatch. |
| `examples/dev-server.rs` | native | std-only TCP server around the same routes for browser testing. |

### Runtime shape

- **Frames.** `render(delta_ms)` ticks the game, runs the effects transition check, builds the screen for the phase,
  and schedules the next frame: 1 s in the lobby, 100 ms during a countdown, 250 ms elsewhere.
- **Listener.** Opened lazily on the first frame with `http_listen(0, …)`, registered over mDNS as
  `_deck-quiz._tcp`, closed in `unload`. The host runs the listener thread and delivers requests through the SDK's
  `__on_http_request` export; the callback routes through `server::handle`, responds, and requests a frame when the
  game changed. Response header lines are `\r\n`-separated with no trailing newline because the host inserts them
  verbatim before its own blank line.
- **Polling.** Phones `GET /state?v=<version>&t=<token>`; the widget answers 204 when nothing changed. The snapshot
  tells the page how often to poll: 400 ms during a question or reveal, 1 s otherwise. A token on the poll re-seats a
  returning phone.
- **Routes.** `GET /` page; `POST /join` (`name`, `color`); `POST /answer` (`t`, `c`); `POST /host` (`t`, `a` in
  `start|skip|end|new`); `POST /leave` (`t`); anything else 404; bodies over 1 KiB 400.
- **Tokens.** 64-bit, two host randoms, sent as 16 hex digits, kept in the phone's `localStorage`.
- **Questions.** `assets/questions-starter.tsv` is parsed at init, filtered by the pack and difficulty params and by
  the played-ID list in KV, shuffled with a host-seeded xorshift, and loaded into the game. Param changes mid-game mark
  the deck stale; it is rebuilt when the lobby returns. When fewer questions remain than a game needs, the played list
  is cleared and the pool starts over.
- **Persistence (KV).** `quiz_played` (comma-separated IDs, capped at 2 000), `quiz_best` (top ten `score|name|date`
  lines), `quiz_best_line` (the sentence the lobby and podium show).
- **Effects.** Join: solid flash in the newcomer's colour and a chime. First question: rising sting; later
  questions: two-note cue. Answer locked: quiet tap. Last 5 s: amber chase with quiet ticks. Reveal: green or red
  breathe by majority, with a bright arpeggio or a soft descending line. Standings: short riser. Podium: chase in
  the winner's colour with a brass fanfare. Sounds are synthesized 22 kHz mono PCM under `assets/sounds/`, about
  360 KB in total. All effects are
  scene-local; `led::stop()` on unload. Sounds are gated by the *Sounds* param and follow device volume.
- **Touch.** Only the lobby's Reset button. `on_touch` requests a frame; the click is read from `render_ui`'s result.

### Scoring

`points = 1000 - 500 * min(elapsed, answer_time) / answer_time`, plus 200 once the streak exceeds three. Wrong or
missing answers score zero and reset the streak. Ties rank by seat order.

### Limits that shaped the design

- The runtime allows 2 listeners, 16 fetches, 4 WebSockets per widget; one listener is used.
- The host listener handles one connection at a time and waits up to 10 s for the widget; it allocates the request
  body from `Content-Length` before the widget can refuse it. The widget's 1 KiB cap guards parsing, not memory.
- Hosted widgets receive a single touch stream, hence phones as controllers.
- The LED API drives the whole strip with one effect, not individual LEDs.
- `Draw::qr` drops text over QR capacity silently; the encoded URL is kept to `http://ip:port/`.
- 64 KiB guest stack and a per-frame fuel budget: all collections are heap `Vec`s, and JSON is built by hand without
  `core::fmt`.

### Sound on the device

`bmc_wasm_sdk::audio_play` hands the clip to the wasm host, which plays it with rodio only when bmc-wasm-runtime is
built with its `audio` feature. Stock bmc-main enables that feature for the desktop testbed alone, so on a Deck the
call is a no-op. The reason is the sound card: the Deck's ALSA `default` is a software-volume layer directly over the
hardware with no mixing plugin, so a second opener gets `Resource busy`, and the stock runtime would hold the card from
startup and block the system's alarm and alert player. `patches/bmc-main-wasm-audio-on-device.patch` changes the
runtime to open the output on demand, hold it until the clip's decoded length plus 250 ms, and release it on the next
clock tick or when the widget goes dormant, and adds `audio` to the device `hostFeatures`. System sounds then only
lose the card during the second a widget clip is playing. The patch is a candidate for upstream.

### Validation

```shell
cargo test -p party-quiz
cargo clippy -p party-quiz --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p party-quiz --all-targets -- -D warnings
cargo fmt -- --check
```

In bmc-main after syncing: `just wasm::dev party-quiz` (testbed), `just wasm::verify party-quiz` (visual baselines),
`just wasm::size party-quiz`, `just validate`.
