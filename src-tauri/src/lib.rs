use std::sync::Mutex;

use tauri::Manager;

mod commands;
mod models;
mod secrets;
mod store;

use store::AppStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let store = AppStore::load(app.handle());
            app.manage(Mutex::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::data::load_data,
            commands::data::patch_game,
            commands::data::save_prefs,
            commands::data::export_json,
            commands::data::export_json_to,
            commands::lists::create_list,
            commands::lists::delete_list,
            commands::lists::add_to_list,
            commands::lists::remove_from_list,
            commands::launch::launch_game,
            commands::launch::open_url,
            commands::sync::add_steam_account,
            commands::sync::start_oauth_flow,
            commands::sync::sync_account,
            commands::sync::remove_account,
            commands::scores::start_score_refresh,
            commands::scores::set_steam_match,
            commands::scores::reject_steam_match,
            commands::scores::auto_steam_match,
            commands::fs::get_library_path,
            commands::fs::reveal_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
