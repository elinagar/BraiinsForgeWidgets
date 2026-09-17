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

//! The phone controller endpoint: a tiny HTTP API over the SDK listener.
//!
//! Everything here is pure: a request comes in as method, path, and body,
//! and a [`Response`] goes out. The wasm glue owns the listener and the
//! [`Game`]; this module owns the routes, the form parsing, and the JSON the
//! phones poll. It compiles and tests on the host.

use crate::model::{
    ANSWER_COUNT, ActionError, Game, JoinError, MAX_PLAYERS, Phase, PlayerColor, push_usize,
};

/// Bodies above this are refused before parsing.
pub const MAX_BODY_BYTES: usize = 1_024;

/// How often, in milliseconds, the phone page polls `/state` while a
/// question is open. The page reads this from the snapshot.
pub const POLL_FAST_MS: u32 = 400;
/// Poll interval outside a question.
pub const POLL_SLOW_MS: u32 = 1_000;

/// Header block for the controller page.
// The host writes `HTTP/1.1 <status>\r\nContent-Length: <n>\r\n<headers>\r\n` and
// then the body, so every header line here must end with its own `\r\n`;
// the host's final `\r\n` is the blank line before the body. Without the
// trailing break the body is glued to the last header and browsers show
// an empty page.
const HTML_HEADERS: &str =
    "Content-Type: text/html; charset=utf-8\r\nCache-Control: max-age=3600\r\n";
/// Header block for JSON replies; the phone page is same-origin so no CORS.
const JSON_HEADERS: &str =
    "Content-Type: application/json; charset=utf-8\r\nCache-Control: no-store\r\n";
const EMPTY_HEADERS: &str = "Cache-Control: no-store\r\n";

/// An inbound request as the host delivers it.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub method: &'a str,
    /// Raw request target, query string included.
    pub path: &'a str,
    pub body: &'a [u8],
}

/// What goes back to the phone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// Header lines, each terminated by `\r\n`; the host prepends
    /// `Content-Length` and appends one more `\r\n` as the blank line.
    pub headers: &'static str,
    pub body: Vec<u8>,
}

impl Response {
    fn json(status: u16, body: String) -> Self {
        Self {
            status,
            headers: JSON_HEADERS,
            body: body.into_bytes(),
        }
    }

    fn error(status: u16, code: &str) -> Self {
        let mut body = String::from("{\"error\":\"");
        body.push_str(code);
        body.push_str("\"}");
        Self::json(status, body)
    }

    fn not_modified() -> Self {
        Self {
            status: 204,
            headers: EMPTY_HEADERS,
            body: Vec::new(),
        }
    }
}

/// Something the request changed, so the Deck should repaint now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    GameChanged,
}

/// Route one request. `fresh_token` mints a seat token for a join.
pub fn handle(
    game: &mut Game,
    req: &Request<'_>,
    page: &'static [u8],
    fresh_token: &mut dyn FnMut() -> u64,
) -> (Response, Effect) {
    if req.body.len() > MAX_BODY_BYTES {
        return (Response::error(400, "too_large"), Effect::None);
    }
    let (route, query) = split_query(req.path);
    let body = core::str::from_utf8(req.body).unwrap_or("");
    match (req.method, route) {
        ("GET", "/" | "/index.html") => (
            Response {
                status: 200,
                headers: HTML_HEADERS,
                body: page.to_vec(),
            },
            Effect::None,
        ),
        ("GET", "/state") => state(game, query),
        ("POST", "/join") => join(game, body, fresh_token),
        ("POST", "/answer") => answer(game, body),
        ("POST", "/host") => host_action(game, body),
        ("POST", "/leave") => leave(game, body),
        _ => (Response::error(404, "not_found"), Effect::None),
    }
}

// ── Routes ───────────────────────────────────────────────────────────

