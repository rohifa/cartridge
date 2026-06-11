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

    // Pick best copy: installed > most recently played > first available
    let copy = game.copies.iter()
        .max_by(|a, b| {
            // First, prefer installed
            match (a.installed, b.installed) {
                (true, false) => return std::cmp::Ordering::Greater,
                (false, true) => return std::cmp::Ordering::Less,
                _ => {}
            }
            // Then prefer most recent
            match (&a.last, &b.last) {
                (Some(a_last), Some(b_last)) => a_last.cmp(b_last),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            }
        })
        .ok_or_else(|| format!("game {game_id} has no platform copies"))?;

    let url = match copy.platform {
        Platform::Steam => format!("steam://rungameid/{}", copy.app_id),
        Platform::Gog => format!("goggalaxy://openGame/{}", copy.app_id),
        Platform::Epic => {
            format!("com.epicgames.launcher://apps/{}?action=launch&silent=true", copy.app_id)
        }
    };

    drop(store);
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
