use std::path::PathBuf;

use anyhow::Context;
use tauri::{AppHandle, Manager};

use crate::models::{CartridgeData, Game, GamePatch};

const SEED: &str = include_str!("seed.json");

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

        let data = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(Self::seed_data)
        } else {
            Self::seed_data()
        };

        Self { data, path, covers_dir }
    }

    fn seed_data() -> CartridgeData {
        serde_json::from_str(SEED).expect("seed.json is invalid — this is a compile-time bug")
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

        if let Some(v) = patch.title       { game.title     = v; }
        if let Some(v) = patch.cover       { game.cover     = v; }
        if let Some(v) = patch.platform    { game.platform  = v; }
        if let Some(v) = patch.genre       { game.genre     = v; }
        if let Some(v) = patch.dev         { game.dev       = v; }
        if let Some(v) = patch.year        { game.year      = v; }
        if let Some(v) = patch.playtime    { game.playtime  = v; }
        if let Some(v) = patch.last        { game.last      = v; }
        if let Some(v) = patch.rating      { game.rating    = v; }
        if let Some(v) = patch.status      { game.status    = v; }
        if let Some(v) = patch.installed   { game.installed = v; }
        if let Some(v) = patch.metacritic  { game.metacritic  = v; }
        if let Some(v) = patch.opencritic  { game.opencritic  = v; }
        if let Some(v) = patch.user_score  { game.user_score  = v; }
        if let Some(v) = patch.tags        { game.tags   = v; }
        if let Some(v) = patch.note        { game.note   = v; }
        if let Some(v) = patch.links       { game.links  = v; }
        if let Some(v) = patch.custom      { game.custom = v; }
        if let Some(v) = patch.app_id      { game.app_id = v; }

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
