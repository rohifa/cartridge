use std::path::PathBuf;

use anyhow::Context;
use tauri::{AppHandle, Manager};

use crate::models::{CartridgeData, Game, GamePatch, GameStatus, Prefs, StatusMeta};

pub struct AppStore {
    pub data: CartridgeData,
    pub path: PathBuf,
    pub covers_dir: PathBuf,
}

impl AppStore {
    pub fn load(app: &AppHandle) -> Self {
        let base = dirs::data_dir()
            .unwrap_or_else(|| app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let app_dir = base.join("com.cartridge.app");
        std::fs::create_dir_all(&app_dir).ok();

        let path = app_dir.join("library.json");
        let covers_dir = app_dir.join("covers");
        std::fs::create_dir_all(&covers_dir).ok();

        let (mut data, first_run) = if path.exists() {
            let json_str = std::fs::read_to_string(&path).ok();
            let data = json_str
                .and_then(|s| {
                    // Try to parse as JSON Value first to check version
                    let value: serde_json::Value = serde_json::from_str(&s).ok()?;
                    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    
                    if version == 0 {
                        // Old format - need migration
                        eprintln!("[Store] Detected v0 library.json, migrating to v1...");
                        Self::migrate_v0_to_v1(value).ok()
                    } else {
                        // Current format
                        serde_json::from_value(value).ok()
                    }
                })
                .unwrap_or_else(Self::empty_data);
            (data, false)
        } else {
            (Self::empty_data(), true)
        };

        // Ensure version is set
        if data.version == 0 {
            data.version = 1;
        }

        let store = Self { data, path, covers_dir };

        // Persist immediately so subsequent boots don't re-seed
        if first_run {
            store.save().ok();
        }

        store
    }

    /// Migrate v0 (old single-platform format) to v1 (multi-platform copies format)
    fn migrate_v0_to_v1(mut value: serde_json::Value) -> anyhow::Result<CartridgeData> {
        // Extract games array
        let games_array = value.get_mut("games")
            .and_then(|v| v.as_array_mut())
            .context("games field missing or not an array")?;

        // Transform each game
        for game_value in games_array.iter_mut() {
            if let Some(game_obj) = game_value.as_object_mut() {
                // Extract old platform-specific fields
                let platform = game_obj.remove("platform");
                let app_id = game_obj.remove("appId");
                let installed = game_obj.remove("installed").and_then(|v| v.as_bool()).unwrap_or(false);
                let playtime = game_obj.remove("playtime");
                let last = game_obj.remove("last");

                // Create a single copy if we have platform info
                if let (Some(platform), Some(app_id)) = (platform, app_id) {
                    let copy = serde_json::json!({
                        "platform": platform,
                        "appId": app_id.as_str().unwrap_or(""),
                        "accountId": null,
                        "installed": installed,
                        "playtime": playtime,
                        "last": last,
                    });
                    game_obj.insert("copies".to_string(), serde_json::json!([copy]));
                } else {
                    // No platform info - create empty copies array
                    game_obj.insert("copies".to_string(), serde_json::json!([]));
                }
            }
        }

        // Set version to 1
        if let Some(obj) = value.as_object_mut() {
            obj.insert("version".to_string(), serde_json::json!(1));
        }

        // Deserialize the transformed value
        serde_json::from_value(value).context("failed to deserialize migrated data")
    }

    fn empty_data() -> CartridgeData {
        CartridgeData {
            version: 1,
            accounts: vec![],
            games: vec![],
            lists: vec![],
            genres: vec![
                "Action".into(), "Adventure".into(), "Art".into(), "Card".into(),
                "Co-op".into(), "Cozy".into(), "Horror".into(), "Metroidvania".into(),
                "Mystery".into(), "Narrative".into(), "Open world".into(), "Platformer".into(),
                "Puzzle".into(), "Roguelike".into(), "RPG".into(), "Simulation".into(),
            ],
            statuses: vec![
                StatusMeta { id: GameStatus::Backlog,  label: "Backlog".into(),  icon: "layers".into() },
                StatusMeta { id: GameStatus::Playing,  label: "Playing".into(),  icon: "play".into() },
                StatusMeta { id: GameStatus::Beaten,   label: "Beaten".into(),   icon: "check-check".into() },
                StatusMeta { id: GameStatus::Dropped,  label: "Dropped".into(),  icon: "x".into() },
                StatusMeta { id: GameStatus::Wishlist, label: "Wishlist".into(), icon: "bookmark".into() },
            ],
            prefs: Prefs::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&self.data).context("serialise CartridgeData")?;
        std::fs::write(&tmp, json).context("write library.json.tmp")?;
        std::fs::rename(&tmp, &self.path).context("rename tmp → library.json")?;
        Ok(())
    }

    pub fn patch_game(&mut self, id: u64, patch: GamePatch) -> Option<Game> {
        let game = self.data.games.iter_mut().find(|g| g.id == id)?;

        if let Some(v) = patch.title      { game.title     = v; }
        if let Some(v) = patch.cover      { game.cover     = v; }
        if let Some(v) = patch.genre      { game.genre     = v; }
        if let Some(v) = patch.dev        { game.dev       = v; }
        if let Some(v) = patch.year       { game.year      = v; }
        if let Some(v) = patch.rating     { game.rating    = v; }
        if let Some(v) = patch.status     { game.status    = v; }
        if let Some(v) = patch.metacritic { game.metacritic  = v; }
        if let Some(v) = patch.opencritic { game.opencritic  = v; }
        if let Some(v) = patch.user_score { game.user_score  = v; }
        if let Some(v) = patch.tags       { game.tags   = v; }
        if let Some(v) = patch.note       { game.note   = v; }
        if let Some(v) = patch.links      { game.links  = v; }
        if let Some(v) = patch.custom     { game.custom = v; }

        let updated = game.clone();
        self.save().ok();
        Some(updated)
    }

    /// Returns a clone of CartridgeData with `api_key` stripped from all accounts.
    /// Always use this when returning data to the frontend.
    pub fn sanitized_data(&self) -> CartridgeData {
        let mut data = self.data.clone();
        for account in &mut data.accounts {
            account.api_key       = None;
            account.access_token  = None;
            account.refresh_token = None;
            account.egs_session   = None;
        }
        data
    }
}