fn join(game: &mut Game, body: &str, fresh_token: &mut dyn FnMut() -> u64) -> (Response, Effect) {
    let name = form_value(body, "name").unwrap_or_default();
    let Some(color) = form_value(body, "color")
        .as_deref()
        .and_then(PlayerColor::from_wire)
    else {
        return (Response::error(400, "bad_color"), Effect::None);
    };
    let token = fresh_token();
    match game.join(&name, color, token) {
        Ok(seat) => {
            let player = &game.players()[seat];
            let mut out = String::from("{\"token\":\"");
            push_hex(&mut out, token);
            out.push_str("\",\"seat\":");
            push_usize(&mut out, seat);
            out.push_str(",\"name\":");
            push_json_str(&mut out, &player.name);
            out.push_str(",\"color\":\"");
            out.push_str(player.color.as_wire());
            out.push_str("\"}");
            (Response::json(200, out), Effect::GameChanged)
        }
        Err(JoinError::Full) => (Response::error(400, "full"), Effect::None),
        Err(JoinError::EmptyName) => (Response::error(400, "empty_name"), Effect::None),
        Err(JoinError::ColorTaken) => (Response::error(400, "color_taken"), Effect::None),
    }
}

fn answer(game: &mut Game, body: &str) -> (Response, Effect) {
    let Some(token) = token_from(body) else {
        return (Response::error(400, "bad_token"), Effect::None);
    };
    let choice = form_value(body, "c")
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(ANSWER_COUNT);
    match game.answer(token, choice) {
        Ok(()) => (
            Response::json(200, String::from("{\"ok\":true}")),
            Effect::GameChanged,
        ),
        Err(e) => (Response::error(400, action_code(e)), Effect::None),
    }
}

fn host_action(game: &mut Game, body: &str) -> (Response, Effect) {
    let Some(token) = token_from(body) else {
        return (Response::error(400, "bad_token"), Effect::None);
    };
    let result = match form_value(body, "a").as_deref() {
        Some("start") => game.start(token),
        Some("skip") => game.skip(token),
        Some("end") => game.end(token),
        Some("new") => game.new_game(token),
        _ => return (Response::error(400, "bad_action"), Effect::None),
    };
    match result {
        Ok(()) => (
            Response::json(200, String::from("{\"ok\":true}")),
            Effect::GameChanged,
        ),
        Err(e) => (Response::error(400, action_code(e)), Effect::None),
    }
}

fn leave(game: &mut Game, body: &str) -> (Response, Effect) {
    let Some(token) = token_from(body) else {
        return (Response::error(400, "bad_token"), Effect::None);
    };
    let before = game.version();
    game.leave(token);
    let effect = if game.version() == before {
        Effect::None
    } else {
        Effect::GameChanged
    };
    (Response::json(200, String::from("{\"ok\":true}")), effect)
}

fn state(game: &mut Game, query: &str) -> (Response, Effect) {
    let known = form_value(query, "v").and_then(|v| v.parse::<u64>().ok());
    let token = form_value(query, "t").and_then(|t| parse_hex(&t));
    // A phone that polls with a token it kept is back at its seat.
    let mut effect = Effect::None;
    if let Some(token) = token {
        let before = game.version();
        game.reconnect(token);
        if game.version() != before {
            effect = Effect::GameChanged;
        }
    }
    if known == Some(game.version()) {
        return (Response::not_modified(), effect);
    }
    (Response::json(200, snapshot(game, token)), effect)
}

// ── Snapshot ─────────────────────────────────────────────────────────

/// The JSON a phone polls. Never carries question or answer text: the Deck
/// is the board, the phone only needs shapes.
#[must_use]
pub fn snapshot(game: &Game, token: Option<u64>) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("{\"v\":");
    push_u64(&mut out, game.version());
    out.push_str(",\"phase\":\"");
    out.push_str(phase_name(game.phase()));
    out.push_str("\",\"max\":");
    push_usize(&mut out, MAX_PLAYERS);
    out.push_str(",\"poll\":");
    let poll = match game.phase() {
        Phase::Question { .. } | Phase::Reveal { .. } => POLL_FAST_MS,
        Phase::Lobby | Phase::Standings { .. } | Phase::Podium { .. } => POLL_SLOW_MS,
    };
    push_u32(&mut out, poll);
    out.push_str(",\"start\":\"");
    out.push_str(match game.start_blocker() {
        None => "ok",
        Some(ActionError::NotEnoughPlayers) => "players",
        Some(ActionError::NoQuestions) => "questions",
        Some(_) => "phase",
    });
    out.push('"');

    push_players(&mut out, game);
    push_question(&mut out, game);
    push_reveal(&mut out, game);
    if let Some(player) = token.and_then(|t| game.player_by_token(t)) {
        push_you(&mut out, game, player.seat);
    }
    out.push('}');
    out
}

