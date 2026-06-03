use std::sync::Mutex;
use tauri::State;

use crate::models::{Account, AccountStatus, Game, GameStatus, Platform, SyncResult};
use crate::store::AppStore;

type Db<'a> = State<'a, Mutex<AppStore>>;

// ----- Steam API response shapes -----

#[derive(serde::Deserialize)]
struct PlayerSummariesResponse {
    response: PlayerSummariesInner,
}
#[derive(serde::Deserialize)]
struct PlayerSummariesInner {
    players: Vec<PlayerSummary>,
}
#[derive(serde::Deserialize)]
struct PlayerSummary {
    personaname: String,
}

#[derive(serde::Deserialize)]
struct OwnedGamesResponse {
    response: OwnedGamesInner,
}
#[derive(serde::Deserialize)]
struct OwnedGamesInner {
    #[serde(default)]
    games: Vec<OwnedGame>,
}
#[derive(serde::Deserialize)]
struct OwnedGame {
    appid: u64,
    name: String,
    playtime_forever: Option<u64>,  // minutes
    rtime_last_played: Option<u64>, // unix timestamp
}

// ----- Helpers -----

fn relative_time(unix: u64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(unix as i64, 0)
        .unwrap_or_else(Utc::now);
    let days = Utc::now().signed_duration_since(dt).num_days();
    match days {
        0 => "today".into(),
        1 => "yesterday".into(),
        2..=6 => format!("{days} days ago"),
        7..=27 => format!("{} weeks ago", days / 7),
        28..=364 => format!("{} months ago", days / 30),
        _ => format!("{} years ago", days / 365),
    }
}

async fn download_cover(
    client: &reqwest::Client,
    url: &str,
    dest: &std::path::Path,
) -> anyhow::Result<()> {
    let bytes = client.get(url).send().await?.error_for_status()?.bytes().await?;
    std::fs::write(dest, &bytes)?;
    Ok(())
}

// ----- Commands -----

#[tauri::command]
pub async fn add_steam_account(
    state: Db<'_>,
    api_key: String,
    steam_id: String,
) -> Result<Account, String> {
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/\
         ?key={api_key}&steamids={steam_id}"
    );
    let resp: PlayerSummariesResponse = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let player = resp
        .response
        .players
        .into_iter()
        .next()
        .ok_or("Steam account not found — check your Steam ID")?;

    let account = Account {
        id: steam_id.clone(),
        name: "Steam".into(),
        handle: player.personaname,
        platform: Platform::Steam,
        count: 0,
        synced: "never".into(),
        status: AccountStatus::Ok,
        api_key: Some(api_key),
    };

    let mut store = state.lock().unwrap();
    match store.data.accounts.iter_mut().find(|a| a.id == steam_id) {
        Some(existing) => *existing = account.clone(),
        None => store.data.accounts.push(account.clone()),
    }
    store.save().map_err(|e| e.to_string())?;

    Ok(Account { api_key: None, ..account })
}

#[tauri::command]
pub async fn sync_account(state: Db<'_>, account_id: String) -> Result<SyncResult, String> {
    let (platform, api_key) = {
        let store = state.lock().unwrap();
        let account = store
            .data
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| format!("account {account_id} not found"))?;
        (account.platform.clone(), account.api_key.clone())
    };

    match platform {
        Platform::Steam => sync_steam(state, api_key, account_id).await,
        _ => Ok(SyncResult {
            added: 0,
            updated: 0,
            errors: vec!["Not implemented".into()],
        }),
    }
}

async fn sync_steam(
    state: Db<'_>,
    api_key: Option<String>,
    account_id: String,
) -> Result<SyncResult, String> {
    let api_key = api_key.ok_or("no Steam API key stored for this account")?;

    // Fetch owned games (no lock held across .await)
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/\
         ?key={api_key}&steamid={account_id}&include_appinfo=1\
         &include_played_free_games=1&format=json"
    );
    let resp: OwnedGamesResponse = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let steam_games = resp.response.games;

    // Merge into the store (sync, under lock)
    let (mut added, mut updated) = (0u32, 0u32);
    let covers_dir = {
        let mut store = state.lock().unwrap();

        for sg in &steam_games {
            let app_id_str = sg.appid.to_string();
            let playtime = sg.playtime_forever.map(|m| m as f64 / 60.0);
            let last = sg.rtime_last_played.filter(|&t| t > 0).map(relative_time);

            if let Some(game) = store
                .data
                .games
                .iter_mut()
                .find(|g| g.app_id.as_deref() == Some(&app_id_str))
            {
                // Preserve user data — only update sync-owned fields
                if let Some(h) = playtime {
                    game.playtime = Some(h);
                }
                if let Some(l) = last {
                    game.last = Some(l);
                }
                game.installed = playtime.map(|h| h > 0.0).unwrap_or(false);
                updated += 1;
            } else {
                let next_id = store.data.games.iter().map(|g| g.id).max().unwrap_or(0) + 1;
                store.data.games.push(Game {
                    id: next_id,
                    title: sg.name.clone(),
                    cover: None,
                    platform: Platform::Steam,
                    genre: vec![],
                    dev: String::new(),
                    year: 0,
                    playtime,
                    last,
                    rating: 0,
                    status: GameStatus::Backlog,
                    installed: playtime.map(|h| h > 0.0).unwrap_or(false),
                    metacritic: None,
                    opencritic: None,
                    user_score: None,
                    tags: vec![],
                    note: String::new(),
                    links: Default::default(),
                    custom: vec![],
                    app_id: Some(app_id_str),
                });
                added += 1;
            }
        }

        let now = chrono::Utc::now().format("%b %d, %Y").to_string();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.synced = now;
            acc.status = AccountStatus::Ok;
            acc.count = steam_games.len() as u32;
        }
        store.save().map_err(|e| e.to_string())?;

        store.covers_dir.clone()
    }; // lock released here

    // Download cover art (async, no lock held)
    let mut errors: Vec<String> = Vec::new();
    let client = reqwest::Client::new();

    for sg in &steam_games {
        let cover_path = covers_dir.join(format!("{}.jpg", sg.appid));
        if cover_path.exists() {
            continue;
        }
        let img_url = format!(
            "https://cdn.akamai.steamstatic.com/steam/apps/{}/library_600x900_2x.jpg",
            sg.appid
        );
        match download_cover(&client, &img_url, &cover_path).await {
            Ok(()) => {
                let app_id_str = sg.appid.to_string();
                let cover_str = cover_path.to_string_lossy().into_owned();
                let mut store = state.lock().unwrap();
                if let Some(game) = store
                    .data
                    .games
                    .iter_mut()
                    .find(|g| g.app_id.as_deref() == Some(&app_id_str))
                {
                    game.cover = Some(cover_str);
                }
                store.save().ok();
            }
            Err(e) => errors.push(format!("cover {}: {e}", sg.appid)),
        }
    }

    Ok(SyncResult { added, updated, errors })
}

#[tauri::command]
pub fn remove_account(state: Db<'_>, account_id: String) -> Result<(), String> {
    let mut store = state.lock().unwrap();
    store.data.accounts.retain(|a| a.id != account_id);
    store.save().map_err(|e| e.to_string())
}
