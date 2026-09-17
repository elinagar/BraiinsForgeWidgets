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

//! The lobby: a QR code to join, the players who have, and who hosts.

#[cfg_attr(
    not(test),
    expect(
        clippy::wildcard_imports,
        reason = "screen code uses many SDK builders, macros, and tokens"
    )
)]
use bmc_wasm_sdk::*;

use crate::screens::parts::{avatar, body, chip, stat_card, topbar};
use crate::theme::{
    ACCENT, BG, CARD, CARD_LIGHT, FAINT, FONT_BODY, FONT_HEADLINE, FONT_LABEL, FONT_STAT, MUTED,
    RADIUS_CARD, TEXT, TEXT_ON_LIGHT,
};

const QR_SIZE: f32 = 240.0;
const QR_PANEL_W: f32 = 300.0;
const SIDE_W: f32 = 260.0;
const AVATAR: f32 = 56.0;

/// Whether phones can join right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyStatus {
    /// The controller is listening at `url` and the deck has questions.
    Ready { url: String },
    /// Waiting for an address, the listener, or the first question pack.
    Loading,
    /// No cached pack and no way to fetch one.
    Offline,
}

/// One seated player as the lobby shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct LobbyPlayer {
    pub name: String,
    pub color: Color,
    pub is_host: bool,
}

/// Everything the lobby draws.
#[derive(Clone, Debug, PartialEq)]
pub struct LobbyData {
    pub status: LobbyStatus,
    pub ssid: String,
    pub players: Vec<LobbyPlayer>,
    pub max_players: usize,
    pub show_names: bool,
    /// Right side of the top bar, e.g. "10 questions · Mixed".
    pub status_line: String,
    /// "Best ever" line, if any game has been finished on this Deck.
    pub best_line: Option<String>,
    /// Offer the on-Deck Reset control (clears every seat). Click key `reset`.
    pub show_reset: bool,
}

/// Click key of the lobby's Reset control.
pub const RESET_KEY: &str = "reset";

#[must_use]
pub fn lobby_view(width: f32, height: f32, data: &LobbyData) -> Node {
    col(
        props!(width: width, height: height, background: BG),
        [
            topbar(
                vec![chip("Lobby", CARD, TEXT)],
                &data.status_line,
                data.show_reset
                    .then(|| button!(RESET_KEY, "Reset", style: Ghost, size: Small)),
            ),
            body(
                28.0,
                28.0,
                row(
                    props!(flex: 1.0, gap: 40.0),
                    [qr_panel(&data.status), middle(data), side(data)],
                ),
            ),
        ],
    )
}

fn qr_panel(status: &LobbyStatus) -> Node {
    let inner = match status {
        LobbyStatus::Ready { url } => vec![
            canvas(
                props!(width: QR_SIZE, height: QR_SIZE),
                [Draw::qr(0.0, 0.0, QR_SIZE, url, QrStyle::default())],
            ),
            text(
                url,
                style!(size: FONT_BODY, weight: FontWeight::BOLD, color: TEXT_ON_LIGHT, align: TextAlign::Center),
            ),
        ],
        LobbyStatus::Loading => vec![text(
            "Loading\u{2026}",
            style!(size: FONT_BODY, weight: FontWeight::BOLD, color: TEXT_ON_LIGHT, align: TextAlign::Center),
        )],
        LobbyStatus::Offline => vec![
            text(
                "Offline",
                style!(size: FONT_BODY, weight: FontWeight::BOLD, color: TEXT_ON_LIGHT, align: TextAlign::Center),
            ),
            text(
                "Connect the Deck to the internet once to download questions.",
                style!(size: FONT_LABEL, color: TEXT_ON_LIGHT, align: TextAlign::Center),
            ),
        ],
    };
    col(
        props!(
            width: QR_PANEL_W,
            padding: 18.0,
            gap: 14.0,
            border_radius: 20.0,
            background: CARD_LIGHT,
            cross_align: CrossAlign::Center,
            justify_content: Justify::Center,
        ),
        inner,
    )
}