fn push_players(out: &mut String, game: &Game) {
    let host_seat = game.host().map(|h| h.seat);
    let ranked = game.standings();
    out.push_str(",\"players\":[");
    for (i, p) in ranked.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"n\":");
        push_json_str(out, &p.name);
        out.push_str(",\"c\":\"");
        out.push_str(p.color.as_wire());
        out.push_str("\",\"s\":");
        push_u32(out, p.score);
        out.push_str(",\"g\":");
        push_u32(out, p.last_gain);
        out.push_str(",\"h\":");
        out.push_str(if host_seat == Some(p.seat) {
            "true"
        } else {
            "false"
        });
        out.push_str(",\"a\":");
        out.push_str(if p.answer.is_some() { "true" } else { "false" });
        out.push_str(",\"on\":");
        out.push_str(if p.connected { "true" } else { "false" });
        out.push('}');
    }
    out.push(']');
}

fn push_question(out: &mut String, game: &Game) {
    let Some(q) = game.current_question() else {
        return;
    };
    let index = match game.phase() {
        Phase::Question { index, .. }
        | Phase::Reveal { index, .. }
        | Phase::Standings { index, .. } => index,
        Phase::Lobby | Phase::Podium { .. } => 0,
    };
    out.push_str(",\"q\":{\"i\":");
    push_usize(out, index);
    out.push_str(",\"n\":");
    push_usize(out, game.question_total());
    out.push_str(",\"cat\":");
    push_json_str(out, &q.category);
    out.push_str(",\"diff\":\"");
    out.push_str(q.difficulty.label());
    out.push_str("\",\"remaining\":");
    push_u32(out, game.remaining_ms());
    out.push_str(",\"total\":");
    push_u32(out, game.settings().answer_time_ms);
    out.push_str(",\"answered\":");
    push_usize(out, game.answered_count());
    out.push('}');
}

fn push_reveal(out: &mut String, game: &Game) {
    if !matches!(game.phase(), Phase::Reveal { .. } | Phase::Standings { .. }) {
        return;
    }
    let Some(q) = game.current_question() else {
        return;
    };
    let mut counts = [0_usize; ANSWER_COUNT];
    for p in game.players() {
        if let Some(a) = p.answer
            && let Some(slot) = counts.get_mut(a.choice)
        {
            *slot += 1;
        }
    }
    out.push_str(",\"r\":{\"correct\":");
    push_usize(out, q.correct);
    out.push_str(",\"counts\":[");
    for (i, c) in counts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_usize(out, *c);
    }
    out.push(']');
    if let Some(why) = &q.why {
        out.push_str(",\"why\":");
        push_json_str(out, why);
    }
    out.push('}');
}

fn push_you(out: &mut String, game: &Game, seat: usize) {
    let Some(p) = game.players().get(seat) else {
        return;
    };
    let rank = game
        .standings()
        .iter()
        .position(|r| r.seat == seat)
        .map_or(0, |i| i + 1);
    let host = game.host().is_some_and(|h| h.seat == seat);
    let correct = match (game.phase(), p.answer, game.current_question()) {
        (Phase::Reveal { .. } | Phase::Standings { .. }, Some(a), Some(q)) => {
            Some(a.choice == q.correct)
        }
        _ => None,
    };
    out.push_str(",\"you\":{\"seat\":");
    push_usize(out, seat);
    out.push_str(",\"name\":");
    push_json_str(out, &p.name);
    out.push_str(",\"color\":\"");
    out.push_str(p.color.as_wire());
    out.push_str("\",\"score\":");
    push_u32(out, p.score);
    out.push_str(",\"rank\":");
    push_usize(out, rank);
    out.push_str(",\"streak\":");
    push_u32(out, p.streak);
    out.push_str(",\"gain\":");
    push_u32(out, p.last_gain);
    out.push_str(",\"host\":");
    out.push_str(if host { "true" } else { "false" });
    if let Some(a) = p.answer {
        out.push_str(",\"choice\":");
        push_usize(out, a.choice);
    }
    if let Some(c) = correct {
        out.push_str(",\"correct\":");
        out.push_str(if c { "true" } else { "false" });
    }
    out.push('}');
}

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Lobby => "lobby",
        Phase::Question { .. } => "question",
        Phase::Reveal { .. } => "reveal",
        Phase::Standings { .. } => "standings",
        Phase::Podium { .. } => "podium",
    }
}

