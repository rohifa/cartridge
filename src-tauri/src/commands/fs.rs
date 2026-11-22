use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

#[tauri::command]
pub fn get_library_path(state: Db<'_>) -> String {
    state.lock().unwrap().path.to_string_lossy().into_owned()
}

#[tauri::command]
pub fn reveal_in_finder(app: AppHandle, path: String) -> Result<(), String> {
    let parent = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or(path);
    app.opener()
        .open_path(parent, None::<&str>)
        .map_err(|e| e.to_string())
}
