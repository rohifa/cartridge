use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

use crate::models::GameList;
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

#[tauri::command]
pub fn add_to_list(state: Db<'_>, list_id: String, game_id: u64) -> Result<(), String> {
    let mut store = state.lock().unwrap();
    let list = store
        .data
        .lists
        .iter_mut()
        .find(|l| l.id == list_id)
        .ok_or_else(|| format!("list {list_id} not found"))?;
    let ids = list.game_ids.get_or_insert_with(Vec::new);
    if !ids.contains(&game_id) {
        ids.push(game_id);
    }
    store.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_list(state: Db<'_>, list_id: String, game_id: u64) -> Result<(), String> {
    let mut store = state.lock().unwrap();
    let list = store
        .data
        .lists
        .iter_mut()
        .find(|l| l.id == list_id)
        .ok_or_else(|| format!("list {list_id} not found"))?;
    if let Some(ids) = &mut list.game_ids {
        ids.retain(|&id| id != game_id);
    }
    store.save().map_err(|e| e.to_string())
}
