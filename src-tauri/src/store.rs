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

        let (data, first_run) = if path.exists() {
            let data = std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(Self::empty_data);
            (data, false)
        } else {
            (Self::empty_data(), true)
        };

        let store = Self { data, path, covers_dir };

        // Persist immediately so subsequent boots don't re-seed
        if first_run {
            store.save().ok();
        }

        store
    }

    fn empty_data() -> CartridgeData {
        CartridgeData {
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
        if let Some(v) = patch.platform   { game.platform  = v; }
        if let Some(v) = patch.genre      { game.genre     = v; }
        if let Some(v) = patch.dev        { game.dev       = v; }
        if let Some(v) = patch.year       { game.year      = v; }
        if let Some(v) = patch.playtime   { game.playtime  = v; }
        if let Some(v) = patch.last       { game.last      = v; }
        if let Some(v) = patch.rating     { game.rating    = v; }
        if let Some(v) = patch.status     { game.status    = v; }
        if let Some(v) = patch.installed  { game.installed = v; }
        if let Some(v) = patch.metacritic { game.metacritic  = v; }
        if let Some(v) = patch.opencritic { game.opencritic  = v; }
        if let Some(v) = patch.user_score { game.user_score  = v; }
        if let Some(v) = patch.tags       { game.tags   = v; }
        if let Some(v) = patch.note       { game.note   = v; }
        if let Some(v) = patch.links      { game.links  = v; }
        if let Some(v) = patch.custom     { game.custom = v; }
        if let Some(v) = patch.app_id     { game.app_id = v; }

        let updated = game.clone();
        self.save().ok();
        Some(updated)
    }

    /// Returns a clone of CartridgeData with `api_key` stripped from all accounts.
    /// Always use this when returning data to the frontend.
    pub fn sanitized_data(&self) -> CartridgeData {
        let mut data = self.data.clone();
        for account in &mut data.accounts {
            account.api_key = None;
        }
        data
    }
}
