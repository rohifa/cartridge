use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Steam,
    Gog,
    Epic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GameStatus {
    Backlog,
    Playing,
    Beaten,
    Dropped,
    Wishlist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ListColor {
    Green,
    Purple,
    Amber,
    Teal,
    Blue,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ListKind {
    Smart,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    Ok,
    Stale,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub k: String,
    pub v: String,
}

/// A Steam app found by title search for a game owned elsewhere.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SteamMatch {
    pub appid: u64,
    /// Steam's name for it, shown so a wrong match is easy to spot.
    pub name: String,
}

/// Platform-specific copy of a game (one game can have multiple store copies).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCopy {
    pub platform: Platform,
    pub app_id: String,
    #[serde(default)]
    pub account_id: Option<String>,
    pub installed: bool,
    pub playtime: Option<f64>,
    pub last: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: u64,
    pub title: String,
    pub cover: Option<String>,

    // User-owned, game-level data
    pub rating: u8,
    pub status: GameStatus,
    pub tags: Vec<String>,
    pub note: String,

    // Store metadata (same across platforms)
    pub genre: Vec<String>,
    pub dev: String,
    pub year: u32,
    pub metacritic: Option<i32>,
    /// Unix time of the last Metacritic check via Steam (None = never).
    #[serde(default)]
    pub metacritic_checked_at: Option<i64>,
    pub opencritic: Option<i32>,
    pub user_score: Option<i32>,
    /// Number of Steam reviews behind `user_score` (None = never fetched).
    #[serde(default)]
    pub user_score_reviews: Option<u32>,
    /// Steam app matched by title, for games with no Steam copy: their Steam
    /// scores come from here. Not an owned copy — never launched or synced.
    #[serde(default)]
    pub steam_match: Option<SteamMatch>,
    /// Unix time of the last title search (None = never), so titles with no
    /// match aren't searched again on every run.
    #[serde(default)]
    pub steam_match_checked_at: Option<i64>,
    /// The user decided this game's Steam match — set by link, or rejected
    /// with "Not this game" (match None). Automatic matching leaves it alone.
    #[serde(default)]
    pub steam_match_manual: bool,
    /// Matcher revision of the last automatic search (see `scores::MATCHER_REV`).
    #[serde(default)]
    pub steam_match_rev: u32,
    pub links: HashMap<String, String>,
    pub custom: Vec<CustomField>,

    // Platform-specific copies
    pub copies: Vec<GameCopy>,
}

/// Partial update — only `Some` fields are applied.
/// For nullable Game fields, use `Some(None)` to clear or `None` to leave unchanged.
/// Note: copies cannot be patched via this struct; use dedicated commands for copy management.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GamePatch {
    pub title: Option<String>,
    pub cover: Option<Option<String>>,
    pub genre: Option<Vec<String>>,
    pub dev: Option<String>,
    pub year: Option<u32>,
    pub rating: Option<u8>,
    pub status: Option<GameStatus>,
    pub metacritic: Option<Option<i32>>,
    pub opencritic: Option<Option<i32>>,
    pub user_score: Option<Option<i32>>,
    pub tags: Option<Vec<String>>,
    pub note: Option<String>,
    pub links: Option<HashMap<String, String>>,
    pub custom: Option<Vec<CustomField>>,
}

/// Stored in library.json and returned to the frontend.
/// Credential fields (`api_key`, `access_token`, `refresh_token`) are stripped by
/// `AppStore::sanitized_data()` before any data is returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub handle: String,
    pub platform: Platform,
    pub count: u32,
    pub synced: String,
    pub status: AccountStatus,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Unix timestamp when the access token expires.
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// Serialized `egs_api::UserData` JSON for Epic Games accounts.
    #[serde(default)]
    pub egs_session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Filters {
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<Platform>,
    #[serde(default)]
    pub statuses: Vec<GameStatus>,
    #[serde(default)]
    pub year_lo: i32,
    #[serde(default)]
    pub year_hi: i32,
    #[serde(default)]
    pub mc_min: i32,
    #[serde(default)]
    pub oc_min: i32,
    #[serde(default)]
    pub usr_min: i32,
    pub playtime_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameList {
    pub id: String,
    pub name: String,
    pub color: ListColor,
    pub kind: ListKind,
    pub icon: String,
    #[serde(default)]
    pub rules: Option<Filters>,
    #[serde(default)]
    pub game_ids: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub default_view: String,
    pub show_scores: bool,
    pub confirm_launch: bool,
    pub auto_sync: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            default_view: "grid".into(),
            show_scores: true,
            confirm_launch: false,
            auto_sync: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMeta {
    pub id: GameStatus,
    pub label: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartridgeData {
    #[serde(default)]
    pub version: u32,
    pub accounts: Vec<Account>,
    pub games: Vec<Game>,
    pub lists: Vec<GameList>,
    pub genres: Vec<String>,
    pub statuses: Vec<StatusMeta>,
    #[serde(default)]
    pub prefs: Prefs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: u32,
    pub updated: u32,
    pub errors: Vec<String>,
}
