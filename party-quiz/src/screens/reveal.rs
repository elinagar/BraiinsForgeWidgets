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

//! The reveal: the right card lit, the rest dimmed, how the room voted, who
//! was fastest, and why the answer is what it is.

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
use crate::screens::parts::{CardState, body, chip, hspace, topbar};
use crate::screens::question::cards;
use crate::theme::{ACCENT, ANSWER_COLORS, BG, CARD, FONT_BODY, FONT_LABEL, MUTED, TEXT};

const SIDE_W: f32 = 460.0;
const GAP: f32 = 36.0;
const BAR_W: f32 = 160.0;
const BAR_H: f32 = 26.0;

/// Everything the reveal draws.
#[derive(Clone, Debug, PartialEq)]
pub struct RevealData {
    pub category: String,
    pub difficulty: &'static str,
    /// Zero-based.
    pub index: usize,
    pub total: usize,
    pub answers: [String; ANSWER_COUNT],
    pub correct: usize,
    pub why: Option<String>,
    /// Votes per answer.
    pub counts: [usize; ANSWER_COUNT],
    /// Comma-joined names per answer.
    pub names: [String; ANSWER_COUNT],
    /// Connected players, so the bars have a full-width reference.
    pub players: usize,
    /// Fastest correct player and their formatted points.
    pub fastest: Option<(String, String)>,
    pub next_in_s: u32,
    /// Offer the on-Deck Reset control (clears every seat). Click key `reset`.
    pub show_reset: bool,
}

#[must_use]
pub fn reveal_view(width: f32, height: f32, data: &RevealData) -> Node {
    let mut status = String::from("Question ");
    push_usize(&mut status, data.index + 1);
    status.push_str(" of ");
    push_usize(&mut status, data.total);
    status.push_str(" \u{b7} Standings in ");
    push_usize(&mut status, usize::try_from(data.next_in_s).unwrap_or(0));
    status.push_str(" s");

    col(
        props!(width: width, height: height, background: BG),
        [
            topbar(
                vec![chip(&data.category, BLUE_50, BG), chip("Answer", TEXT, BG)],
                &status,
                data.show_reset
                    .then(|| button!(RESET_KEY, "Reset", style: Ghost, size: Small)),
            ),
            body(
                22.0,
                18.0,
                row(props!(flex: 1.0, gap: GAP), [board(data), side(data)]),
            ),
        ],
    )
}

fn board(data: &RevealData) -> Node {
    let correct = data.correct.min(ANSWER_COUNT - 1);
    let mut headline = vec![text(
        data.answers[correct].as_str(),
        style!(size: 34, weight: FontWeight::BOLD, color: ANSWER_COLORS[correct]),
    )];
    if let Some(why) = &data.why {
        headline.push(text(
            why.as_str(),
            style!(size: FONT_BODY, color: MUTED, flex: 1.0, text_overflow: TextOverflow::Ellipsis),
        ));
    }
    col(
        props!(flex: 1.0, gap: 18.0),
        [
            row(
                props!(gap: 16.0, cross_align: CrossAlign::Center, height: 96.0),
                headline,
            ),
            cards(&data.answers, |i| {
                if i == correct {
                    CardState::Correct
                } else {
                    CardState::Dim
                }
            }),
        ],
    )
}

fn side(data: &RevealData) -> Node {
    let mut rows: Vec<Node> = (0..ANSWER_COUNT)
        .map(|i| {
            vote_bar(
                &data.answers[i],
                ANSWER_COLORS[i],
                data.counts[i],
                data.players,
                &data.names[i],
            )
        })
        .collect();
    rows.push(fastest_card(data.fastest.as_ref()));
    col(
        props!(width: SIDE_W, justify_content: Justify::Center, gap: 16.0),
        rows,
    )
}

fn vote_bar(label: &str, color: Color, count: usize, players: usize, names: &str) -> Node {
    #[expect(clippy::cast_precision_loss, reason = "player counts are tiny")]
    let fill_w = if players == 0 {
        0.0
    } else {
        BAR_W * (count.min(players) as f32) / (players as f32)
    };
    row(
        props!(gap: 14.0, cross_align: CrossAlign::Center),
        [
            canvas(
                props!(width: 16.0, height: 16.0),
                [Draw::rect(0.0, 0.0, 16.0, 16.0, color)],
            ),
            col(
                props!(width: 90.0),
                [text(
                    label,
                    style!(size: 16, weight: FontWeight::SEMIBOLD, color: TEXT, text_overflow: TextOverflow::Ellipsis),
                )],
            ),
            row(
                props!(width: BAR_W, height: BAR_H, border_radius: 6.0, background: CARD),
                [col(
                    props!(width: fill_w, height: BAR_H, background: color),
                    [],
                )],
            ),
            col(
                props!(flex: 1.0),
                [text(
                    names,
                    style!(size: FONT_LABEL, color: MUTED, text_overflow: TextOverflow::Ellipsis),
                )],
            ),
        ],
    )
}

fn fastest_card(fastest: Option<&(String, String)>) -> Node {
    let value: Vec<Node> = match fastest {
        Some((name, points)) => vec![
            text(
                name.as_str(),
                style!(size: 20, weight: FontWeight::BOLD, color: TEXT),
            ),
            text(
                {
                    let mut s = String::from("+");
                    s.push_str(points);
                    s
                },
                style!(size: 20, weight: FontWeight::BOLD, color: ACCENT),
            ),
        ],
        None => vec![text(
            "Nobody this time",
            style!(size: FONT_LABEL, color: MUTED),
        )],
    };
    row(
        props!(
            padding: 14.0,
            border_radius: 14.0,
            background: CARD,
            cross_align: CrossAlign::Center,
        ),
        [
            hspace(4.0),
            text("Fastest correct", style!(size: FONT_LABEL, color: MUTED)),
            spacer(1.0),
            row(props!(gap: 8.0), value),
            hspace(4.0),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reveal_builds_with_and_without_a_winner() {
        let data = RevealData {
            category: String::from("Science"),
            difficulty: "Medium",
            index: 2,
            total: 10,
            answers: [
                String::from("Mars"),
                String::from("Jupiter"),
                String::from("Venus"),
                String::from("Saturn"),
            ],
            correct: 1,
            why: Some(String::from("Jupiter spins once every 9 h 56 min.")),
            counts: [1, 2, 0, 1],
            names: [
                String::from("Petra"),
                String::from("Eli, Marta"),
                String::new(),
                String::from("Jonas"),
            ],
            players: 4,
            fastest: Some((String::from("Marta"), String::from("980"))),
            next_in_s: 4,
            show_reset: true,
        };
        let _ = reveal_view(1_280.0, 480.0, &data);
        let mut nobody = data;
        nobody.fastest = None;
        nobody.why = None;
        nobody.players = 0;
        nobody.correct = 99;
        let _ = reveal_view(1_280.0, 480.0, &nobody);
    }
}
