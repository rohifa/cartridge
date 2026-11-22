use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

/// Path of `library.json` for display, with the home directory shown as `~`.
#[tauri::command]
pub fn get_library_path(state: Db<'_>) -> String {
    let path = state.lock().unwrap().path.clone();
    abbreviate_home(&path, dirs::home_dir().as_deref())
}

/// Opens Finder with `library.json` selected. Takes no path, so the frontend
/// can only reveal the library file, never an arbitrary location.
#[tauri::command]
pub fn reveal_library(app: AppHandle, state: Db<'_>) -> Result<(), String> {
    let path = state.lock().unwrap().path.clone();
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}

fn abbreviate_home(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.to_string_lossy()),
        None => path.to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_abbreviated() {
        let home = Path::new("/Users/someone");
        let path =
            Path::new("/Users/someone/Library/Application Support/com.cartridge.app/library.json");
        assert_eq!(
            abbreviate_home(path, Some(home)),
            "~/Library/Application Support/com.cartridge.app/library.json"
        );
    }

    #[test]
    fn paths_outside_home_are_unchanged() {
        let home = Path::new("/Users/someone");
        // A sibling whose name merely starts with the home dir's name.
        let path = Path::new("/Users/someone-else/library.json");
        assert_eq!(
            abbreviate_home(path, Some(home)),
            "/Users/someone-else/library.json"
        );
        assert_eq!(
            abbreviate_home(path, None),
            "/Users/someone-else/library.json"
        );
    }
}
