use std::path::Path;
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

/// Writes the library (without credentials) to `path`, chosen by the user in a
/// save dialog. Returns the number of games exported.
#[tauri::command]
pub fn export_json_to(state: Db<'_>, path: String) -> Result<u32, String> {
    let data = state.lock().unwrap().sanitized_data();
    write_export(&data, Path::new(&path)).map_err(|e| format!("Could not export: {e}"))
}

/// Pretty-prints `data` to `path` through a temp file in the same directory,
/// so a failed write never leaves a half-written export behind.
fn write_export(data: &CartridgeData, path: &Path) -> std::io::Result<u32> {
    let json = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("no file name"))?
        .to_string_lossy();
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&tmp, json)
        .and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            std::fs::remove_file(&tmp).ok();
        })?;
    Ok(data.games.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Account, AccountStatus, Platform};
    use std::path::PathBuf;

    const SECRETS: [&str; 4] = [
        "secret-api-key",
        "secret-access-token",
        "secret-refresh-token",
        "secret-egs-session",
    ];

    fn store_with_secrets(path: PathBuf) -> AppStore {
        let account = Account {
            id: "76561198000000000".into(),
            name: "Steam".into(),
            handle: "someone".into(),
            platform: Platform::Steam,
            count: 0,
            synced: "just now".into(),
            status: AccountStatus::Ok,
            api_key: Some(SECRETS[0].into()),
            access_token: Some(SECRETS[1].into()),
            refresh_token: Some(SECRETS[2].into()),
            expires_at: Some(0),
            egs_session: Some(SECRETS[3].into()),
        };
        AppStore {
            data: CartridgeData {
                version: 1,
                accounts: vec![account],
                games: vec![],
                lists: vec![],
                genres: vec![],
                statuses: vec![],
                prefs: Prefs::default(),
            },
            covers_dir: path.with_file_name("covers"),
            path,
        }
    }

    #[test]
    fn export_contains_no_credentials() {
        let dir = std::env::temp_dir().join(format!("cartridge-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("cartridge-export.json");
        let store = store_with_secrets(dir.join("library.json"));

        let count = write_export(&store.sanitized_data(), &out).unwrap();

        let json = std::fs::read_to_string(&out).unwrap();
        assert_eq!(count, 0);
        for secret in SECRETS {
            assert!(!json.contains(secret), "export leaks {secret}");
        }
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let account = &value["accounts"][0];
        assert_eq!(account["handle"], "someone");
        for field in ["apiKey", "accessToken", "refreshToken", "egsSession"] {
            assert!(account[field].is_null(), "{field} is not stripped");
        }
        // Only the export itself is left; the temp file was renamed away.
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
