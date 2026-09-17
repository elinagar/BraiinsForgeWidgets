# Implementation Plan: Party Quiz Widget

Tracks delivery of the Party Quiz widget described in [`docs/USER_STORIES.md`](docs/USER_STORIES.md). Manifest:
[`party-quiz/manifest.json`](party-quiz/manifest.json). The crate lives in this repository and is copied into a
bmc-main checkout with `scripts/sync-to-bmc-main.sh` for the testbed and deployment. Screen mockups live in the design
canvas "Deck Party Quiz" (Claude artifact, private). The phone page can be driven without a Deck through
`cargo run -p party-quiz --example dev-server -- 8080`.

Status legend: `[ ]` not started · `[~]` in progress · `[x]` done · `[!]` blocked

## Branching

Work on `main` of this repository until a second widget arrives; then one branch per widget. Deployment always goes
through a bmc-main checkout (`scripts/sync-to-bmc-main.sh`), where bmc-main's own commit and ticket rules apply if a
widget is ever proposed upstream.

## Decisions taken

- **Phones are the controllers.** Hosted WASM widgets collapse multi-touch into one stream
  (`docs/stories/touch-and-gestures.md`), so the Deck is the board and each phone is a buzzer.
- **The widget is the server.** One SDK HTTP listener (`http_listen`, ephemeral port) serves the controller page and a
  small JSON API. Phones poll every 400 ms during a question and 1 s otherwise; no WebSocket server exists in the SDK
  and the host listener is serial, so polling stays modest.
- **Full-screen 1280x480 only**, like Fleet Management. The board needs the width; the LED strip exists only on BMC100.
- **Questions come as packs from Nexus, cached on device.** A bundled 500-question starter pack makes the widget usable
  before the Nexus endpoint ships. The corpus target is 200k to 500k reviewed questions, not "unlimited"; freshness
  comes from weekly generated packs, not from raw count.
- **Scoring**: 1 000 points decaying linearly to 500 over the answer time, +200 per correct answer after a streak of
  three. Simple enough to explain on the reveal screen.

## Checkpoints

### 0. Documentation

- [x] Widget story in `docs/USER_STORIES.md` (bmc-main format, so it can be proposed upstream as
      `docs/stories/widgets/party-quiz.md`)
- [x] Manifest `party-quiz/manifest.json` (UID `f4ebfd1d-…`, category `utility`, six params)
- [x] This plan
- [~] bmc-main's `just validate` (manifest load test, content checks) after syncing. Not yet run: this machine has
      no `nix`. The manifest was checked against `manifest.schema.json` with a JSON Schema validator instead.

Validation: `nix develop -c cargo test -p bmc-widget-manifest-tests`.

### 1. Crate skeleton and lobby

- [x] `party-quiz/` crate (`cdylib` + `lib`, SDK as a pinned git dependency, `[package.metadata.nix] include =
      ["assets/**"]`) in this workspace; the sync script registers it in bmc-main's
- [x] `just wasm::gen party-quiz` to generate `src/manifest_params.rs` (run as the underlying `cargo run -p
      bmc-widget-codegen` here)
- [x] `assets/icon.svg` (flat shapes, no filters)
- [x] Pure game model in `src/model.rs`: `Player`, `Question`, `Phase` (Lobby, Question, Reveal, Standings, Podium),
      `Game` with join/answer/advance methods, scoring, host succession. Host-testable, no SDK imports.
- [x] Lobby screen: title bar, QR via `Draw::qr` of `http://<ip>:<port>/` from `network::info()`, plain address,
      avatar row, player count card, host hint card. `Loading…` until IP and pack are known.
- [x] Unit tests for scoring, streaks, host succession, name deduplication, capacity limit (13 tests)

Validation: `cargo test -p party-quiz` and `cargo clippy -p party-quiz --target wasm32-unknown-unknown -- -D warnings`
(both green, from `widgets-wasm/`); `just wasm::dev party-quiz` for the visual check needs the Nix shell. Release wasm
is about 93 KB against the 1 MB gate.

Decision recorded during the build: six colours serve twelve seats. A colour must be unique only while a free colour
remains; from the seventh player on, colours repeat. The story's "greyed out" rule applies to the first six.

### 2. Controller endpoint and phone page

