use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::models::{GameCopy, Platform};
use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

/// The copy to launch: installed first, then the most recent session, then the
/// first listed.
fn pick_copy(copies: &[GameCopy]) -> Option<&GameCopy> {
    copies.iter().reduce(|best, c| {
        if (c.installed, c.last_played_at) > (best.installed, best.last_played_at) {
            c
        } else {
            best
        }
    })
}

#[tauri::command]
pub fn launch_game(state: Db<'_>, app: AppHandle, game_id: u64) -> Result<(), String> {
    let store = state.lock().unwrap();
    let game = store
        .data
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("game {game_id} not found"))?;

    let copy =
        pick_copy(&game.copies).ok_or_else(|| format!("game {game_id} has no platform copies"))?;

    let url = match copy.platform {
        Platform::Steam => format!("steam://rungameid/{}", copy.app_id),
        Platform::Gog => format!("goggalaxy://openGame/{}", copy.app_id),
        Platform::Epic => {
            format!(
                "com.epicgames.launcher://apps/{}?action=launch&silent=true",
                copy.app_id
            )
        }
    };

    drop(store);
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(app_id: &str, installed: bool, last_played_at: Option<i64>) -> GameCopy {
        GameCopy {
            platform: Platform::Steam,
            app_id: app_id.into(),
            account_id: None,
            installed,
            playtime: None,
            last_played_at,
        }
    }

    fn pick(copies: &[GameCopy]) -> Option<&str> {
        pick_copy(copies).map(|c| c.app_id.as_str())
    }

    #[test]
    fn prefers_installed_then_recent_then_first() {
        assert_eq!(pick(&[]), None);
        // Installed beats a more recent session.
        assert_eq!(
            pick(&[copy("a", false, Some(900)), copy("b", true, Some(100))]),
            Some("b")
        );
        // Most recent session among installed copies.
        assert_eq!(
            pick(&[copy("a", true, Some(100)), copy("b", true, Some(900))]),
            Some("b")
        );
        // A known session beats none.
        assert_eq!(
            pick(&[copy("a", false, None), copy("b", false, Some(1))]),
            Some("b")
        );
        // Ties keep the first.
        assert_eq!(
            pick(&[copy("a", false, None), copy("b", false, None)]),
            Some("a")
        );
        assert_eq!(
            pick(&[copy("a", true, Some(5)), copy("b", true, Some(5))]),
            Some("a")
        );
    }
}
