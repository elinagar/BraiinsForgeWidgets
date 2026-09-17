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

//! A desk-side stand-in for the Deck's listener: serves the controller page
//! and the game routes over plain TCP so the phone page can be exercised in
//! a browser without a Deck or the testbed.
//!
//! ```shell
//! cargo run -p party-quiz --example dev-server -- 8080
//! ```
//!
//! Then open `http://localhost:8080/` in two browser tabs. The game state is
//! ticked from wall-clock time between requests, so countdowns run.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::time::Instant;

    use party_quiz::model::{Game, Settings};
    use party_quiz::server::{Request, handle};

    const PAGE: &[u8] = include_bytes!("../assets/controller.html");

    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = TcpListener::bind(("0.0.0.0", port)).expect("bind dev server port");
    eprintln!("party-quiz dev server on http://localhost:{port}/");

    let mut game = Game::new(Settings {
        question_count: 3,
        answer_time_ms: 15_000,
    });
    game.load_questions(sample_questions());
    let mut last_tick = Instant::now();
    let mut next_token = 0x1000_u64;

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            continue;
        }
        let mut parts = request_line.trim().splitn(3, ' ');
        let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (method, path) = (method.to_owned(), path.to_owned());
        let mut content_length = 0_usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0_u8; content_length.min(1 << 16)];
        if !body.is_empty() {
            let _ = reader.read_exact(&mut body);
        }

        let now = Instant::now();
        let delta = u32::try_from(now.duration_since(last_tick).as_millis()).unwrap_or(u32::MAX);
        last_tick = now;
        game.tick(delta);

        let mut mint = || {
            next_token += 1;
            next_token
        };
        let (response, _effect) = handle(
            &mut game,
            &Request {
                method: &method,
                path: &path,
                body: &body,
            },
            PAGE,
            &mut mint,
        );
        let status_text = match response.status {
            204 => "No Content",
            400 => "Bad Request",
            404 => "Not Found",
            _ => "OK",
        };
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n\r\n",
            response.status,
            status_text,
            response.body.len(),
            response.headers
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&response.body);
        let _ = stream.flush();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn sample_questions() -> Vec<party_quiz::model::Question> {
    use party_quiz::model::{Question, QuestionDifficulty};
    let q = |id: &str, text: &str, answers: [&str; 4], correct: usize, why: &str| Question {
        id: id.to_owned(),
        category: "Science".to_owned(),
        difficulty: QuestionDifficulty::Medium,
        text: text.to_owned(),
        answers: answers.map(str::to_owned),
        correct,
        why: Some(why.to_owned()),
    };
    vec![
        q(
            "dev-1",
            "Which planet has the shortest day?",
            ["Mars", "Jupiter", "Venus", "Saturn"],
            1,
            "Jupiter spins once every 9 h 56 min.",
        ),
        q(
            "dev-2",
            "How many bits are in a byte?",
            ["4", "16", "8", "2"],
            2,
            "Eight bits, by convention since the 1960s.",
        ),
        q(
            "dev-3",
            "What year was the Bitcoin genesis block mined?",
            ["2008", "2010", "2011", "2009"],
            3,
            "3 January 2009.",
        ),
    ]
}