- [x] `src/server.rs`: pure router over `Request {method, path, body}` → `Response {status, headers, body}` plus an
      `Effect` telling the glue to repaint. Routes: `GET /` (page), `POST /join` (`name`, `color`), `GET /state?v=&t=`
      (204 when the phone already has this version), `POST /answer` (`t`, `c`), `POST /host` (`t`, `a` in
      start/skip/end/new), `POST /leave`, everything else 404. Bodies over 1 KiB → 400 `too_large`. Form bodies are
      parsed by hand (percent-decoding, no JSON parser needed on the way in); JSON out is built without `core::fmt`.
- [x] Snapshot JSON versioned by `Game::version`; carries players, question meta (never question or answer text),
      reveal counts and explanation, and the caller's own seat. Polling with a kept token re-seats a returning phone.
- [x] `assets/controller.html`: one file, 14 KB, no external resources. Join, lobby with host controls, answer grid
      (four colour-and-shape buttons), result, podium. Token in `localStorage`; unknown token → back to join.
- [x] Glue in `lib.rs`: `http_listen(0, …)` opened lazily on the first frame, `mdns_register("_deck-quiz._tcp", …)`,
      request callback routes through `server::handle` and calls `request_frame()` on a change, `unload` closes both.
      Seat tokens are two host randoms.
- [x] `examples/dev-server.rs`: a std-only TCP server around the same routes so the phone page can be driven in a
      browser without a Deck (`cargo run -p party-quiz --example dev-server -- 8080`).
- [x] Tests: 8 route tests (page, 404, oversize body, join incl. percent-decoding, 204 on same version, snapshot
      content, token checks on answer/host, re-seat on poll) plus wire helper round-trips. 21 tests in the crate.
- [x] Browser check: headless Chrome against the dev server rendered join, answer, result, and podium views for a
      real two-player game driven over HTTP.

Findings while wiring the host listener (`bmc-wasm-runtime/src/runtime/background/http.rs`):

- Response headers are inserted verbatim before the host's blank line, so header lines must be `\r\n`-separated
  with no trailing newline. `server.rs` does this.
- The host handles one connection at a time and waits up to 10 s for the widget's reply, and it allocates the request
  body from `Content-Length` before the widget can refuse it. The widget's 1 KiB cap therefore protects parsing, not
  memory; a host-side cap would be a runtime change worth proposing.
- Because of the serial listener, the phone polls at 400 ms during a question and 1 s otherwise (the snapshot tells
  the page which), not the 300 ms first planned. Twelve phones give roughly 30 requests per second at peak; measure
  on a Deck with `just wasm::profile`.

Validation: `cargo test -p party-quiz`, clippy on wasm32 and native with `-D warnings`, `cargo fmt --check`, all green.

### 3. Question, reveal, standings screens

- [x] Question screen: category and difficulty chips, auto-fit question text, four colour-and-shape cards, countdown
      ring with seconds, one dot per connected player (filled once answered)
- [x] Reveal screen: correct card ringed, others dimmed, vote bars with names, fastest-correct card, explanation
- [x] Standings and podium: one screen, podium blocks for the top three, ranked list for the rest, final headline
- [x] Phase timing from `Game::tick`; early countdown end after the grace period; the host's Skip and End work
- [x] Scores through `format_number!`; fixed font sizes; ellipsis on long labels
- [x] Starter pack `assets/questions-starter.tsv`: 160 hand-reviewed questions, 16 per pack across the ten packs, in a
      tab-separated line format parsed in pure Rust (`src/pack.rs`, with shuffle and filtering). A test asserts the
      pack parses whole, IDs are unique, answers are distinct, and every pack has at least ten questions.
- [ ] Visual check of the four Deck screens in the testbed (`just wasm::dev party-quiz`) and baselines via
      `just wasm::record` / `just wasm::update-baselines`. Needs the Nix shell and a GPU; not possible on this machine.

### 4. LEDs, sound, persistence

- [x] `src/effects.rs` (wasm-only): join flash in the newest player's colour, amber chase for the last 5 s, green or
      red breathe on the reveal by majority, winner-colour chase on the podium; `led::stop()` on unload
- [x] Sounds: `assets/sounds/{tick,correct,wrong,fanfare}.wav`, synthesized 22 kHz mono PCM, gated by the *Sounds*
      param. Ticks on the last three seconds, chime or low tone on the reveal, fanfare on the podium.
