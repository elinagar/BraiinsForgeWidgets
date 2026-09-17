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

//! Question packs: the bundled starter pack's line format, filtering by the
//! widget params, and a small deterministic shuffle.
//!
//! The starter pack is tab-separated text rather than JSON so it parses in
//! pure Rust on any target and costs no host call. Nexus packs arrive as JSON
//! and are decoded by the host parser in the wasm glue; both end up as
//! [`Question`]s.

use crate::model::{ANSWER_COUNT, Question, QuestionDifficulty};

/// The starter pack, one question per line:
/// `id<TAB>category<TAB>difficulty<TAB>question<TAB>a<TAB>b<TAB>c<TAB>d<TAB>correct<TAB>why`
/// where `correct` is `0..=3` and `why` may be empty. Lines starting with `#`
/// and blank lines are skipped.
pub const STARTER_PACK: &str = include_str!("../assets/questions-starter.tsv");

/// Which pack a question belongs to, as the manifest names them.
#[must_use]
pub fn pack_of(question: &Question) -> &str {
    question.id.split_once('-').map_or("misc", |(pack, _)| pack)
}

/// Parse a tab-separated pack. Malformed lines are dropped, not fatal: a
/// single bad line in a 500-line pack must not blank the game.
#[must_use]
pub fn parse_tsv(text: &str) -> Vec<Question> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(parse_line)
        .collect()
}

fn parse_line(line: &str) -> Option<Question> {
    let mut fields = line.split('\t');
    let id = fields.next()?.trim();
    let category = fields.next()?.trim();
    let difficulty = QuestionDifficulty::from_wire(fields.next()?.trim())?;
    let text = fields.next()?.trim();
    let mut answers: [String; ANSWER_COUNT] = Default::default();
    for slot in &mut answers {
        let answer = fields.next()?.trim();
        if answer.is_empty() {
            return None;
        }
        answer.clone_into(slot);
    }
    let correct: usize = fields.next()?.trim().parse().ok()?;
    if correct >= ANSWER_COUNT || id.is_empty() || text.is_empty() {
        return None;
    }
    let why = fields
        .next()
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned);
    Some(Question {
        id: id.to_owned(),
        category: category.to_owned(),
        difficulty,
        text: text.to_owned(),
        answers,
        correct,
        why,
    })
}

/// Keep the questions a game may ask: the chosen pack (or all for `mixed`),
/// the chosen difficulty (or all), minus the IDs already played.
#[must_use]
pub fn select(
    questions: Vec<Question>,
    pack: Option<&str>,
    difficulty: Option<QuestionDifficulty>,
    played: &[String],
) -> Vec<Question> {
    questions
        .into_iter()
        .filter(|q| pack.is_none_or(|p| pack_of(q) == p))
        .filter(|q| difficulty.is_none_or(|d| q.difficulty == d))
        .filter(|q| !played.contains(&q.id))
        .collect()
}

/// A tiny xorshift generator so the shuffle is deterministic under test and
/// needs one host random for its seed on the device.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // Zero is a fixed point of xorshift; nudge it.
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform-ish index below `n`; `n` must be non-zero.
    pub fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next_u64() % u64::try_from(n.max(1)).unwrap_or(1)).unwrap_or(0)
    }
}

/// Fisher–Yates in place.
pub fn shuffle<T>(items: &mut [T], rng: &mut Rng) {
    for i in (1..items.len()).rev() {
        let j = rng.below(i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# comment line\n\
science-1\tScience\teasy\tWhat is H2O?\tWater\tSalt\tSugar\tOil\t0\tTwo hydrogens, one oxygen.\n\
\n\
history-1\tHistory\thard\tWhen?\t1\t2\t3\t4\t3\t\n\
bad-1\tBroken\teasy\tMissing answers\tA\tB\n\
bad-2\tBroken\tmedium\tBad index\tA\tB\tC\tD\t7\tx\n\
bad-3\tBroken\tlegendary\tBad difficulty\tA\tB\tC\tD\t0\tx\n";

    #[test]
    fn good_lines_parse_and_bad_lines_are_dropped() {
        let qs = parse_tsv(SAMPLE);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].id, "science-1");
        assert_eq!(qs[0].answers[0], "Water");
        assert_eq!(qs[0].correct, 0);
        assert_eq!(qs[0].why.as_deref(), Some("Two hydrogens, one oxygen."));
        assert_eq!(qs[1].difficulty, QuestionDifficulty::Hard);
        assert_eq!(qs[1].why, None, "an empty why is None, not an empty string");
        assert_eq!(pack_of(&qs[1]), "history");
    }

    #[test]
    fn the_bundled_starter_pack_is_well_formed() {
        let qs = parse_tsv(STARTER_PACK);
        let lines = STARTER_PACK
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .count();
        assert_eq!(qs.len(), lines, "every non-comment line must parse");
        assert!(
            qs.len() >= 100,
            "the starter pack is meant to carry a whole evening"
        );
        let mut ids: Vec<&str> = qs.iter().map(|q| q.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), qs.len(), "question IDs are unique");
        for q in &qs {
            let mut answers: Vec<&str> = q.answers.iter().map(String::as_str).collect();
            answers.sort_unstable();
            answers.dedup();
            assert_eq!(
                answers.len(),
                ANSWER_COUNT,
                "answers are distinct: {}",
                q.id
            );
            assert!(
                q.text.chars().count() <= 120,
                "question fits the board: {}",
                q.id
            );
            for a in &q.answers {
                assert!(a.chars().count() <= 32, "answer fits a card: {}", q.id);
            }
        }
        let packs = [
            "science",
            "geography",
            "history",
            "entertainment",
            "sport",
            "food",
            "animals",
            "tech",
            "bitcoin",
            "kids",
        ];
        for pack in packs {
            assert!(
                qs.iter().filter(|q| pack_of(q) == pack).count() >= 10,
                "pack {pack} has at least ten questions"
            );
        }
    }

    #[test]
    fn select_filters_by_pack_difficulty_and_history() {
        let qs = parse_tsv(STARTER_PACK);
        let all = qs.len();
        assert_eq!(select(qs.clone(), None, None, &[]).len(), all);
        let science = select(qs.clone(), Some("science"), None, &[]);
        assert!(science.iter().all(|q| pack_of(q) == "science"));
        let hard = select(qs.clone(), None, Some(QuestionDifficulty::Hard), &[]);
        assert!(
            hard.iter()
                .all(|q| q.difficulty == QuestionDifficulty::Hard)
        );
        let played = vec![qs[0].id.clone(), qs[1].id.clone()];
        assert_eq!(select(qs, None, None, &played).len(), all - 2);
    }

    #[test]
    fn shuffle_is_a_permutation_and_seed_dependent() {
        let mut a: Vec<usize> = (0..50).collect();
        let mut b = a.clone();
        shuffle(&mut a, &mut Rng::new(1));
        shuffle(&mut b, &mut Rng::new(2));
        assert_ne!(a, b);
        let mut sorted = a.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
        let mut c: Vec<usize> = (0..50).collect();
        shuffle(&mut c, &mut Rng::new(1));
        assert_eq!(a, c, "same seed, same order");
        let mut empty: Vec<usize> = Vec::new();
        shuffle(&mut empty, &mut Rng::new(0));
    }
}