fn action_code(e: ActionError) -> &'static str {
    match e {
        ActionError::UnknownToken => "unknown_token",
        ActionError::NotHost => "not_host",
        ActionError::NotEnoughPlayers => "not_enough_players",
        ActionError::NoQuestions => "no_questions",
        ActionError::WrongPhase => "wrong_phase",
        ActionError::AlreadyAnswered => "already_answered",
        ActionError::BadChoice => "bad_choice",
    }
}

// ── Wire helpers ─────────────────────────────────────────────────────

/// Split a request target into its path and query string.
#[must_use]
pub fn split_query(target: &str) -> (&str, &str) {
    target.split_once('?').unwrap_or((target, ""))
}

/// Look up one key in an `application/x-www-form-urlencoded` body or query.
#[must_use]
pub fn form_value(form: &str, key: &str) -> Option<String> {
    form.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        (k == key).then(|| percent_decode(v))
    })
}

fn token_from(body: &str) -> Option<u64> {
    form_value(body, "t").and_then(|t| parse_hex(&t))
}

/// Decode `+` and `%XX` escapes; malformed escapes are kept as typed.
#[must_use]
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                (Some(hi), Some(lo)) => {
                    out.push(hi * 16 + lo);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse a token as up to sixteen hex digits.
#[must_use]
pub fn parse_hex(s: &str) -> Option<u64> {
    if s.is_empty() || s.len() > 16 {
        return None;
    }
    s.bytes().try_fold(0_u64, |acc, b| {
        hex_val(b).map(|v| (acc << 4) | u64::from(v))
    })
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Append `n` as sixteen lowercase hex digits.
pub fn push_hex(out: &mut String, n: u64) {
    for shift in (0..16).rev() {
        let nibble = ((n >> (shift * 4)) & 0xF) as usize;
        out.push(char::from(HEX_DIGITS[nibble]));
    }
}

fn push_u32(out: &mut String, n: u32) {
    push_usize(out, usize::try_from(n).unwrap_or(usize::MAX));
}

fn push_u64(out: &mut String, n: u64) {
    push_usize(out, usize::try_from(n).unwrap_or(usize::MAX));
}

/// Append `s` as a JSON string literal, quotes included.
pub fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                out.push_str("\\u00");
                let code = u32::from(c);
                let hi = ((code >> 4) & 0xF) as usize;
                let lo = (code & 0xF) as usize;
                out.push(char::from(HEX_DIGITS[hi]));
                out.push(char::from(HEX_DIGITS[lo]));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Question, QuestionDifficulty, Settings};

    const PAGE: &[u8] = b"<html>controller</html>";

    fn game_with_questions() -> Game {
        let mut game = Game::new(Settings::default());
        game.load_questions(vec![Question {
            id: String::from("q1"),
            category: String::from("Science"),
            difficulty: QuestionDifficulty::Easy,
            text: String::from("Q?"),
            answers: [
                String::from("A"),
                String::from("B"),
                String::from("C"),
                String::from("D"),
            ],
            correct: 1,
            why: Some(String::from("Because \"B\".")),
        }]);
        game
    }

    fn post<'a>(path: &'a str, body: &'a str) -> Request<'a> {
        Request {
            method: "POST",
            path,
            body: body.as_bytes(),
        }
    }

    fn get(path: &str) -> Request<'_> {
        Request {
            method: "GET",
            path,
            body: b"",
        }
    }

    fn body_text(r: &Response) -> String {
        String::from_utf8(r.body.clone()).expect("utf8 body")
    }

    #[test]
    fn the_page_is_served_at_the_root_and_nothing_else_exists() {
        let mut game = Game::default();
        let mut tokens = || 7_u64;
        let (r, e) = handle(&mut game, &get("/"), PAGE, &mut tokens);
        assert_eq!((r.status, r.body.as_slice(), e), (200, PAGE, Effect::None));
        assert!(r.headers.starts_with("Content-Type: text/html"));
        let (r, _) = handle(&mut game, &get("/admin"), PAGE, &mut tokens);
        assert_eq!(r.status, 404);
        let (r, _) = handle(&mut game, &post("/", ""), PAGE, &mut tokens);
        assert_eq!(r.status, 404);
    }

    #[test]
    fn every_header_block_ends_with_crlf_so_the_host_can_add_the_blank_line() {
        // Mirrors the host's framing: `Content-Length: n\r\n<headers>\r\n<body>`.
        for headers in [HTML_HEADERS, JSON_HEADERS, EMPTY_HEADERS] {
            assert!(headers.ends_with("\r\n"), "{headers:?}");
            assert!(!headers.ends_with("\r\n\r\n"), "{headers:?}");
            for line in headers.trim_end().split("\r\n") {
                assert!(line.contains(": "), "{line:?} is not a header line");
            }
        }
    }

    #[test]
    fn oversized_bodies_are_refused_before_parsing() {
        let mut game = Game::default();
        let mut tokens = || 7_u64;
        let big = "x".repeat(MAX_BODY_BYTES + 1);
        let (r, _) = handle(&mut game, &post("/join", &big), PAGE, &mut tokens);
        assert_eq!(
            (r.status, body_text(&r)),
            (400, String::from("{\"error\":\"too_large\"}"))
        );
    }

    #[test]
    fn a_phone_joins_and_gets_its_token_and_cleaned_name() {
        let mut game = Game::default();
        let mut tokens = || 0xDEAD_BEEF_u64;
        let (r, e) = handle(
            &mut game,
            &post("/join", "name=%20Marta+Nov%C3%A1k%20&color=blue"),
            PAGE,
            &mut tokens,
        );
        assert_eq!((r.status, e), (200, Effect::GameChanged));
        assert_eq!(
            body_text(&r),
            "{\"token\":\"00000000deadbeef\",\"seat\":0,\"name\":\"Marta Novák\",\"color\":\"blue\"}"
        );
        let (r, _) = handle(
            &mut game,
            &post("/join", "name=X&color=pink"),
            PAGE,
            &mut tokens,
        );
        assert_eq!(body_text(&r), "{\"error\":\"bad_color\"}");
        let (r, _) = handle(
            &mut game,
            &post("/join", "name=Y&color=blue"),
            PAGE,
            &mut tokens,
        );
        assert_eq!(body_text(&r), "{\"error\":\"color_taken\"}");
    }

    #[test]
    fn state_is_204_when_the_phone_already_has_this_version() {
        let mut game = game_with_questions();
        let mut tokens = || 1_u64;
        let (r, _) = handle(&mut game, &get("/state"), PAGE, &mut tokens);
        assert_eq!(r.status, 200);
        let mut same = String::from("/state?v=");
        push_usize(&mut same, usize::try_from(game.version()).expect("fits"));
        let (r, _) = handle(&mut game, &get(&same), PAGE, &mut tokens);
        assert_eq!((r.status, r.body.len()), (204, 0));
    }

    #[test]
    fn the_snapshot_carries_players_question_and_the_callers_seat_but_no_text() {
        let mut game = game_with_questions();
        game.join("Eli", PlayerColor::Coral, 0xA).expect("seat");
        game.join("Bo", PlayerColor::Blue, 0xB).expect("seat");
        game.start(0xA).expect("start");
        game.tick(5_000);
        game.answer(0xB, 1).expect("answer");
        let json = snapshot(&game, Some(0xB));
        assert!(json.contains("\"phase\":\"question\""));
        assert!(json.contains("\"remaining\":15000"));
        assert!(json.contains("\"answered\":1"));
        assert!(json.contains("\"you\":{\"seat\":1,\"name\":\"Bo\""));
        assert!(json.contains("\"choice\":1"));
        assert!(json.contains("\"host\":false"));
        assert!(
            !json.contains("Q?"),
            "question text never reaches the phone"
        );
        assert!(
            !json.contains("\"correct\""),
            "no reveal before the countdown ends"
        );
        game.tick(20_000);
        let json = snapshot(&game, Some(0xB));
        assert!(json.contains("\"phase\":\"reveal\""));
        assert!(
            json.contains(
                "\"r\":{\"correct\":1,\"counts\":[0,1,0,0],\"why\":\"Because \\\"B\\\".\"}"
            )
        );
        assert!(json.contains("\"correct\":true"));
    }

    #[test]
    fn answers_and_host_actions_need_a_valid_token() {
        let mut game = game_with_questions();
        let mut tokens = || 0xA_u64;
        handle(
            &mut game,
            &post("/join", "name=Eli&color=coral"),
            PAGE,
            &mut tokens,
        );
        let mut tokens = || 0xB_u64;
        handle(
            &mut game,
            &post("/join", "name=Bo&color=blue"),
            PAGE,
            &mut tokens,
        );
        let (r, _) = handle(
            &mut game,
            &post("/host", "t=000000000000000b&a=start"),
            PAGE,
            &mut tokens,
        );
        assert_eq!(body_text(&r), "{\"error\":\"not_host\"}");
        let (r, e) = handle(&mut game, &post("/host", "t=a&a=start"), PAGE, &mut tokens);
        assert_eq!((r.status, e), (200, Effect::GameChanged));
        let (r, _) = handle(&mut game, &post("/answer", "t=zz&c=1"), PAGE, &mut tokens);
        assert_eq!(body_text(&r), "{\"error\":\"bad_token\"}");
        let (r, _) = handle(&mut game, &post("/answer", "t=b&c=9"), PAGE, &mut tokens);
        assert_eq!(body_text(&r), "{\"error\":\"bad_choice\"}");
        let (r, e) = handle(&mut game, &post("/answer", "t=b&c=2"), PAGE, &mut tokens);
        assert_eq!(
            (body_text(&r), e),
            (String::from("{\"ok\":true}"), Effect::GameChanged)
        );
        let (r, _) = handle(&mut game, &post("/answer", "t=b&c=2"), PAGE, &mut tokens);
        assert_eq!(body_text(&r), "{\"error\":\"already_answered\"}");
        let (r, _) = handle(&mut game, &post("/host", "t=a&a=dance"), PAGE, &mut tokens);
        assert_eq!(body_text(&r), "{\"error\":\"bad_action\"}");
    }

    #[test]
    fn polling_with_a_token_seats_a_returning_phone() {
        let mut game = Game::default();
        game.join("Eli", PlayerColor::Coral, 0xA).expect("seat");
        game.leave(0xA);
        assert_eq!(game.connected().count(), 0);
        let mut tokens = || 1_u64;
        let (_, e) = handle(&mut game, &get("/state?t=a&v=0"), PAGE, &mut tokens);
        assert_eq!(e, Effect::GameChanged);
        assert_eq!(game.connected().count(), 1);
    }

    #[test]
    fn wire_helpers_round_trip() {
        assert_eq!(split_query("/state?v=3"), ("/state", "v=3"));
        assert_eq!(split_query("/"), ("/", ""));
        assert_eq!(
            form_value("a=1&b=two+words&c=%41", "b").as_deref(),
            Some("two words")
        );
        assert_eq!(form_value("a=1&b=2", "c"), None);
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%4"), "%4");
        assert_eq!(parse_hex("ff"), Some(255));
        assert_eq!(parse_hex("12345678123456789"), None);
        assert_eq!(parse_hex(""), None);
        let mut s = String::new();
        push_hex(&mut s, 0x1234_ABCD);
        assert_eq!(parse_hex(&s), Some(0x1234_ABCD));
        let mut j = String::new();
        push_json_str(&mut j, "a\"b\\c\n\u{1}");
        assert_eq!(j, "\"a\\\"b\\\\c\\n\\u0001\"");
    }
}