fn middle(data: &LobbyData) -> Node {
    let mut avatars: Vec<Node> = data
        .players
        .iter()
        .map(|p| {
            avatar(
                &p.name,
                p.color,
                AVATAR,
                data.show_names.then_some(FONT_LABEL),
                p.is_host.then_some("Host"),
            )
        })
        .collect();
    if data.players.len() < data.max_players {
        avatars.push(waiting_slot());
    }
    let hint = if data.ssid.is_empty() {
        String::from("No app, no account. Your phone becomes the buzzer, this screen is the board.")
    } else {
        let ssid = &data.ssid;
        fmt!(
            "Join the Wi-Fi \u{201c}{ssid}\u{201d}, then scan. Your phone becomes the buzzer, this screen is the board."
        )
    };
    col(
        props!(flex: 1.0, gap: 22.0, justify_content: Justify::Center),
        [
            col(
                props!(gap: 0.0),
                [
                    text(
                        "Scan to join",
                        style!(size: FONT_HEADLINE, weight: FontWeight::BOLD, color: TEXT, line_height: 1.1),
                    ),
                    text(
                        "from your phone",
                        style!(size: FONT_HEADLINE, weight: FontWeight::BOLD, color: ACCENT, line_height: 1.1),
                    ),
                ],
            ),
            text(hint, style!(size: FONT_BODY, color: MUTED, max_width: 520)),
            row(
                props!(gap: 26.0, wrap: true, cross_align: CrossAlign::Start),
                avatars,
            ),
        ],
    )
}

fn waiting_slot() -> Node {
    col(
        props!(gap: 8.0, cross_align: CrossAlign::Center),
        [
            canvas(
                props!(width: AVATAR, height: AVATAR),
                [Draw::circle(
                    AVATAR / 2.0,
                    AVATAR / 2.0,
                    AVATAR / 2.0,
                    FAINT,
                )],
            ),
            text("waiting\u{2026}", style!(size: FONT_LABEL, color: FAINT)),
        ],
    )
}

fn side(data: &LobbyData) -> Node {
    let mut count = String::new();
    crate::model::push_usize(&mut count, data.players.len());
    let mut of_max = String::from("of ");
    crate::model::push_usize(&mut of_max, data.max_players);

    let host_line = match data.players.iter().find(|p| p.is_host) {
        Some(host) => {
            let name = &host.name;
            fmt!("{name} taps Start on their phone")
        }
        None => String::from("The first player to join hosts"),
    };

    let mut cards = vec![
        stat_card(
            "PLAYERS",
            vec![row(
                props!(gap: 8.0, cross_align: CrossAlign::End),
                [
                    text(
                        count,
                        style!(size: FONT_STAT, weight: FontWeight::BOLD, color: TEXT, line_height: 1.0),
                    ),
                    text(of_max, style!(size: FONT_BODY, color: MUTED)),
                ],
            )],
        ),
        stat_card(
            "HOST",
            vec![text(host_line, style!(size: FONT_LABEL, color: TEXT))],
        ),
    ];
    if let Some(best) = &data.best_line {
        cards.push(stat_card(
            "BEST EVER",
            vec![text(best, style!(size: FONT_LABEL, color: TEXT))],
        ));
    }
    col(
        props!(width: SIDE_W, gap: 12.0, justify_content: Justify::Center, border_radius: RADIUS_CARD),
        cards,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(players: usize) -> LobbyData {
        LobbyData {
            status: LobbyStatus::Ready {
                url: String::from("http://192.168.1.42:8080/"),
            },
            ssid: String::from("Home"),
            players: (0..players)
                .map(|i| LobbyPlayer {
                    name: String::from("Player"),
                    color: RED_50,
                    is_host: i == 0,
                })
                .collect(),
            max_players: 12,
            show_names: true,
            status_line: String::from("10 questions \u{b7} Mixed"),
            best_line: None,
            show_reset: true,
        }
    }

    #[test]
    fn the_lobby_builds_for_every_status_and_a_full_table() {
        // The tree is serialized on the host; building it must not panic for
        // any status or player count the model allows.
        let _ = lobby_view(1_280.0, 480.0, &data(0));
        let _ = lobby_view(1_280.0, 480.0, &data(12));
        let mut loading = data(3);
        loading.status = LobbyStatus::Loading;
        let _ = lobby_view(1_280.0, 480.0, &loading);
        let mut offline = data(3);
        offline.status = LobbyStatus::Offline;
        offline.ssid.clear();
        let _ = lobby_view(1_280.0, 480.0, &offline);
    }
}
