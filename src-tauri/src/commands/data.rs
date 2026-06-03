use std::sync::Mutex;
use tauri::State;

use crate::models::{CartridgeData, Game, GamePatch, Prefs};
use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

#[tauri::command]
pub fn load_data(state: Db<'_>) -> CartridgeData {
    state.lock().unwrap().sanitized_data()
}

#[tauri::command]
pub fn patch_game(state: Db<'_>, id: u64, patch: GamePatch) -> Result<Game, String> {
    state
        .lock()
        .unwrap()
        .patch_game(id, patch)
        .ok_or_else(|| format!("game {id} not found"))
}

#[tauri::command]
pub fn save_prefs(state: Db<'_>, prefs: Prefs) -> Result<(), String> {
    let mut store = state.lock().unwrap();
    store.data.prefs = prefs;
    store.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_json(state: Db<'_>) -> Result<String, String> {
    let data = state.lock().unwrap().sanitized_data();
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}
