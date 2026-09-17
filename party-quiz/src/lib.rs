// Copyright (C) 2026  Braiins Forge s.r.o.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// Braiins Systems s.r.o. and Braiins Forge s.r.o. each reserve the right
// to grant any party a license to this program, or any part thereof,
// under any terms, and such a grant shall be considered distinct from
// the grant above.

//! Party Quiz: the Deck is the board, phones are the buzzers.
//!
//! [`model`] holds the game rules, [`server`] the phone endpoint, [`pack`]
//! the questions, [`screens`] draws the board, and `effects` drives the LEDs
//! and sounds. The wasm glue below wires
//! them to the host: frames, params, network info, the HTTP listener, mDNS.

#[cfg(target_arch = "wasm32")]
mod effects;
mod manifest_params;
pub mod model;
pub mod pack;
pub mod screens;
pub mod server;
pub mod theme;

#[cfg(target_arch = "wasm32")]
mod wasm_glue {
    use std::cell::{Cell, RefCell};

    use bmc_wasm_sdk::http_listener::{HttpListener, HttpRequest, http_listen};
    use bmc_wasm_sdk::mdns::{MdnsRegistration, mdns_register};
    #[expect(
        clippy::wildcard_imports,
        reason = "widget glue uses many SDK exports and macros in one file"
    )]
    use bmc_wasm_sdk::*;

    use crate::effects::{self, Kind};
    use crate::manifest_params::{CategoryPack, Difficulty, Params};
    use crate::model::{
        ANSWER_COUNT, Game, MAX_PLAYERS, PODIUM_MS, Phase, Player, Question, QuestionDifficulty,
        REVEAL_MS, STANDINGS_MS, Settings,
    };
    use crate::pack::{self, Rng};
    use crate::screens::lobby::{LobbyData, LobbyPlayer, LobbyStatus, RESET_KEY, lobby_view};
    use crate::screens::question::{PlayerDot, QuestionData, question_view};
    use crate::screens::reveal::{RevealData, reveal_view};
    use crate::screens::standings::{StandingRow, StandingsData, standings_view};
    use crate::server::{self, Effect, Request};
    use crate::theme;

    /// The phone page, served from the widget's own listener.
    const CONTROLLER_PAGE: &[u8] = include_bytes!("../assets/controller.html");
    /// mDNS service a companion could browse for.
    const MDNS_SERVICE: &str = "_deck-quiz._tcp";

    /// Lobby frames: nothing moves faster than a phone joining.
    const LOBBY_FRAME_MS: u32 = 1_000;
    /// Countdown frames: the ring and the answered dots.
    const QUESTION_FRAME_MS: u32 = 100;
    /// Reveal, standings, podium: only a seconds counter moves.
    const SLOW_FRAME_MS: u32 = 250;

    /// Top ten finishes, one `score|name|date` per line.
    const KV_BEST: &str = "quiz_best";
    /// The rendered "Best ever" sentence for the lobby and the podium.
    const KV_BEST_LINE: &str = "quiz_best_line";
    /// Comma-separated IDs of questions already asked on this Deck.
    const KV_PLAYED: &str = "quiz_played";
    const BEST_KEEP: usize = 10;
    const PLAYED_KEEP: usize = 2_000;

    thread_local! {
        static GAME: RefCell<Game> = RefCell::new(Game::default());
        static LISTENER: Cell<Option<HttpListener>> = const { Cell::new(None) };
        static PORT: Cell<u16> = const { Cell::new(0) };
        static MDNS: RefCell<Option<MdnsRegistration>> = const { RefCell::new(None) };
        /// Params changed mid-game; rebuild the deck when the lobby returns.
        static DECK_STALE: Cell<bool> = const { Cell::new(false) };
    }

    fn settings_from(params: &Params) -> Settings {
        Settings {
            question_count: usize::try_from(params.question_count).unwrap_or(10),
            answer_time_ms: u32::try_from(params.answer_time)
                .unwrap_or(20)
                .saturating_mul(1_000),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "widget dimensions are small; u32 to f32 is exact below 2^24"
    )]
    fn px(v: u32) -> f32 {
        v as f32
    }

    fn points_text(points: u32) -> String {
        format_number!(points, 0)
    }

    // ── Questions ────────────────────────────────────────────────────

    /// Build the deck for the current params from the bundled starter pack.
    fn reload_deck(game: &mut Game, params: &Params) {
        let pack_filter = match params.category_pack {
            CategoryPack::Mixed => None,
            other => Some(other.as_manifest_value()),
        };
        let difficulty = match params.difficulty {
            Difficulty::Mixed => None,
            Difficulty::Easy => Some(QuestionDifficulty::Easy),
            Difficulty::Medium => Some(QuestionDifficulty::Medium),
            Difficulty::Hard => Some(QuestionDifficulty::Hard),
        };
        let all = pack::parse_tsv(pack::STARTER_PACK);
        let played = played_ids();
        let mut questions: Vec<Question> =
            pack::select(all.clone(), pack_filter, difficulty, &played);
        if questions.len() < game.settings().question_count {
            // The pool is exhausted for these params: start over.
            kv::delete(KV_PLAYED);
            questions = pack::select(all, pack_filter, difficulty, &[]);
        }
        let seed = (u64::from(random_u32()) << 32) | u64::from(random_u32());
        let mut rng = Rng::new(seed);
        pack::shuffle(&mut questions, &mut rng);
        for question in &mut questions {
            pack::shuffle_answers(question, &mut rng);
        }
        game.load_questions(questions);
        DECK_STALE.set(false);
    }

    // ── Persistence ──────────────────────────────────────────────────

    fn played_ids() -> Vec<String> {
        kv::get_string(KV_PLAYED)
            .map(|s| {
                s.split(',')
                    .filter(|id| !id.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn mark_played(id: &str) {
        let mut ids = played_ids();
        if ids.iter().any(|known| known == id) {
            return;
        }
        ids.push(id.to_owned());
        if ids.len() > PLAYED_KEEP {
            let drop = ids.len() - PLAYED_KEEP;
            ids.drain(..drop);
        }
        kv::set(KV_PLAYED, ids.join(",").as_bytes());
    }

    fn today_text() -> String {
        format_date(
            SystemTime::now(),
            FormatDateOpts {
                format: None,
                timezone: system::current().timezone().map(Tz::from_runtime),
            },
        )
    }

    /// Record the winner in the best-ever list and refresh the lobby line.
    fn record_best(game: &Game) {
        let ranked = game.standings();
        let Some(winner) = ranked.first() else {
            return;
        };
        if winner.score == 0 {
            return;
        }
        let mut entries: Vec<(u32, String, String)> = kv::get_string(KV_BEST)
            .map(|s| {
                s.lines()
                    .filter_map(|line| {
                        let mut parts = line.splitn(3, '|');
                        let score = parts.next()?.parse().ok()?;
                        let name = parts.next()?.to_owned();
                        let date = parts.next().unwrap_or("").to_owned();
                        Some((score, name, date))
                    })
                    .collect()
            })
            .unwrap_or_default();
        entries.push((winner.score, winner.name.clone(), today_text()));
        entries.sort_by_key(|entry| core::cmp::Reverse(entry.0));
        entries.truncate(BEST_KEEP);
        let mut blob = String::new();
        for (score, name, date) in &entries {
            let name = name.replace(['|', '\n'], " ");
            let date = date.replace(['|', '\n'], " ");
            let line = fmt!("{score}|{name}|{date}\n");
            blob.push_str(&line);
        }
        kv::set(KV_BEST, blob.as_bytes());
        if let Some((score, name, date)) = entries.first() {
            let score_text = points_text(*score);
            kv::set(
                KV_BEST_LINE,
                fmt!("Best ever on this Deck: {name}, {score_text} \u{b7} {date}").as_bytes(),
            );
        }
    }

    /// Frame bookkeeping that follows a phase change.
    fn on_entered(kind: Kind, game: &Game) {
        match kind {
            Kind::Reveal => {
                if let Some(q) = game.current_question() {
                    mark_played(&q.id);
                }
            }
            Kind::Podium => record_best(game),
            // Back in the lobby after a game: rebuild the deck so the next game
            // draws fresh questions in a fresh order, skipping the ones played.
            Kind::Lobby => DECK_STALE.set(true),
            Kind::Question | Kind::Standings => {}
        }
    }

    // ── Phone endpoint ───────────────────────────────────────────────

    /// A seat token: two host randoms, so a phone cannot guess another seat.
    fn mint_token() -> u64 {
        (u64::from(random_u32()) << 32) | u64::from(random_u32())
    }

    fn on_request(_listener: HttpListener, req: &HttpRequest) {
        let request = Request {
            method: &req.method,
            path: &req.path,
            body: &req.body,
        };
        let mut mint = mint_token;
        let (response, effect) =
            GAME.with_borrow_mut(|game| server::handle(game, &request, CONTROLLER_PAGE, &mut mint));
        req.respond(response.status, response.headers, &response.body);
        if effect == Effect::GameChanged {
            request_frame();
        }
    }

    /// Open the listener once; the lobby shows its port in the QR code.
    fn ensure_listener() {
        if LISTENER.get().is_some() {
            return;
        }
        let listener = http_listen(0, on_request);
        let port = listener.port();
        if port == 0 {
            log_warn!("party-quiz: HTTP listener failed to bind");
            listener.close();
            return;
        }
        LISTENER.set(Some(listener));
        PORT.set(port);
        let registration = mdns_register(MDNS_SERVICE, "Deck Party Quiz", port, &[("v", "1")]);
        MDNS.with_borrow_mut(|slot| *slot = registration);
    }

    fn close_listener() {
        if let Some(listener) = LISTENER.take() {
            listener.close();
        }
        PORT.set(0);
        if let Some(registration) = MDNS.with_borrow_mut(Option::take) {
            registration.unregister();
        }
    }

    // ── Screen data ──────────────────────────────────────────────────

    fn status_line(params: &Params) -> String {
        let count = params.question_count;
        let pack_label = params.category_pack.as_manifest_label();
        fmt!("{count} questions \u{b7} {pack_label}")
    }

    fn lobby_status(net: &NetworkInfo) -> LobbyStatus {
        let port = PORT.get();
        if port == 0 || net.ip.is_empty() {
            return LobbyStatus::Loading;
        }
        let ip = &net.ip;
        LobbyStatus::Ready {
            url: fmt!("http://{ip}:{port}/"),
        }
    }

    fn lobby_data(game: &Game, params: &Params) -> LobbyData {
        let host_seat = game.host().map(|h| h.seat);
        let players = game
            .connected()
            .map(|p| LobbyPlayer {
                name: p.name.clone(),
                color: theme::player_color(p.color),
                is_host: host_seat == Some(p.seat),
            })
            .collect();
        let net = network::info();
        LobbyData {
            status: lobby_status(&net),
            ssid: net.ssid,
            players,
            max_players: MAX_PLAYERS,
            show_names: params.show_player_names,
            status_line: status_line(params),
            best_line: kv::get_string(KV_BEST_LINE),
            show_reset: !game.players().is_empty(),
        }
    }

    fn question_data(game: &Game, q: &Question, index: usize) -> QuestionData {
        QuestionData {
            category: q.category.clone(),
            difficulty: q.difficulty.label(),
            index,
            total: game.question_total(),
            text: q.text.clone(),
            answers: q.answers.clone(),
            remaining_ms: game.remaining_ms(),
            total_ms: game.settings().answer_time_ms,
            dots: game
                .connected()
                .map(|p| PlayerDot {
                    color: theme::player_color(p.color),
                    answered: p.answer.is_some(),
                })
                .collect(),
            show_reset: true,
        }
    }

    fn reveal_data(game: &Game, q: &Question, index: usize, elapsed_ms: u32) -> RevealData {
        let mut counts = [0_usize; ANSWER_COUNT];
        let mut names: [String; ANSWER_COUNT] = Default::default();
        let mut fastest: Option<&Player> = None;
        for p in game.players() {
            let Some(a) = p.answer else { continue };
            let Some(slot) = counts.get_mut(a.choice) else {
                continue;
            };
            *slot += 1;
            if !names[a.choice].is_empty() {
                names[a.choice].push_str(", ");
            }
            names[a.choice].push_str(&p.name);
            if a.choice == q.correct
                && fastest.is_none_or(|f| f.answer.is_some_and(|fa| a.elapsed_ms < fa.elapsed_ms))
            {
                fastest = Some(p);
            }
        }
        RevealData {
            category: q.category.clone(),
            difficulty: q.difficulty.label(),
            index,
            total: game.question_total(),
            answers: q.answers.clone(),
            correct: q.correct,
            why: q.why.clone(),
            counts,
            names,
            players: game.connected().count(),
            fastest: fastest.map(|p| (p.name.clone(), points_text(p.last_gain))),
            next_in_s: REVEAL_MS.saturating_sub(elapsed_ms).div_ceil(1_000),
            show_reset: true,
        }
    }

    fn standings_rows(game: &Game) -> Vec<StandingRow> {
        game.standings()
            .into_iter()
            .map(|p| StandingRow {
                name: p.name.clone(),
                color: theme::player_color(p.color),
                score: points_text(p.score),
                gain: (p.last_gain > 0).then(|| points_text(p.last_gain)),
            })
            .collect()
    }

    fn standings_data(game: &Game, index: usize, elapsed_ms: u32) -> StandingsData {
        let asked = index + 1;
        let left = game.question_total().saturating_sub(asked);
        let secs = STANDINGS_MS.saturating_sub(elapsed_ms).div_ceil(1_000);
        let status_line = if left == 0 {
            fmt!("After question {asked} \u{b7} Podium in {secs} s")
        } else {
            fmt!("After question {asked} \u{b7} {left} to go \u{b7} Next in {secs} s")
        };
        StandingsData {
            rows: standings_rows(game),
            is_final: false,
            status_line,
            footnote: None,
            show_reset: true,
        }
    }

    fn podium_data(game: &Game, elapsed_ms: u32) -> StandingsData {
        let secs = PODIUM_MS.saturating_sub(elapsed_ms).div_ceil(1_000);
        StandingsData {
            rows: standings_rows(game),
            is_final: true,
            status_line: fmt!("Host taps New game \u{b7} Lobby in {secs} s"),
            footnote: kv::get_string(KV_BEST_LINE),
            show_reset: true,
        }
    }

    fn build_tree(game: &mut Game, params: &Params, width: f32, height: f32) -> (Node, u32) {
        match game.phase() {
            Phase::Lobby => {
                if DECK_STALE.get() {
                    reload_deck(game, params);
                }
                (
                    lobby_view(width, height, &lobby_data(game, params)),
                    LOBBY_FRAME_MS,
                )
            }
            Phase::Question { index, .. } => match game.current_question() {
                Some(q) => (
                    question_view(width, height, &question_data(game, q, index)),
                    QUESTION_FRAME_MS,
                ),
                None => (
                    lobby_view(width, height, &lobby_data(game, params)),
                    LOBBY_FRAME_MS,
                ),
            },
            Phase::Reveal { index, elapsed_ms } => match game.current_question() {
                Some(q) => (
                    reveal_view(width, height, &reveal_data(game, q, index, elapsed_ms)),
                    SLOW_FRAME_MS,
                ),
                None => (
                    lobby_view(width, height, &lobby_data(game, params)),
                    LOBBY_FRAME_MS,
                ),
            },
            Phase::Standings { index, elapsed_ms } => (
                standings_view(width, height, &standings_data(game, index, elapsed_ms)),
                SLOW_FRAME_MS,
            ),
            Phase::Podium { elapsed_ms } => (
                standings_view(width, height, &podium_data(game, elapsed_ms)),
                SLOW_FRAME_MS,
            ),
        }
    }

    // ── Lifecycle ────────────────────────────────────────────────────

    #[unsafe(no_mangle)]
    pub extern "C" fn init() {
        let params = Params::current();
        GAME.with_borrow_mut(|game| {
            game.set_settings(settings_from(&params));
            reload_deck(game, &params);
        });
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn render(delta_ms: u32) {
        ensure_listener();
        let ws = widget_size();
        let (width, height) = (px(ws.width), px(ws.height));
        let params = Params::current();

        let (tree, next_frame_ms) = GAME.with_borrow_mut(|game| {
            game.tick(delta_ms);
            if let Some(kind) = effects::on_frame(game, params.sounds) {
                on_entered(kind, game);
            }
            build_tree(game, &params, width, height)
        });

        let result = render_ui(ws.width, ws.height, tree);
        if result.clicks.contains_key(RESET_KEY) {
            GAME.with_borrow_mut(Game::reset_all);
            request_frame();
        }
        request_frame_after(next_frame_ms);
    }

    /// The host never renders on touch by itself; the lobby's Reset control
    /// needs a frame to be read back.
    #[unsafe(no_mangle)]
    pub extern "C" fn on_touch() {
        request_frame();
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn on_params_update() {
        let params = Params::current();
        GAME.with_borrow_mut(|game| game.set_settings(settings_from(&params)));
        // The deck follows the pack and difficulty params; swapping it under
        // a running game would move the question on screen, so wait for the lobby.
        DECK_STALE.set(true);
        request_frame();
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn on_network_update() {
        // The lobby shows the SSID and the address; both come from here.
        request_frame();
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn unload() {
        effects::shutdown();
        close_listener();
    }
}