- [x] KV: `quiz_best` (top ten `score|name|date` lines) and `quiz_best_line` written on the podium; `quiz_played`
      (comma-separated IDs, capped at 2 000) appended at each reveal and honoured when the deck is rebuilt, wrapping
      around when the pool for the chosen params runs dry
- [x] On-Deck Reset control in the lobby top bar (`button!`, `on_touch` export), clears every seat
- [x] Sounds are committed here as plain files (80 KB total). bmc-main tracks `*.wav` in Git LFS, so `git lfs
      install` is needed before syncing there.

Validation: 30 tests, clippy `-D warnings` on wasm32 and native (`--all-targets`), `cargo fmt --check`, all green.
Release wasm is 276 KB with the page, the pack, and the four sounds embedded, against the 1 MB gate.

### 5. Nexus question packs (separate service work)

- [ ] Endpoint `GET /api/v1/data/party-quiz/packs` → list of `{id, label, difficulty_levels, version, count}`
- [ ] Endpoint `GET /api/v1/data/party-quiz/pack?id=&difficulty=&seed=&n=200` → questions, schema below, with pack
      version and ETag
- [ ] Widget: fetch on scene enter when cache is stale (>1 day) or below 50 unplayed; store in blob cache; fall back to
      cache, then to the starter pack; show `Offline` state when nothing is available
- [ ] Widget: drop questions whose pack version is withdrawn on refresh

Question schema (JSON, one object per question):

```json
{
  "id": "sci-000123",
  "pack": "science",
  "version": 3,
  "difficulty": "medium",
  "q": "Which planet has the shortest day, spinning once in under ten hours?",
  "a": ["Mars", "Jupiter", "Venus", "Saturn"],
  "correct": 1,
  "why": "Jupiter spins once every 9 h 56 min.",
  "lang": "en"
}
```

### 6. Question corpus pipeline (separate repo or `tools/`)

- [ ] Seed: import openly licensed sets (Open Trivia DB, The Trivia API) into the schema; keep licence and source
      fields; 5k to 10k questions
- [ ] Generate: Claude Message Batches API (default model `claude-opus-5`, structured outputs so every item is
      schema-valid), prompts per pack × difficulty × region, batches of 50 with the previous 200 stems as a
      do-not-repeat list; target 200k to 500k
- [ ] Gate: exact and normalized-text dedupe, embedding near-duplicate dedupe, blind re-answer pass that flags
      disagreement, 2 % human spot check per pack, drop ambiguous or dated items
- [ ] Publish: versioned packs to Nexus; weekly current-events pack job
- [ ] Localization: Czech and German packs after English is stable

Cost note: at roughly 120 output tokens per question, 500k questions is about 60M output tokens, low thousands of
dollars at batch pricing. 100M questions would be about 12B tokens and 15 GB, which neither the budget nor the 24 MiB
device asset cap supports and which no household would ever exhaust.

### 7. Release

- [ ] `just validate`, `just wasm::verify party-quiz`, `just wasm::size party-quiz`
- [ ] Deploy to a Deck: `nix run .#deck -- deploy --device "$DEVICE_IP" --packages widget-party-quiz`; play a full
      game with four phones on the Deck's Wi-Fi
- [ ] Root `README.md` official widget list, story index, `docs/USER_DOC`-equivalent (frontend help text via
      `config_help`)
- [ ] Remove the worktree after merge

## Open questions

- Should the Nexus own the pack list so new categories appear without a widget release, or should the param enum stay
  fixed in the manifest? Plan assumes the manifest enum for v1 and a Nexus-driven list later.
- Do we want a "household questions" pack the host types on their phone? Out of v1; the API shape allows it.
- The ambient LED gating by manifest is a known follow-up in `docs/stories/widget-led-effects.md`; this widget uses
  scene-local effects only and is unaffected.

## Risks

- **Fuel budget**: serving `GET /state` for 12 phones at 300 ms is about 40 requests per second. The snapshot copy
  keeps each request under a few thousand fuel units; measure with `just wasm::profile`.
- **Listener limits**: the runtime allows 2 listeners per widget; one is used. The host delivers requests via the
  `__on_http_request` export, so request handling must not block or allocate large buffers.
- **QR capacity**: the URL is short; `Draw::qr` drops over-long text silently, so keep the encoded text to the bare
  `http://ip:port/` form.
- **Phones on a different network**: guests on a guest Wi-Fi with client isolation cannot reach the Deck. The lobby
  should say "connect to the same Wi-Fi as the Deck".
