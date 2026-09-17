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

//! The question on the board: text, four cards, the countdown ring, and who
//! has answered.

#[cfg_attr(
    not(test),
    expect(
        clippy::wildcard_imports,
        reason = "screen code uses many SDK builders, macros, and tokens"
    )
)]
use bmc_wasm_sdk::*;

use crate::model::{ANSWER_COUNT, push_usize};
use crate::screens::lobby::RESET_KEY;
use crate::screens::parts::{CardState, answer_card, body, chip, topbar};
use crate::theme::{
    ACCENT, ANSWER_COLORS, BG, CARD, FAINT, FONT_CAPTION, MUTED, PAGE_PAD, RULE, TEXT,
};

const SIDE_W: f32 = 150.0;
const GAP: f32 = 36.0;
const RING: f32 = 140.0;
const RING_R: f32 = 60.0;
const RING_W: f32 = 12.0;
const QUESTION_H: f32 = 96.0;
const TAU: f32 = core::f32::consts::TAU;

/// One connected player as a dot under the ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerDot {
    pub color: Color,
    pub answered: bool,
}

/// Everything the question screen draws.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestionData {
    pub category: String,
    pub difficulty: &'static str,
    /// Zero-based.
    pub index: usize,
    pub total: usize,
    pub text: String,
    pub answers: [String; ANSWER_COUNT],
    pub remaining_ms: u32,
    pub total_ms: u32,
    pub dots: Vec<PlayerDot>,
    /// Offer the on-Deck Reset control (clears every seat). Click key `reset`.
    pub show_reset: bool,
}

#[must_use]
pub fn question_view(width: f32, height: f32, data: &QuestionData) -> Node {
    let answered = data.dots.iter().filter(|d| d.answered).count();
    let mut status = String::from("Question ");
    push_usize(&mut status, data.index + 1);
    status.push_str(" of ");
    push_usize(&mut status, data.total);
    status.push_str(" \u{b7} ");
    push_usize(&mut status, answered);
    status.push_str(" of ");
    push_usize(&mut status, data.dots.len());
    status.push_str(" answered");

    let board_w = width - 2.0 * PAGE_PAD - GAP - SIDE_W;
    col(
        props!(width: width, height: height, background: BG),
        [
            topbar(
                vec![
                    chip(&data.category, BLUE_50, BG),
                    chip(data.difficulty, CARD, TEXT),
                ],
                &status,
                data.show_reset
                    .then(|| button!(RESET_KEY, "Reset", style: Ghost, size: Small)),
            ),
            body(
                22.0,
                18.0,
                row(
                    props!(flex: 1.0, gap: GAP),
                    [board(board_w, data), side(data)],
                ),
            ),
        ],
    )
}

fn board(board_w: f32, data: &QuestionData) -> Node {
    col(
        props!(flex: 1.0, gap: 18.0),
        [
            canvas(
                props!(height: QUESTION_H),
                [Draw::autofit_text(
                    0.0,
                    0.0,
                    board_w,
                    QUESTION_H,
                    data.text.as_str(),
                    style!(size: 40, weight: FontWeight::BOLD, color: TEXT, valign: VerticalAlign::Center),
                )],
            ),
            cards(&data.answers, |_| CardState::Open),
        ],
    )
}

/// Two rows of two cards; `state` decides each card's look.
#[must_use]
pub fn cards(answers: &[String; ANSWER_COUNT], state: impl Fn(usize) -> CardState) -> Node {
    let card = |i: usize| answer_card(i, &answers[i], ANSWER_COLORS[i], state(i));
    col(
        props!(gap: 16.0),
        [
            row(props!(gap: 16.0), [card(0), card(1)]),
            row(props!(gap: 16.0), [card(2), card(3)]),
        ],
    )
}

fn side(data: &QuestionData) -> Node {
    let dots: Vec<Node> = data
        .dots
        .iter()
        .map(|d| {
            let draw = if d.answered {
                Draw::circle(7.0, 7.0, 7.0, d.color)
            } else {
                Draw::arc(
                    7.0,
                    7.0,
                    5.5,
                    0.0,
                    TAU,
                    2.0,
                    FAINT,
                    ArcSegments::Continuous,
                    ArcCap::Butt,
                )
            };
            canvas(props!(width: 14.0, height: 14.0), [draw])
        })
        .collect();
    col(
        props!(
            width: SIDE_W,
            cross_align: CrossAlign::Center,
            justify_content: Justify::Center,
            gap: 14.0,
        ),
        [
            ring(data.remaining_ms, data.total_ms),
            text(
                "SECONDS",
                style!(size: FONT_CAPTION, weight: FontWeight::SEMIBOLD, color: MUTED),
            ),
            row(
                props!(gap: 6.0, wrap: true, justify_content: Justify::Center),
                dots,
            ),
        ],
    )
}

/// Seconds left, rounded up so the ring never shows 0 while time remains.
#[must_use]
pub fn seconds_left(remaining_ms: u32) -> u32 {
    remaining_ms.div_ceil(1_000)
}

fn ring(remaining_ms: u32, total_ms: u32) -> Node {
    let c = RING / 2.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "millisecond counts are far below 2^24"
    )]
    let fraction = if total_ms == 0 {
        0.0
    } else {
        (remaining_ms.min(total_ms) as f32) / (total_ms as f32)
    };
    let mut secs = String::new();
    push_usize(
        &mut secs,
        usize::try_from(seconds_left(remaining_ms)).unwrap_or(0),
    );
    let mut draws = vec![Draw::arc(
        c,
        c,
        RING_R,
        0.0,
        TAU,
        RING_W,
        RULE,
        ArcSegments::Continuous,
        ArcCap::Butt,
    )];
    if fraction > 0.005 {
        draws.push(Draw::arc(
            c,
            c,
            RING_R,
            0.0,
            TAU * fraction,
            RING_W,
            ACCENT,
            ArcSegments::Continuous,
            ArcCap::Round,
        ));
    }
    draws.push(Draw::autofit_text(
        0.0,
        0.0,
        RING,
        RING,
        secs,
        style!(
            size: 52,
            weight: FontWeight::BOLD,
            color: TEXT,
            align: TextAlign::Center,
            valign: VerticalAlign::Center,
        ),
    ));
    canvas(props!(width: RING, height: RING), draws)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_round_up() {
        assert_eq!(seconds_left(0), 0);
        assert_eq!(seconds_left(1), 1);
        assert_eq!(seconds_left(1_000), 1);
        assert_eq!(seconds_left(1_001), 2);
    }

    #[test]
    fn the_question_screen_builds_with_and_without_players() {
        let data = QuestionData {
            category: String::from("Science"),
            difficulty: "Medium",
            index: 2,
            total: 10,
            text: String::from("Which planet has the shortest day?"),
            answers: [
                String::from("Mars"),
                String::from("Jupiter"),
                String::from("Venus"),
                String::from("Saturn"),
            ],
            remaining_ms: 8_000,
            total_ms: 20_000,
            dots: vec![
                PlayerDot {
                    color: RED_50,
                    answered: true,
                },
                PlayerDot {
                    color: BLUE_50,
                    answered: false,
                },
            ],
            show_reset: true,
        };
        let _ = question_view(1_280.0, 480.0, &data);
        let mut empty = data;
        empty.dots.clear();
        empty.remaining_ms = 0;
        empty.total_ms = 0;
        let _ = question_view(1_280.0, 480.0, &empty);
    }
}
