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

//! Pure game model: players, questions, phases, and scoring.
//!
//! No SDK imports, so the whole game logic compiles and tests on the host.
//! The wasm entry module owns one [`Game`] and drives it from `render`
//! (`tick`) and from the phone controller endpoint (`join`, `answer`, …).

/// Seats in one game; the lobby refuses the thirteenth phone.
pub const MAX_PLAYERS: usize = 12;
/// A game needs at least two players to start.
pub const MIN_PLAYERS: usize = 2;
/// Player names are cut to this many characters.
pub const MAX_NAME_CHARS: usize = 12;
/// How long the reveal stays on screen.
pub const REVEAL_MS: u32 = 4_000;
/// How long the standings stay on screen between questions.
pub const STANDINGS_MS: u32 = 5_000;
/// How long the podium stays before the Deck returns to a fresh lobby.
pub const PODIUM_MS: u32 = 30_000;
/// Once everyone has answered, the countdown ends after this grace period.
pub const GRACE_MS: u32 = 1_000;
/// Points for a correct answer in the first instant.
pub const MAX_POINTS: u32 = 1_000;
/// Points for a correct answer exactly at the deadline.
pub const MIN_POINTS: u32 = 500;
/// Correct answers in a row before the streak bonus starts paying.
pub const STREAK_THRESHOLD: u32 = 3;
/// Bonus per correct answer once the streak threshold is reached.
pub const STREAK_BONUS: u32 = 200;
/// Every question has exactly four answers.
pub const ANSWER_COUNT: usize = 4;

/// One of the six avatar colours a player picks in the lobby.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerColor {
    Coral,
    Blue,
    Lime,
    Violet,
    Amber,
    Teal,
}

impl PlayerColor {
    /// Every colour, in the order the phone offers them.
    pub const ALL: [Self; 6] = [
        Self::Coral,
        Self::Blue,
        Self::Lime,
        Self::Violet,
        Self::Amber,
        Self::Teal,
    ];

    /// The wire name the controller page sends.
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Coral => "coral",
            Self::Blue => "blue",
            Self::Lime => "lime",
            Self::Violet => "violet",
            Self::Amber => "amber",
            Self::Teal => "teal",
        }
    }

    /// Parse the wire name; `None` for anything the phone should not send.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_wire() == s)
    }
}

/// Question difficulty as carried by a pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestionDifficulty {
    Easy,
    Medium,
    Hard,
}

impl QuestionDifficulty {
    /// Parse the pack's wire value.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "easy" => Some(Self::Easy),
            "medium" => Some(Self::Medium),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }

    /// Label shown on the Deck's category chip.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
        }
    }
}

/// One multiple-choice question from a pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub category: String,
    pub difficulty: QuestionDifficulty,
    pub text: String,
    pub answers: [String; ANSWER_COUNT],
    /// Index into `answers`.
    pub correct: usize,
    /// One-line explanation shown on the reveal; `None` when the pack has none.
    pub why: Option<String>,
}

/// A locked-in answer for the current question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answer {
    pub choice: usize,
    /// Milliseconds into the countdown when the answer was locked.
    pub elapsed_ms: u32,
}

/// One seat at the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub seat: usize,
    /// Secret the phone keeps; every request names its seat with it.
    pub token: u64,
    pub name: String,
    pub color: PlayerColor,
    pub score: u32,
    pub streak: u32,
    /// Answer for the question on screen, cleared when the next one starts.
    pub answer: Option<Answer>,
    /// Points earned on the last scored question.
    pub last_gain: u32,
    pub connected: bool,
}

/// Where the game is. Timers are in milliseconds since the phase began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Lobby,
    Question {
        index: usize,
        elapsed_ms: u32,
        /// Countdown position when the last player answered, if everyone has.
        all_answered_at: Option<u32>,
    },
    Reveal {
        index: usize,
        elapsed_ms: u32,
    },
    Standings {
        index: usize,
        elapsed_ms: u32,
    },
    Podium {
        elapsed_ms: u32,
    },
}

