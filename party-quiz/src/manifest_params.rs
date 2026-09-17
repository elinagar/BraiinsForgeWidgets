// AUTO-GENERATED FROM ../manifest.json by `bmc-widget-codegen` v0.1.0.
// Do not edit by hand. Run `just wasm::gen <widget>` after changing the manifest.

#![allow(
    dead_code,
    reason = "fields are widget-specific; not every key is used by every render path"
)]

use bmc_wasm_sdk::params as snapshot;
use bmc_wasm_sdk::params::typed::ParamRead;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryPack {
    Mixed,
    Science,
    Geography,
    History,
    Entertainment,
    Sport,
    Food,
    Animals,
    Tech,
    Bitcoin,
    Kids,
}
impl CategoryPack {
    /// Every variant, in manifest-declaration order. Useful when a widget
    /// wants to render a "pick one" UI or audit the enum exhaustively.
    pub const ALL: &'static [Self] = &[
        Self::Mixed,
        Self::Science,
        Self::Geography,
        Self::History,
        Self::Entertainment,
        Self::Sport,
        Self::Food,
        Self::Animals,
        Self::Tech,
        Self::Bitcoin,
        Self::Kids,
    ];
    /// Manifest wire value for this variant.
    #[must_use]
    pub fn as_manifest_value(self) -> &'static str {
        match self {
            Self::Mixed => "mixed",
            Self::Science => "science",
            Self::Geography => "geography",
            Self::History => "history",
            Self::Entertainment => "entertainment",
            Self::Sport => "sport",
            Self::Food => "food",
            Self::Animals => "animals",
            Self::Tech => "tech",
            Self::Bitcoin => "bitcoin",
            Self::Kids => "kids",
        }
    }
    /// Human-readable label declared in the manifest's `enum_values`.
    #[must_use]
    pub fn as_manifest_label(self) -> &'static str {
        match self {
            Self::Mixed => "Mixed",
            Self::Science => "Science and space",
            Self::Geography => "Geography",
            Self::History => "History",
            Self::Entertainment => "Film, TV and music",
            Self::Sport => "Sport",
            Self::Food => "Food and drink",
            Self::Animals => "Animals and nature",
            Self::Tech => "Technology",
            Self::Bitcoin => "Bitcoin and mining",
            Self::Kids => "Kids",
        }
    }
    #[must_use]
    pub fn from_manifest_value(s: &str) -> Option<Self> {
        match s {
            "mixed" => Some(Self::Mixed),
            "science" => Some(Self::Science),
            "geography" => Some(Self::Geography),
            "history" => Some(Self::History),
            "entertainment" => Some(Self::Entertainment),
            "sport" => Some(Self::Sport),
            "food" => Some(Self::Food),
            "animals" => Some(Self::Animals),
            "tech" => Some(Self::Tech),
            "bitcoin" => Some(Self::Bitcoin),
            "kids" => Some(Self::Kids),
            _ => None,
        }
    }
}
bmc_wasm_sdk::impl_manifest_str_enum!(CategoryPack);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Mixed,
}
impl Difficulty {
    /// Every variant, in manifest-declaration order. Useful when a widget
    /// wants to render a "pick one" UI or audit the enum exhaustively.
    pub const ALL: &'static [Self] = &[Self::Easy, Self::Medium, Self::Hard, Self::Mixed];
    /// Manifest wire value for this variant.
    #[must_use]
    pub fn as_manifest_value(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Mixed => "mixed",
        }
    }
    /// Human-readable label declared in the manifest's `enum_values`.
    #[must_use]
    pub fn as_manifest_label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
            Self::Mixed => "Mixed",
        }
    }
    #[must_use]
    pub fn from_manifest_value(s: &str) -> Option<Self> {
        match s {
            "easy" => Some(Self::Easy),
            "medium" => Some(Self::Medium),
            "hard" => Some(Self::Hard),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}
bmc_wasm_sdk::impl_manifest_str_enum!(Difficulty);
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub answer_time: i32,
    pub category_pack: CategoryPack,
    pub difficulty: Difficulty,
    pub question_count: i32,
    pub show_player_names: bool,
    pub sounds: bool,
}
impl Params {
    /// Materialise a typed snapshot from a dynamic [`snapshot::Params`].
    #[must_use]
    pub fn from_snapshot(snap: &snapshot::Params) -> Self {
        Self {
            answer_time: <i32 as ParamRead>::read_required(snap, "answer_time"),
            category_pack: <CategoryPack as ParamRead>::read_required(snap, "category_pack"),
            difficulty: <Difficulty as ParamRead>::read_required(snap, "difficulty"),
            question_count: <i32 as ParamRead>::read_required(snap, "question_count"),
            show_player_names: <bool as ParamRead>::read_required(snap, "show_player_names"),
            sounds: <bool as ParamRead>::read_required(snap, "sounds"),
        }
    }
    /// Latest typed snapshot delivered for this widget instance.
    /// Cached per-thread; only re-parses when `snapshot::version()` changes
    /// since the last call.
    #[must_use]
    pub fn current() -> Self {
        thread_local! {
            static CACHE : core::cell::RefCell < Option < (u64, Params) >> = const {
            core::cell::RefCell::new(None) };
        }
        let v = snapshot::version();
        CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            if let Some((cv, ref params)) = *cache
                && cv == v
            {
                return params.clone();
            }
            let fresh = Self::from_snapshot(&snapshot::current());
            *cache = Some((v, fresh.clone()));
            fresh
        })
    }
    /// Snapshot delivered immediately before [`current`]; `None` until at
    /// least one update has been observed (i.e. during `init` and the
    /// first `render`).
    #[must_use]
    pub fn previous() -> Option<Self> {
        let prev = snapshot::previous();
        if prev.is_empty() {
            None
        } else {
            Some(Self::from_snapshot(&prev))
        }
    }
    /// Manifest keys whose value differs between `self` and `other`.
    ///
    /// Intended for `on_params_update` diffing — pass `current()` and the
    /// inside-hook value of `previous()` to get the set of keys to react
    /// to. Field-by-field `PartialEq`; emitted in struct-field order so
    /// the result is deterministic.
    #[must_use]
    pub fn changed_keys(&self, other: &Self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.answer_time != other.answer_time {
            out.push("answer_time");
        }
        if self.category_pack != other.category_pack {
            out.push("category_pack");
        }
        if self.difficulty != other.difficulty {
            out.push("difficulty");
        }
        if self.question_count != other.question_count {
            out.push("question_count");
        }
        if self.show_player_names != other.show_player_names {
            out.push("show_player_names");
        }
        if self.sounds != other.sounds {
            out.push("sounds");
        }
        out
    }
}
