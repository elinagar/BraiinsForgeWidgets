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

//! Colours and type sizes shared by every screen.

#[cfg_attr(
    not(test),
    expect(
        clippy::wildcard_imports,
        reason = "the theme names many SDK colour constants"
    )
)]
use bmc_wasm_sdk::*;

use crate::model::PlayerColor;

pub const BG: Color = BLACK;
pub const CARD: Color = GRAY_90;
pub const CARD_LIGHT: Color = GRAY_10;
pub const RULE: Color = GRAY_80;
pub const TEXT: Color = GRAY_10;
pub const TEXT_ON_LIGHT: Color = GRAY_100;
pub const MUTED: Color = GRAY_50;
pub const FAINT: Color = GRAY_70;
pub const ACCENT: Color = GOLD_40;

pub const FONT_TITLE: u32 = 22;
pub const FONT_HEADLINE: u32 = 44;
pub const FONT_BODY: u32 = 18;
pub const FONT_LABEL: u32 = 15;
pub const FONT_CAPTION: u32 = 13;
pub const FONT_STAT: u32 = 40;

pub const PAGE_PAD: f32 = 40.0;
pub const TOPBAR_H: f32 = 56.0;
pub const RADIUS_CARD: f32 = 16.0;
pub const RADIUS_CHIP: f32 = 14.0;

/// A font size derived from a box size; layout math is f32, text sizes u32.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "box sizes are small positive pixel counts"
)]
#[must_use]
pub fn font_for(box_px: f32) -> u32 {
    (box_px.max(0.0)).round() as u32
}

/// The strip colour behind an avatar or an LED for a player's colour choice.
#[must_use]
pub fn player_color(color: PlayerColor) -> Color {
    match color {
        PlayerColor::Coral => RED_50,
        PlayerColor::Blue => BLUE_50,
        PlayerColor::Lime => LIME_50,
        PlayerColor::Violet => PURPLE_50,
        PlayerColor::Amber => ORANGE_40,
        PlayerColor::Teal => TEAL_50,
    }
}

/// The four fixed answer colours, in answer order. The phone buttons use the
/// same order so colour and shape match across the two screens.
pub const ANSWER_COLORS: [Color; 4] = [RED_50, BLUE_50, GOLD_40, LIME_50];
