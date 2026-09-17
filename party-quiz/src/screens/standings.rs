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

//! Standings between questions and the podium at the end: the same screen
//! with a different headline.

#[cfg_attr(
    not(test),
    expect(
        clippy::wildcard_imports,
        reason = "screen code uses many SDK builders, macros, and tokens"
    )
)]
use bmc_wasm_sdk::*;

use crate::model::push_usize;
use crate::screens::parts::{avatar, body, chip, hspace, topbar, vspace};
use crate::theme::{BG, CARD, FONT_CAPTION, FONT_LABEL, MUTED, TEXT};

const LIST_W: f32 = 480.0;
const PODIUM_W: f32 = 170.0;
const PODIUM_H: [f32; 3] = [200.0, 150.0, 110.0];
/// Rows the list beside the podium can show without scrolling.
const LIST_ROWS: usize = 5;

/// One ranked player.
#[derive(Clone, Debug, PartialEq)]
pub struct StandingRow {
    pub name: String,
    pub color: Color,
    /// Formatted with the device number format.
    pub score: String,
    /// Points from the last question, formatted; `None` when nothing was earned.
    pub gain: Option<String>,
}

/// Everything the standings draw.
#[derive(Clone, Debug, PartialEq)]
pub struct StandingsData {
    /// Best first.
    pub rows: Vec<StandingRow>,
    pub is_final: bool,
    /// Right side of the top bar.
    pub status_line: String,
    /// Under the list, e.g. the best-ever line.
    pub footnote: Option<String>,
}

#[must_use]
pub fn standings_view(width: f32, height: f32, data: &StandingsData) -> Node {
    let label = if data.is_final { "Final" } else { "Standings" };
    col(
        props!(width: width, height: height, background: BG),
        [
            topbar(vec![chip(label, CARD, TEXT)], &data.status_line, None),
            body(
                20.0,
                0.0,
                row(props!(flex: 1.0, gap: 48.0), [podium(data), list(data)]),
            ),
        ],
    )
}

fn podium(data: &StandingsData) -> Node {
    // Second, first, third: the classic podium order, skipping missing seats.
    let order = [1_usize, 0, 2];
    let blocks: Vec<Node> = order
        .into_iter()
        .filter_map(|rank| data.rows.get(rank).map(|row| block(rank, row)))
        .collect();
    let mut children = Vec::new();
    if data.is_final
        && let Some(winner) = data.rows.first()
    {
        let mut headline = winner.name.clone();
        headline.push_str(" wins!");
        children.push(text(
            headline,
            style!(size: 34, weight: FontWeight::BOLD, color: TEXT, align: TextAlign::Center),
        ));
    }
    children.push(row(
        props!(flex: 1.0, cross_align: CrossAlign::End, justify_content: Justify::Center, gap: 22.0),
        blocks,
    ));
    col(
        props!(flex: 1.0, cross_align: CrossAlign::Center, gap: 12.0),
        children,
    )
}

fn block(rank: usize, row_data: &StandingRow) -> Node {
    let mut place = String::new();
    push_usize(&mut place, rank + 1);
    col(
        props!(width: PODIUM_W, cross_align: CrossAlign::Center, gap: 10.0),
        [
            avatar(&row_data.name, row_data.color, 64.0, true, None),
            col(
                props!(
                    width: PODIUM_W,
                    height: PODIUM_H[rank.min(2)],
                    border_radius: 16.0,
                    background: row_data.color,
                    cross_align: CrossAlign::Center,
                    gap: 2.0,
                ),
                [
                    vspace(12.0),
                    text(place, style!(size: 30, weight: FontWeight::BOLD, color: BG)),
                    text(
                        row_data.score.as_str(),
                        style!(size: 16, weight: FontWeight::SEMIBOLD, color: BG),
                    ),
                ],
            ),
        ],
    )
}

fn list(data: &StandingsData) -> Node {
    let mut rows: Vec<Node> = data
        .rows
        .iter()
        .enumerate()
        .skip(3)
        .take(LIST_ROWS)
        .map(|(i, r)| list_row(i + 1, r))
        .collect();
    if data.rows.len() > 3 + LIST_ROWS {
        let mut more = String::from("and ");
        push_usize(&mut more, data.rows.len() - 3 - LIST_ROWS);
        more.push_str(" more");
        rows.push(text(more, style!(size: FONT_CAPTION, color: MUTED)));
    }
    if let Some(note) = &data.footnote {
        rows.push(text(
            note.as_str(),
            style!(size: FONT_CAPTION, color: MUTED),
        ));
    }
    col(
        props!(width: LIST_W, justify_content: Justify::Center, gap: 10.0),
        rows,
    )
}

fn list_row(rank: usize, r: &StandingRow) -> Node {
    let mut place = String::new();
    push_usize(&mut place, rank);
    let mut children = vec![
        col(
            props!(width: 28.0),
            [text(
                place,
                style!(size: 20, weight: FontWeight::BOLD, color: MUTED),
            )],
        ),
        canvas(
            props!(width: 34.0, height: 34.0),
            [Draw::circle(17.0, 17.0, 17.0, r.color)],
        ),
        text(
            r.name.as_str(),
            style!(size: 18, weight: FontWeight::SEMIBOLD, color: TEXT, flex: 1.0, text_overflow: TextOverflow::Ellipsis),
        ),
    ];
    if let Some(gain) = &r.gain {
        let mut plus = String::from("+");
        plus.push_str(gain);
        children.push(text(plus, style!(size: FONT_LABEL, color: LIME_50)));
    }
    children.push(col(
        props!(width: 90.0),
        [text(
            r.score.as_str(),
            style!(size: 20, weight: FontWeight::BOLD, color: TEXT, align: TextAlign::Right),
        )],
    ));
    row(
        props!(
            gap: 16.0,
            padding: 12.0,
            border_radius: 12.0,
            background: CARD,
            cross_align: CrossAlign::Center,
        ),
        {
            let mut padded = vec![hspace(6.0)];
            padded.extend(children);
            padded.push(hspace(6.0));
            padded
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(n: usize) -> Vec<StandingRow> {
        (0..n)
            .map(|i| {
                let mut name = String::from("P");
                push_usize(&mut name, i);
                StandingRow {
                    name,
                    color: BLUE_50,
                    score: String::from("1 000"),
                    gain: (i % 2 == 0).then(|| String::from("500")),
                }
            })
            .collect()
    }

    #[test]
    fn standings_build_for_two_players_and_a_full_table() {
        let data = StandingsData {
            rows: rows(2),
            is_final: false,
            status_line: String::from("After question 3 \u{b7} 7 to go"),
            footnote: None,
        };
        let _ = standings_view(1_280.0, 480.0, &data);
        let full = StandingsData {
            rows: rows(12),
            is_final: true,
            status_line: String::from("Final standings"),
            footnote: Some(String::from("Best ever on this Deck: P0, 9 420")),
        };
        let _ = standings_view(1_280.0, 480.0, &full);
        let nobody = StandingsData {
            rows: Vec::new(),
            is_final: true,
            status_line: String::new(),
            footnote: None,
        };
        let _ = standings_view(1_280.0, 480.0, &nobody);
    }
}