/// Per-game knobs, mirrored from the widget params.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub question_count: usize,
    pub answer_time_ms: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            question_count: 10,
            answer_time_ms: 20_000,
        }
    }
}

/// Why a phone could not join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinError {
    Full,
    EmptyName,
    ColorTaken,
}

/// Why a phone's action was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    UnknownToken,
    NotHost,
    NotEnoughPlayers,
    NoQuestions,
    WrongPhase,
    AlreadyAnswered,
    BadChoice,
}

/// The whole game state. Every mutation bumps `version` so the phones can
/// poll cheaply.
#[derive(Clone, Debug)]
pub struct Game {
    players: Vec<Player>,
    phase: Phase,
    deck: Vec<Question>,
    settings: Settings,
    version: u64,
}

impl Default for Game {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl Game {
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self {
            players: Vec::new(),
            phase: Phase::Lobby,
            deck: Vec::new(),
            settings,
            version: 1,
        }
    }

    // ── Read side ────────────────────────────────────────────────────

    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    #[must_use]
    pub fn settings(&self) -> Settings {
        self.settings
    }

    #[must_use]
    pub fn players(&self) -> &[Player] {
        &self.players
    }

    /// Seats whose phone is still around.
    pub fn connected(&self) -> impl Iterator<Item = &Player> {
        self.players.iter().filter(|p| p.connected)
    }

    /// The host is the earliest-joined player still connected.
    #[must_use]
    pub fn host(&self) -> Option<&Player> {
        self.connected().next()
    }

