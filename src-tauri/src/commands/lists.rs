use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

use crate::models::{CartridgeData, GameList, ListKind};
use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

#[tauri::command]
pub fn create_list(state: Db<'_>, mut list: GameList) -> Result<GameList, String> {
    list.id = Uuid::new_v4().to_string();
    let mut store = state.lock().unwrap();
    store.data.lists.push(list.clone());
    store.save().map_err(|e| e.to_string())?;
    Ok(list)
}

#[tauri::command]
pub fn delete_list(state: Db<'_>, id: String) -> Result<(), String> {
    let mut store = state.lock().unwrap();
    store.data.lists.retain(|l| l.id != id);
    store.save().map_err(|e| e.to_string())
}

/// Adds a game to a manual list and returns the updated list.
#[tauri::command]
pub fn add_to_list(state: Db<'_>, list_id: String, game_id: u64) -> Result<GameList, String> {
    let mut store = state.lock().unwrap();
    let list = add_game(&mut store.data, &list_id, game_id)?;
    store.save().map_err(|e| e.to_string())?;
    Ok(list)
}

/// Removes a game from a manual list and returns the updated list.
#[tauri::command]
pub fn remove_from_list(state: Db<'_>, list_id: String, game_id: u64) -> Result<GameList, String> {
    let mut store = state.lock().unwrap();
    let list = remove_game(&mut store.data, &list_id, game_id)?;
    store.save().map_err(|e| e.to_string())?;
    Ok(list)
}

/// The manual list `list_id`. Smart lists fill themselves from their rules,
/// so their membership can't be edited by hand.
fn manual_list<'a>(data: &'a mut CartridgeData, list_id: &str) -> Result<&'a mut GameList, String> {
    let list = data
        .lists
        .iter_mut()
        .find(|l| l.id == list_id)
        .ok_or_else(|| format!("List {list_id} not found."))?;
    if list.kind == ListKind::Smart {
        return Err(format!(
            "\"{}\" is a smart list; it fills itself from its filters.",
            list.name
        ));
    }
    Ok(list)
}

/// Adds `game_id` to a manual list (no-op if already there). Doesn't save.
fn add_game(data: &mut CartridgeData, list_id: &str, game_id: u64) -> Result<GameList, String> {
    if !data.games.iter().any(|g| g.id == game_id) {
        return Err(format!("Game {game_id} not found."));
    }
    let list = manual_list(data, list_id)?;
    let ids = list.game_ids.get_or_insert_with(Vec::new);
    if !ids.contains(&game_id) {
        ids.push(game_id);
    }
    Ok(list.clone())
}

/// Removes `game_id` from a manual list (no-op if absent). Doesn't save.
fn remove_game(data: &mut CartridgeData, list_id: &str, game_id: u64) -> Result<GameList, String> {
    let list = manual_list(data, list_id)?;
    if let Some(ids) = &mut list.game_ids {
        ids.retain(|&id| id != game_id);
    }
    Ok(list.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Games 1 and 2; manual list "m" holding game 1, smart list "s".
    fn data() -> CartridgeData {
        let game = |id: u64| {
            serde_json::json!({
                "id": id, "title": format!("Game {id}"), "cover": null, "genre": [],
                "dev": "", "year": 0, "rating": 0, "status": "backlog",
                "metacritic": null, "opencritic": null, "userScore": null,
                "tags": [], "note": "", "links": {}, "custom": [], "copies": [],
            })
        };
        serde_json::from_value(serde_json::json!({
            "accounts": [], "genres": [], "statuses": [],
            "games": [game(1), game(2)],
            "lists": [
                { "id": "m", "name": "Couch", "color": "green", "kind": "manual",
                  "icon": "list", "gameIds": [1] },
                { "id": "s", "name": "Short", "color": "teal", "kind": "smart",
                  "icon": "zap", "rules": { "genres": [], "platforms": [], "statuses": [],
                  "yearLo": 0, "yearHi": 3000, "mcMin": 0, "ocMin": 0, "usrMin": 0 } },
            ],
        }))
        .unwrap()
    }

    fn ids(d: &CartridgeData, list_id: &str) -> Option<Vec<u64>> {
        d.lists
            .iter()
            .find(|l| l.id == list_id)
            .unwrap()
            .game_ids
            .clone()
    }

    #[test]
    fn add_returns_updated_list_and_ignores_duplicates() {
        let mut d = data();
        let list = add_game(&mut d, "m", 2).unwrap();
        assert_eq!(list.game_ids, Some(vec![1, 2]));
        assert_eq!(ids(&d, "m"), Some(vec![1, 2]));

        let again = add_game(&mut d, "m", 2).unwrap();
        assert_eq!(again.game_ids, Some(vec![1, 2]));
    }

    #[test]
    fn add_creates_ids_when_missing() {
        let mut d = data();
        d.lists[0].game_ids = None;
        let list = add_game(&mut d, "m", 2).unwrap();
        assert_eq!(list.game_ids, Some(vec![2]));
    }

    #[test]
    fn add_to_smart_list_is_an_error() {
        let mut d = data();
        let err = add_game(&mut d, "s", 1).unwrap_err();
        assert!(err.contains("smart list"), "{err}");
        assert_eq!(ids(&d, "s"), None);
    }

    #[test]
    fn add_unknown_list_or_game_is_an_error() {
        let mut d = data();
        assert!(add_game(&mut d, "nope", 1).is_err());
        assert!(add_game(&mut d, "m", 99).is_err());
        assert_eq!(ids(&d, "m"), Some(vec![1]));
    }

    #[test]
    fn remove_returns_updated_list() {
        let mut d = data();
        let list = remove_game(&mut d, "m", 1).unwrap();
        assert_eq!(list.game_ids, Some(vec![]));
        assert_eq!(ids(&d, "m"), Some(vec![]));

        // Removing a game that isn't there is a no-op, not an error.
        let again = remove_game(&mut d, "m", 1).unwrap();
        assert_eq!(again.game_ids, Some(vec![]));
    }

    #[test]
    fn remove_from_smart_list_is_an_error() {
        let mut d = data();
        assert!(remove_game(&mut d, "s", 1).is_err());
    }
}
