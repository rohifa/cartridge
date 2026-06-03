use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::models::Platform;
use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

#[tauri::command]
pub fn launch_game(state: Db<'_>, app: AppHandle, game_id: u64) -> Result<(), String> {
    let store = state.lock().unwrap();
    let game = store
        .data
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("game {game_id} not found"))?;

    let url = if let Some(app_id) = &game.app_id {
        match game.platform {
            Platform::Steam => format!("steam://rungameid/{app_id}"),
            Platform::Gog => format!("goggalaxy://openGame/{app_id}"),
            Platform::Epic => {
                format!("com.epicgames.launcher://apps/{app_id}?action=launch&silent=true")
            }
        }
    } else if let Some(store_url) = game.links.get("store") {
        store_url.clone()
    } else {
        return Err("no app_id or store link for this game".into());
    };

    drop(store);
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