    /// Colours no connected player is using. Empty once six players sit,
    /// after which colours may repeat.
    pub fn free_colors(&self) -> impl Iterator<Item = PlayerColor> + '_ {
        PlayerColor::ALL
            .into_iter()
            .filter(move |c| !self.connected().any(|p| p.color == *c))
    }

    #[must_use]
    pub fn player_by_token(&self, token: u64) -> Option<&Player> {
        self.players.iter().find(|p| p.token == token)
    }

    /// Questions loaded and not yet exhausted by this game.
    #[must_use]
    pub fn questions_available(&self) -> usize {
        self.deck.len()
    }

    /// How many questions this game will ask: the setting, capped by the deck.
    #[must_use]
    pub fn question_total(&self) -> usize {
        self.settings.question_count.min(self.deck.len())
    }

    /// The question on screen during Question, Reveal, and Standings.
    #[must_use]
    pub fn current_question(&self) -> Option<&Question> {
        match self.phase {
            Phase::Question { index, .. }
            | Phase::Reveal { index, .. }
            | Phase::Standings { index, .. } => self.deck.get(index),
            Phase::Lobby | Phase::Podium { .. } => None,
        }
    }

    /// Connected players who have locked an answer for the current question.
    #[must_use]
    pub fn answered_count(&self) -> usize {
        self.connected().filter(|p| p.answer.is_some()).count()
    }

    /// Milliseconds left on the countdown; zero outside the Question phase.
    #[must_use]
    pub fn remaining_ms(&self) -> u32 {
        match self.phase {
            Phase::Question { elapsed_ms, .. } => {
                self.settings.answer_time_ms.saturating_sub(elapsed_ms)
            }
            Phase::Lobby
            | Phase::Reveal { .. }
            | Phase::Standings { .. }
            | Phase::Podium { .. } => 0,
        }
    }

    /// Players by score, highest first, ties by seat.
    #[must_use]
    pub fn standings(&self) -> Vec<&Player> {
        let mut ranked: Vec<&Player> = self.players.iter().collect();
        ranked.sort_by(|a, b| b.score.cmp(&a.score).then(a.seat.cmp(&b.seat)));
        ranked
    }

    /// Whether the game may start right now, and if not, why.
    #[must_use]
    pub fn start_blocker(&self) -> Option<ActionError> {
        if self.phase != Phase::Lobby {
            return Some(ActionError::WrongPhase);
        }
        if self.connected().count() < MIN_PLAYERS {
            return Some(ActionError::NotEnoughPlayers);
        }
        if self.deck.is_empty() {
            return Some(ActionError::NoQuestions);
        }
        None
    }

    // ── Setup ────────────────────────────────────────────────────────

    /// Replace the settings; takes effect at the next question.
    pub fn set_settings(&mut self, settings: Settings) {
        if self.settings != settings {
            self.settings = settings;
            self.bump();
        }
    }

    /// Replace the deck. Questions are asked in the order given, so the
    /// caller shuffles and filters out already-played IDs.
    pub fn load_questions(&mut self, questions: Vec<Question>) {
        self.deck = questions
            .into_iter()
            .filter(|q| q.correct < ANSWER_COUNT)
            .collect();
        self.bump();
    }

    // ── Phones ───────────────────────────────────────────────────────

    /// Seat a new player. Returns the seat index on success.
    pub fn join(
        &mut self,
        requested: &str,
        color: PlayerColor,
        token: u64,
    ) -> Result<usize, JoinError> {
        let cleaned = clean_name(requested);
        if cleaned.is_empty() {
            return Err(JoinError::EmptyName);
        }
        if self.connected().count() >= MAX_PLAYERS {
            return Err(JoinError::Full);
        }
        let color_in_use = self.connected().any(|p| p.color == color);
        if color_in_use && self.free_colors().next().is_some() {
            return Err(JoinError::ColorTaken);
        }
        let name = self.unique_name(cleaned);
        let seat = self.players.len();
        self.players.push(Player {
            seat,
            token,
            name,
            color,
            score: 0,
            streak: 0,
            answer: None,
            last_gain: 0,
            connected: true,
        });
        self.bump();
        Ok(seat)
    }

    /// A phone came back with a token it kept; `false` if the seat is unknown.
    pub fn reconnect(&mut self, token: u64) -> bool {
        let Some(player) = self.players.iter_mut().find(|p| p.token == token) else {
            return false;
        };
        if !player.connected {
            player.connected = true;
            self.bump();
        }
        true
    }

    /// The phone left. The seat and score stay so it can come back.
    pub fn leave(&mut self, token: u64) {
        if let Some(player) = self.players.iter_mut().find(|p| p.token == token)
            && player.connected
        {
            player.connected = false;
            self.bump();
        }
    }

    /// Host starts the game from the lobby.
    pub fn start(&mut self, token: u64) -> Result<(), ActionError> {
        self.require_host(token)?;
        if let Some(blocker) = self.start_blocker() {
            return Err(blocker);
        }
        for player in &mut self.players {
            player.score = 0;
            player.streak = 0;
            player.last_gain = 0;
            player.answer = None;
        }
        self.enter_question(0);
        Ok(())
    }

    /// A player locks in `choice` for the question on screen.
    pub fn answer(&mut self, token: u64, choice: usize) -> Result<(), ActionError> {
        let Phase::Question {
            elapsed_ms,
            all_answered_at,
            ..
        } = self.phase
        else {
            return Err(ActionError::WrongPhase);
        };
        if choice >= ANSWER_COUNT {
            return Err(ActionError::BadChoice);
        }
        let player = self
            .players
            .iter_mut()
            .find(|p| p.token == token)
            .ok_or(ActionError::UnknownToken)?;
        if player.answer.is_some() {
            return Err(ActionError::AlreadyAnswered);
        }
        player.answer = Some(Answer { choice, elapsed_ms });
        let everyone = self.connected().all(|p| p.answer.is_some());
        if everyone && all_answered_at.is_none() {
            self.phase = Phase::Question {
                index: self.question_index(),
                elapsed_ms,
                all_answered_at: Some(elapsed_ms),
            };
        }
        self.bump();
        Ok(())
    }

    /// Host jumps to the next step: closes the countdown, or shortens a
    /// reveal or standings screen.
    pub fn skip(&mut self, token: u64) -> Result<(), ActionError> {
        self.require_host(token)?;
        match self.phase {
            Phase::Question { index, .. } => self.finish_question(index),
            Phase::Reveal { index, .. } => self.enter_standings(index),
            Phase::Standings { index, .. } => self.after_standings(index),
            Phase::Lobby | Phase::Podium { .. } => return Err(ActionError::WrongPhase),
        }
        Ok(())
    }

    /// Host ends the game early: straight to the podium.
    pub fn end(&mut self, token: u64) -> Result<(), ActionError> {
        self.require_host(token)?;
        match self.phase {
            Phase::Question { .. } | Phase::Reveal { .. } | Phase::Standings { .. } => {
                self.phase = Phase::Podium { elapsed_ms: 0 };
                self.bump();
                Ok(())
            }
            Phase::Lobby | Phase::Podium { .. } => Err(ActionError::WrongPhase),
        }
    }

    /// Host returns to the lobby, keeping the seats.
    pub fn new_game(&mut self, token: u64) -> Result<(), ActionError> {
        self.require_host(token)?;
        if self.phase == Phase::Lobby {
            return Err(ActionError::WrongPhase);
        }
        self.reset_to_lobby();
        Ok(())
    }

    /// Wipe every seat and return to an empty lobby (the Deck's Reset control).
    pub fn reset_all(&mut self) {
        self.players.clear();
        self.phase = Phase::Lobby;
        self.bump();
    }

    // ── Clock ────────────────────────────────────────────────────────

    /// Advance the phase timers by `delta_ms` of wall clock.
    pub fn tick(&mut self, delta_ms: u32) {
        match self.phase {
            Phase::Lobby => {}
            Phase::Question {
                index,
                elapsed_ms,
                all_answered_at,
            } => {
                let elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                let timed_out = elapsed_ms >= self.settings.answer_time_ms;
                let grace_over = all_answered_at.is_some_and(|at| elapsed_ms >= at + GRACE_MS);
                if timed_out || grace_over {
                    self.finish_question(index);
                } else {
                    self.phase = Phase::Question {
                        index,
                        elapsed_ms,
                        all_answered_at,
                    };
                }
            }
            Phase::Reveal { index, elapsed_ms } => {
                let elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                if elapsed_ms >= REVEAL_MS {
                    self.enter_standings(index);
                } else {
                    self.phase = Phase::Reveal { index, elapsed_ms };
                }
            }
            Phase::Standings { index, elapsed_ms } => {
                let elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                if elapsed_ms >= STANDINGS_MS {
                    self.after_standings(index);
                } else {
                    self.phase = Phase::Standings { index, elapsed_ms };
                }
            }
            Phase::Podium { elapsed_ms } => {
                let elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                if elapsed_ms >= PODIUM_MS {
                    self.reset_to_lobby();
                } else {
                    self.phase = Phase::Podium { elapsed_ms };
                }
            }
        }
    }

    // ── Internals ────────────────────────────────────────────────────

    fn bump(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    fn question_index(&self) -> usize {
        match self.phase {
            Phase::Question { index, .. }
            | Phase::Reveal { index, .. }
            | Phase::Standings { index, .. } => index,
            Phase::Lobby | Phase::Podium { .. } => 0,
        }
    }

    fn require_host(&self, token: u64) -> Result<(), ActionError> {
        let player = self
            .player_by_token(token)
            .ok_or(ActionError::UnknownToken)?;
        let is_host = self.host().is_some_and(|h| h.seat == player.seat);
        if is_host {
            Ok(())
        } else {
            Err(ActionError::NotHost)
        }
    }

    fn unique_name(&self, base: String) -> String {
        let taken = |candidate: &str| {
            self.players
                .iter()
                .any(|p| p.name.eq_ignore_ascii_case(candidate))
        };
        if !taken(&base) {
            return base;
        }
        let mut n = 2_usize;
        loop {
            let mut candidate = base.clone();
            candidate.push(' ');
            push_usize(&mut candidate, n);
            if !taken(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    fn enter_question(&mut self, index: usize) {
        for player in &mut self.players {
            player.answer = None;
            player.last_gain = 0;
        }
        self.phase = Phase::Question {
            index,
            elapsed_ms: 0,
            all_answered_at: None,
        };
        self.bump();
    }

    fn finish_question(&mut self, index: usize) {
        let correct = self.deck.get(index).map(|q| q.correct);
        let answer_time_ms = self.settings.answer_time_ms;
        for player in &mut self.players {
            let hit = player.answer.filter(|a| Some(a.choice) == correct);
            if let Some(answer) = hit {
                player.streak += 1;
                let bonus = if player.streak > STREAK_THRESHOLD {
                    STREAK_BONUS
                } else {
                    0
                };
                player.last_gain = points_for(answer.elapsed_ms, answer_time_ms) + bonus;
                player.score += player.last_gain;
            } else {
                player.streak = 0;
                player.last_gain = 0;
            }
        }
        self.phase = Phase::Reveal {
            index,
            elapsed_ms: 0,
        };
        self.bump();
    }

    fn enter_standings(&mut self, index: usize) {
        self.phase = Phase::Standings {
            index,
            elapsed_ms: 0,
        };
        self.bump();
    }

    fn after_standings(&mut self, index: usize) {
        let next = index + 1;
        if next < self.question_total() {
            self.enter_question(next);
        } else {
            self.phase = Phase::Podium { elapsed_ms: 0 };
            self.bump();
        }
    }

    fn reset_to_lobby(&mut self) {
        // Seats whose phone is gone are dropped so their colour frees up.
        self.players.retain(|p| p.connected);
        for (seat, player) in self.players.iter_mut().enumerate() {
            player.seat = seat;
            player.score = 0;
            player.streak = 0;
            player.last_gain = 0;
            player.answer = None;
        }
        self.phase = Phase::Lobby;
        self.bump();
    }
}

/// Points for a correct answer locked `elapsed_ms` into a countdown of
/// `answer_time_ms`: linear from [`MAX_POINTS`] down to [`MIN_POINTS`].
#[must_use]
pub fn points_for(elapsed_ms: u32, answer_time_ms: u32) -> u32 {
    if answer_time_ms == 0 {
        return MIN_POINTS;
    }
    let elapsed = u64::from(elapsed_ms.min(answer_time_ms));
    let span = u64::from(MAX_POINTS - MIN_POINTS);
    let decay = span * elapsed / u64::from(answer_time_ms);
    MAX_POINTS - u32::try_from(decay).unwrap_or(MAX_POINTS - MIN_POINTS)
}

/// Trim, collapse inner whitespace to single spaces, and cut to
/// [`MAX_NAME_CHARS`] characters.
#[must_use]
pub fn clean_name(raw: &str) -> String {
    let mut out = String::new();
    let mut count = 0_usize;
    for word in raw.split_whitespace() {
        if count > 0 {
            out.push(' ');
            count += 1;
        }
        for ch in word.chars() {
            if count >= MAX_NAME_CHARS {
                break;
            }
            if !ch.is_control() {
                out.push(ch);
                count += 1;
            }
        }
        if count >= MAX_NAME_CHARS {
            break;
        }
    }
    out.trim_end().to_owned()
}

/// Append a decimal number without pulling `core::fmt` into the binary.
pub fn push_usize(out: &mut String, mut n: usize) {
    if n == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0_u8; 20];
    let mut len = 0_usize;
    while n > 0 {
        digits[len] = b'0' + u8::try_from(n % 10).unwrap_or(0);
        n /= 10;
        len += 1;
    }
    for digit in digits[..len].iter().rev() {
        out.push(char::from(*digit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(n: usize) -> Question {
        let mut id = String::from("q");
        push_usize(&mut id, n);
        Question {
            id,
            category: String::from("Science"),
            difficulty: QuestionDifficulty::Medium,
            text: String::from("Which planet has the shortest day?"),
            answers: [
                String::from("Mars"),
                String::from("Jupiter"),
                String::from("Venus"),
                String::from("Saturn"),
            ],
            correct: 1,
            why: Some(String::from("Jupiter spins once every 9 h 56 min.")),
        }
    }

    fn table(players: usize, questions: usize) -> Game {
        let mut game = Game::new(Settings {
            question_count: questions,
            answer_time_ms: 20_000,
        });
        game.load_questions((0..questions).map(question).collect());
        for (i, color) in PlayerColor::ALL.iter().enumerate().take(players) {
            let mut name = String::from("P");
            push_usize(&mut name, i);
            game.join(&name, *color, 100 + i as u64).expect("seat");
        }
        game
    }

    #[test]
    fn points_decay_linearly_from_max_to_min() {
        assert_eq!(points_for(0, 20_000), MAX_POINTS);
        assert_eq!(points_for(10_000, 20_000), 750);
        assert_eq!(points_for(20_000, 20_000), MIN_POINTS);
        assert_eq!(points_for(99_000, 20_000), MIN_POINTS);
    }

    #[test]
    fn names_are_trimmed_capped_and_deduplicated() {
        assert_eq!(clean_name("  Marta   Nováková  "), "Marta Nováko");
        assert_eq!(clean_name("\t\n"), "");
        let mut game = Game::default();
        game.join("Eli", PlayerColor::Coral, 1).expect("seat");
        game.join("eli", PlayerColor::Blue, 2).expect("seat");
        game.join("ELI", PlayerColor::Lime, 3).expect("seat");
        let names: Vec<&str> = game.players().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Eli", "eli 2", "ELI 3"]);
    }

    #[test]
    fn join_refuses_empty_names_taken_colours_and_a_full_table() {
        let mut game = Game::default();
        assert_eq!(
            game.join("   ", PlayerColor::Coral, 1),
            Err(JoinError::EmptyName)
        );
        game.join("A", PlayerColor::Coral, 1).expect("seat");
        assert_eq!(
            game.join("B", PlayerColor::Coral, 2),
            Err(JoinError::ColorTaken)
        );
        for (i, color) in PlayerColor::ALL.iter().enumerate().skip(1) {
            game.join("B", *color, 10 + i as u64).expect("seat");
        }
        assert_eq!(game.free_colors().count(), 0);
        // Six colours, twelve seats: once every colour is in use they repeat.
        for i in 6..MAX_PLAYERS {
            game.join("C", PlayerColor::Coral, 100 + i as u64)
                .expect("colours repeat once all six are taken");
        }
        assert_eq!(
            game.join("Late", PlayerColor::Blue, 9_999),
            Err(JoinError::Full)
        );
    }

    #[test]
    fn host_is_the_earliest_connected_player() {
        let mut game = table(3, 1);
        assert_eq!(game.host().map(|p| p.seat), Some(0));
        game.leave(100);
        assert_eq!(game.host().map(|p| p.seat), Some(1));
        assert!(game.reconnect(100));
        assert_eq!(game.host().map(|p| p.seat), Some(0));
        assert!(!game.reconnect(4_242));
    }

    #[test]
    fn start_needs_the_host_two_players_and_questions() {
        let mut game = table(1, 1);
        assert_eq!(game.start(100), Err(ActionError::NotEnoughPlayers));
        game.join("B", PlayerColor::Blue, 101).expect("seat");
        assert_eq!(game.start(101), Err(ActionError::NotHost));
        let mut empty = table(2, 0);
        assert_eq!(empty.start(100), Err(ActionError::NoQuestions));
        assert_eq!(game.start(100), Ok(()));
        assert!(matches!(game.phase(), Phase::Question { index: 0, .. }));
        assert_eq!(game.start(100), Err(ActionError::WrongPhase));
    }

    #[test]
    fn answers_lock_once_and_score_by_speed() {
        let mut game = table(2, 1);
        game.start(100).expect("start");
        game.tick(2_000);
        assert_eq!(game.answer(100, 1), Ok(()));
        assert_eq!(game.answer(100, 2), Err(ActionError::AlreadyAnswered));
        assert_eq!(game.answer(101, 7), Err(ActionError::BadChoice));
        assert_eq!(game.answer(999, 1), Err(ActionError::UnknownToken));
        game.tick(8_000);
        game.answer(101, 0).expect("wrong answer still locks");
        assert_eq!(game.answered_count(), 2);
        // Everyone answered: the countdown closes after the grace period.
        game.tick(GRACE_MS - 1);
        assert!(matches!(game.phase(), Phase::Question { .. }));
        game.tick(1);
        assert!(matches!(game.phase(), Phase::Reveal { index: 0, .. }));
        let p0 = game.player_by_token(100).expect("p0");
        let p1 = game.player_by_token(101).expect("p1");
        assert_eq!(p0.score, points_for(2_000, 20_000));
        assert_eq!(p0.last_gain, 950);
        assert_eq!(p1.score, 0);
        assert_eq!(p1.streak, 0);
    }

    #[test]
    fn streak_bonus_starts_after_three_in_a_row() {
        let mut game = table(2, 5);
        game.start(100).expect("start");
        let mut gains = Vec::new();
        for _ in 0..5 {
            game.answer(100, 1).expect("answer");
            game.tick(20_000);
            gains.push(game.player_by_token(100).expect("p0").last_gain);
            game.tick(REVEAL_MS);
            game.tick(STANDINGS_MS);
        }
        assert_eq!(gains, [1_000, 1_000, 1_000, 1_200, 1_200]);
        assert!(matches!(game.phase(), Phase::Podium { .. }));
    }

    #[test]
    fn a_timed_out_question_moves_to_reveal_then_standings_then_next() {
        let mut game = table(2, 2);
        game.start(100).expect("start");
        game.tick(19_999);
        assert!(matches!(game.phase(), Phase::Question { .. }));
        game.tick(1);
        assert!(matches!(game.phase(), Phase::Reveal { index: 0, .. }));
        game.tick(REVEAL_MS);
        assert!(matches!(game.phase(), Phase::Standings { index: 0, .. }));
        game.tick(STANDINGS_MS);
        assert!(matches!(game.phase(), Phase::Question { index: 1, .. }));
        assert_eq!(game.answered_count(), 0);
    }

    #[test]
    fn host_can_skip_end_and_restart() {
        let mut game = table(2, 3);
        assert_eq!(game.skip(100), Err(ActionError::WrongPhase));
        game.start(100).expect("start");
        assert_eq!(game.skip(101), Err(ActionError::NotHost));
        game.skip(100).expect("skip closes the countdown");
        assert!(matches!(game.phase(), Phase::Reveal { .. }));
        game.skip(100).expect("skip the reveal");
        assert!(matches!(game.phase(), Phase::Standings { .. }));
        game.end(100).expect("end early");
        assert!(matches!(game.phase(), Phase::Podium { .. }));
        game.leave(101);
        game.new_game(100).expect("back to lobby");
        assert_eq!(game.phase(), Phase::Lobby);
        assert_eq!(game.players().len(), 1, "disconnected seats are dropped");
        assert_eq!(game.players()[0].score, 0);
    }

    #[test]
    fn podium_returns_to_the_lobby_on_its_own() {
        let mut game = table(2, 1);
        game.start(100).expect("start");
        game.tick(20_000);
        game.tick(REVEAL_MS);
        game.tick(STANDINGS_MS);
        assert!(matches!(game.phase(), Phase::Podium { .. }));
        game.tick(PODIUM_MS);
        assert_eq!(game.phase(), Phase::Lobby);
    }

    #[test]
    fn standings_rank_by_score_then_seat() {
        let mut game = table(3, 1);
        game.start(100).expect("start");
        game.tick(1_000);
        game.answer(102, 1).expect("fast and right");
        game.tick(5_000);
        game.answer(100, 1).expect("slower and right");
        game.answer(101, 0).expect("wrong");
        game.tick(20_000);
        let seats: Vec<usize> = game.standings().iter().map(|p| p.seat).collect();
        assert_eq!(seats, [2, 0, 1]);
    }

    #[test]
    fn reset_all_empties_the_table_from_any_phase() {
        let mut game = table(3, 2);
        game.start(100).expect("start");
        game.reset_all();
        assert_eq!(game.phase(), Phase::Lobby);
        assert!(game.players().is_empty());
        assert_eq!(game.answer(100, 1), Err(ActionError::WrongPhase));
    }

    #[test]
    fn every_mutation_bumps_the_version() {
        let mut game = table(2, 1);
        let v0 = game.version();
        game.leave(9_999);
        assert_eq!(game.version(), v0, "no-op leave does not bump");
        game.leave(101);
        assert!(game.version() > v0);
    }
}
