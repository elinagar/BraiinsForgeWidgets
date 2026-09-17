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

//! LED strip and sound feedback, driven from phase transitions.
//!
//! The strip API is whole-strip effects (solid, breathe, chase) rather than
//! per-LED pixels, so the story's "one LED per player" becomes a flash in
//! the newest player's colour. Everything here is wasm-only: it talks to
//! the host directly.

use std::cell::Cell;

#[expect(
    clippy::wildcard_imports,
    reason = "effects use SDK audio, LED, and asset exports"
)]
use bmc_wasm_sdk::*;

use crate::model::{Game, PODIUM_MS, Phase, REVEAL_MS};
use crate::screens::question::seconds_left;
use crate::theme;

const SND_JOIN: Audio = include_audio!("assets/sounds/join.wav");
const SND_START: Audio = include_audio!("assets/sounds/start.wav");
const SND_QUESTION: Audio = include_audio!("assets/sounds/question.wav");
const SND_LOCK: Audio = include_audio!("assets/sounds/lock.wav");
const SND_TICK: Audio = include_audio!("assets/sounds/tick.wav");
const SND_CORRECT: Audio = include_audio!("assets/sounds/correct.wav");
const SND_WRONG: Audio = include_audio!("assets/sounds/wrong.wav");
const SND_STANDINGS: Audio = include_audio!("assets/sounds/standings.wav");
const SND_FANFARE: Audio = include_audio!("assets/sounds/fanfare.wav");

/// Quiet cues (answer locks, ticks) sit under the room's chatter.
const SOFT: Volume = Volume::new(55);

/// The amber chase starts with this many seconds left.
const WARN_FROM_S: u32 = 5;
/// Ticks sound on each of the last seconds from here down.
const TICK_FROM_S: u32 = 3;
const JOIN_FLASH_MS: u32 = 1_500;
const PODIUM_CHASE_MS: u32 = 3_000;
const CHASE_PERIOD_MS: u32 = 500;
const BREATHE_PERIOD_MS: u32 = 2_000;

/// Which screen a phase belongs to; transitions between kinds fire effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Lobby,
    Question,
    Reveal,
    Standings,
    Podium,
}

#[must_use]
pub fn kind(phase: Phase) -> Kind {
    match phase {
        Phase::Lobby => Kind::Lobby,
        Phase::Question { .. } => Kind::Question,
        Phase::Reveal { .. } => Kind::Reveal,
        Phase::Standings { .. } => Kind::Standings,
        Phase::Podium { .. } => Kind::Podium,
    }
}

thread_local! {
    static LAST_KIND: Cell<Kind> = const { Cell::new(Kind::Lobby) };
    static LAST_PLAYERS: Cell<usize> = const { Cell::new(0) };
    static LAST_SECOND: Cell<u32> = const { Cell::new(u32::MAX) };
    static WARNED: Cell<bool> = const { Cell::new(false) };
    static LAST_ANSWERED: Cell<usize> = const { Cell::new(0) };
}

/// Call once per frame after the game ticked. Returns the kind entered on
/// this frame, if the phase changed, so the caller can do its own bookkeeping.
pub fn on_frame(game: &Game, sounds: bool) -> Option<Kind> {
    let now = kind(game.phase());
    let last = LAST_KIND.replace(now);
    let entered = (now != last).then_some(now);
    if let Some(kind) = entered {
        on_enter(kind, game, sounds);
    }
    match now {
        Kind::Lobby => on_lobby_frame(game, sounds),
        Kind::Question => on_question_frame(game, sounds),
        Kind::Reveal | Kind::Standings | Kind::Podium => {}
    }
    entered
}

/// The widget is going away: leave the strip as we found it.
pub fn shutdown() {
    led::stop();
}

fn on_enter(kind: Kind, game: &Game, sounds: bool) {
    match kind {
        Kind::Lobby => {
            led::stop();
            LAST_PLAYERS.set(game.connected().count());
        }
        Kind::Question => {
            led::stop();
            WARNED.set(false);
            LAST_SECOND.set(u32::MAX);
            LAST_ANSWERED.set(0);
            if sounds {
                let first = matches!(game.phase(), Phase::Question { index: 0, .. });
                play(if first { &SND_START } else { &SND_QUESTION }, Volume::FULL);
            }
        }
        Kind::Reveal => {
            let (right, total) = correct_share(game);
            let majority = total > 0 && right * 2 >= total;
            let (color, sound) = if majority {
                (GREEN_50, &SND_CORRECT)
            } else {
                (RED_50, &SND_WRONG)
            };
            led::set_effect(
                LedEffect::Breathe,
                color,
                BREATHE_PERIOD_MS,
                Some(REVEAL_MS),
            );
            if sounds {
                play(sound, Volume::FULL);
            }
        }
        Kind::Standings => {
            if sounds {
                play(&SND_STANDINGS, Volume::FULL);
            }
        }
        Kind::Podium => {
            if let Some(winner) = game.standings().first() {
                led::set_effect(
                    LedEffect::Chase,
                    theme::player_color(winner.color),
                    CHASE_PERIOD_MS,
                    Some(PODIUM_CHASE_MS.min(PODIUM_MS)),
                );
            }
            if sounds {
                play(&SND_FANFARE, Volume::FULL);
            }
        }
    }
}

fn on_lobby_frame(game: &Game, sounds: bool) {
    let count = game.connected().count();
    let before = LAST_PLAYERS.replace(count);
    if count > before
        && let Some(newest) = game.connected().last()
    {
        led::set_effect(
            LedEffect::Solid,
            theme::player_color(newest.color),
            0,
            Some(JOIN_FLASH_MS),
        );
        if sounds {
            play(&SND_JOIN, Volume::FULL);
        }
    }
}

fn on_question_frame(game: &Game, sounds: bool) {
    let remaining = game.remaining_ms();
    let secs = seconds_left(remaining);
    let before = LAST_SECOND.replace(secs);
    if secs <= WARN_FROM_S && !WARNED.get() {
        WARNED.set(true);
        led::set_effect(
            LedEffect::Chase,
            theme::ACCENT,
            CHASE_PERIOD_MS,
            Some(remaining.max(1)),
        );
    }
    if sounds && secs != before && secs > 0 && secs <= TICK_FROM_S {
        play(&SND_TICK, SOFT);
    }
    let answered = game.answered_count();
    let seen = LAST_ANSWERED.replace(answered);
    if sounds && answered > seen {
        play(&SND_LOCK, SOFT);
    }
}

/// Connected players who had the answer, and how many were connected.
fn correct_share(game: &Game) -> (usize, usize) {
    let correct = game.current_question().map(|q| q.correct);
    let total = game.connected().count();
    let right = game
        .connected()
        .filter(|p| p.answer.is_some_and(|a| Some(a.choice) == correct))
        .count();
    (right, total)
}

fn play(audio: &Audio, volume: Volume) {
    audio_play(ensure_audio_registered(audio), volume);
}
