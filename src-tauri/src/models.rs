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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: u64,
    pub title: String,
    pub cover: Option<String>,
    pub platform: Platform,
    pub genre: Vec<String>,
    pub dev: String,
    pub year: u32,
    pub playtime: Option<f64>,
    pub last: Option<String>,
    pub rating: u8,
    pub status: GameStatus,
    pub installed: bool,
    pub metacritic: Option<i32>,
    pub opencritic: Option<i32>,
    pub user_score: Option<i32>,
    pub tags: Vec<String>,
    pub note: String,
    pub links: HashMap<String, String>,
    pub custom: Vec<CustomField>,
    #[serde(default)]
    pub app_id: Option<String>,
}

/// Partial update — only `Some` fields are applied. `api_key` is intentionally excluded.
/// For nullable Game fields, use `Some(None)` to clear or `None` to leave unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GamePatch {
    pub title: Option<String>,
    pub cover: Option<Option<String>>,
    pub platform: Option<Platform>,
    pub genre: Option<Vec<String>>,
    pub dev: Option<String>,
    pub year: Option<u32>,
    pub playtime: Option<Option<f64>>,
    pub last: Option<Option<String>>,
    pub rating: Option<u8>,
    pub status: Option<GameStatus>,
    pub installed: Option<bool>,
    pub metacritic: Option<Option<i32>>,
    pub opencritic: Option<Option<i32>>,
    pub user_score: Option<Option<i32>>,
    pub tags: Option<Vec<String>>,
    pub note: Option<String>,
    pub links: Option<HashMap<String, String>>,
    pub custom: Option<Vec<CustomField>>,
    pub app_id: Option<Option<String>>,
}

/// Stored in library.json and returned to the frontend.
/// `api_key` is never serialized in Tauri command responses — use `AppStore::sanitized_data()`.
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
    /// Stored on disk for sync calls; stripped before returning to the frontend.
    #[serde(default)]
    pub api_key: Option<String>,
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
