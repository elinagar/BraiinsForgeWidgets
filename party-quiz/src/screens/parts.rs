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

//! Pieces shared by several screens: the top bar, chips, cards, avatars.

#[cfg_attr(
    not(test),
    expect(
        clippy::wildcard_imports,
        reason = "screen code uses many SDK builders, macros, and tokens"
    )
)]
use bmc_wasm_sdk::*;

use crate::theme::{
    CARD, FONT_CAPTION, FONT_LABEL, FONT_TITLE, MUTED, PAGE_PAD, RADIUS_CARD, RADIUS_CHIP, RULE,
    TEXT, TOPBAR_H,
};

/// A fixed-width gap inside a row.
#[must_use]
pub fn hspace(width: f32) -> Node {
    col(props!(width: width), [])
}

/// A fixed-height gap inside a column.
#[must_use]
pub fn vspace(height: f32) -> Node {
    col(props!(height: height), [])
}

/// The area under the top bar: page side margins and `top`/`bottom` room
/// around `inner`, which stretches to fill (`flex: 1.0`).
///
/// `PropsData` has only a uniform `padding` and its `inset_*` fields mean
/// absolute positioning, so asymmetric spacing is built from spacer nodes.
#[must_use]
pub fn body(top: f32, bottom: f32, inner: Node) -> Node {
    col(
        props!(flex: 1.0),
        [
            vspace(top),
            row(
                props!(flex: 1.0),
                [hspace(PAGE_PAD), inner, hspace(PAGE_PAD)],
            ),
            vspace(bottom),
        ],
    )
}

/// The bar across the top of every screen: the game name, a chip naming the
/// phase, and a right-aligned status line.
#[must_use]
pub fn topbar(chips: Vec<Node>, status: &str, trailing: Option<Node>) -> Node {
    let mut left = vec![text(
        "PARTY QUIZ",
        style!(size: FONT_TITLE, weight: FontWeight::BOLD, color: TEXT),
    )];
    left.extend(chips);
    let mut right = vec![text(status, style!(size: FONT_LABEL, color: MUTED))];
    right.extend(trailing);
    col(
        props!(height: TOPBAR_H),
        [
            row(
                props!(flex: 1.0, cross_align: CrossAlign::Center),
                [
                    hspace(PAGE_PAD),
                    row(props!(gap: 14.0, cross_align: CrossAlign::Center), left),
                    spacer(1.0),
                    row(props!(gap: 20.0, cross_align: CrossAlign::Center), right),
                    hspace(PAGE_PAD),
                ],
            ),
            col(props!(height: 1.0, background: RULE), []),
        ],
    )
}

/// A small rounded label.
#[must_use]
pub fn chip(label: &str, background: Color, foreground: Color) -> Node {
    row(
        props!(
            padding: 6.0,
            border_radius: RADIUS_CHIP,
            background: background,
            cross_align: CrossAlign::Center,
        ),
        [
            hspace(4.0),
            text(
                label,
                style!(size: FONT_LABEL, weight: FontWeight::SEMIBOLD, color: foreground),
            ),
            hspace(4.0),
        ],
    )
}

/// A dark rounded card with a small caption above its content.
#[must_use]
pub fn stat_card(caption: &str, content: Vec<Node>) -> Node {
    let mut children = vec![text(
        caption,
        style!(size: FONT_CAPTION, weight: FontWeight::SEMIBOLD, color: MUTED),
    )];
    children.extend(content);
    col(
        props!(padding: 18.0, gap: 6.0, border_radius: RADIUS_CARD, background: CARD),
        children,
    )
}

/// A round avatar in the player's colour with their initial, a name below.
#[must_use]
pub fn avatar(name: &str, color: Color, size: f32, show_name: bool, caption: Option<&str>) -> Node {
    let initial: String = name.chars().take(1).collect::<String>().to_uppercase();
    let font = crate::theme::font_for(size * 0.4);
    let mut children = vec![canvas(
        props!(width: size, height: size),
        [
            Draw::circle(size / 2.0, size / 2.0, size / 2.0, color),
            Draw::autofit_text(
                0.0,
                0.0,
                size,
                size,
                initial,
                style!(
                    size: font,
                    weight: FontWeight::BOLD,
                    color: BLACK,
                    align: TextAlign::Center,
                    valign: VerticalAlign::Center,
                ),
            ),
        ],
    )];
    if show_name {
        children.push(text(name, style!(size: FONT_LABEL, color: TEXT)));
    }
    if let Some(caption) = caption {
        children.push(text(
            caption,
            style!(size: FONT_CAPTION, weight: FontWeight::SEMIBOLD, color: crate::theme::ACCENT),
        ));
    }
    col(props!(gap: 8.0, cross_align: CrossAlign::Center), children)
}

/// How an answer card reads on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardState {
    /// Countdown running: full colour.
    Open,
    /// Reveal: this was the answer.
    Correct,
    /// Reveal: this was not the answer.
    Dim,
}

const CARD_H: f32 = 108.0;
const SHAPE_BOX: f32 = 54.0;

/// One of the four answer cards: colour and shape fixed by position so the
/// phone's buttons match, the text from the question.
#[must_use]
pub fn answer_card(index: usize, label: &str, color: Color, state: CardState) -> Node {
    let (background, ink, border) = match state {
        CardState::Open => (color, BLACK, 0.0),
        CardState::Correct => (color, BLACK, 4.0),
        CardState::Dim => (color.scale_alpha(0.25), TEXT.scale_alpha(0.6), 0.0),
    };
    row(
        props!(
            flex: 1.0,
            height: CARD_H,
            border_radius: 18.0,
            background: background,
            border_width: border,
            border_color: TEXT,
            cross_align: CrossAlign::Center,
            gap: 18.0,
        ),
        [
            hspace(26.0),
            shape_box(index, ink),
            text(
                label,
                style!(
                    size: 26,
                    weight: FontWeight::BOLD,
                    color: ink,
                    flex: 1.0,
                    text_overflow: TextOverflow::Ellipsis,
                ),
            ),
            hspace(20.0),
        ],
    )
}

/// The shape for answer `index`: triangle, diamond, circle, square.
#[must_use]
pub fn shape_box(index: usize, ink: Color) -> Node {
    let s = SHAPE_BOX;
    let c = s / 2.0;
    let shape = match index {
        0 => Draw::fill_path(
            vec![(c, 10.0), (s - 8.0, s - 12.0), (8.0, s - 12.0)],
            ink,
            false,
        ),
        1 => Draw::fill_path(
            vec![(c, 9.0), (s - 9.0, c), (c, s - 9.0), (9.0, c)],
            ink,
            false,
        ),
        2 => Draw::circle(c, c, 15.0, ink),
        _ => Draw::rect(13.0, 13.0, s - 26.0, s - 26.0, ink),
    };
    canvas(
        props!(width: s, height: s),
        [Draw::rect(0.0, 0.0, s, s, TEXT.scale_alpha(0.55)), shape],
    )
}
