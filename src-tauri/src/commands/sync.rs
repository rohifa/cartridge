use std::sync::Mutex;
use tauri::State;

use egs_api::EpicGames;
use egs_api::api::types::account::UserData as EgsUserData;

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

use crate::models::{Account, AccountStatus, Game, GameCopy, GameStatus, Platform, SyncResult};
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

/// Normalize title for fuzzy matching across platforms
fn normalize_title(s: &str) -> String {
    s.to_lowercase()
        .replace("™", "")
        .replace("®", "")
        .replace(":", "")
        .replace("'", "")
        .replace("-", "")
        .replace(" ", "")
        .trim()
        .to_string()
}

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
    // When editing, the caller passes the old ID so it can be removed if it changed.
    prev_id: Option<String>,
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

    let mut store = state.lock().unwrap();

    // Remove the old account if the Steam ID changed during an edit.
    if let Some(ref old_id) = prev_id {
        if old_id != &steam_id {
            store.data.accounts.retain(|a| &a.id != old_id);
        }
    }

    let result = match store.data.accounts.iter_mut().find(|a| a.id == steam_id) {
        Some(existing) => {
            // Update only credential/identity fields; preserve sync history.
            existing.handle  = player.personaname;
            existing.api_key = Some(api_key);
            Account { api_key: None, ..existing.clone() }
        }
        None => {
            let account = Account {
                id:            steam_id.clone(),
                name:          "Steam".into(),
                handle:        player.personaname,
                platform:      Platform::Steam,
                count:         0,
                synced:        "never".into(),
                status:        AccountStatus::Ok,
                api_key:       Some(api_key),
                access_token:  None,
                refresh_token: None,
                expires_at:    None,
                egs_session:   None,
            };
            store.data.accounts.push(account.clone());
            Account { api_key: None, ..account }
        }
    };

    store.save().map_err(|e| e.to_string())?;
    Ok(result)
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
        Platform::Gog   => sync_gog(state, account_id).await,
        Platform::Epic  => sync_epic(state, account_id).await,
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
            let installed = playtime.map(|h| h > 0.0).unwrap_or(false);

            // First pass: look for existing game with this Steam app_id in any copy
            let existing_by_app_id = store.data.games.iter_mut().find(|g| {
                g.copies.iter().any(|c| c.platform == Platform::Steam && c.app_id == app_id_str)
            });

            if let Some(game) = existing_by_app_id {
                // Update Steam copy
                if let Some(copy) = game.copies.iter_mut().find(|c| c.platform == Platform::Steam && c.app_id == app_id_str) {
                    copy.playtime = playtime;
                    copy.last = last.clone();
                    copy.installed = installed;
                }
                updated += 1;
            } else {
                // Second pass: try fuzzy match by normalized title
                let normalized = normalize_title(&sg.name);
                let fuzzy_match = store.data.games.iter_mut().find(|g| {
                    normalize_title(&g.title) == normalized
                });

                if let Some(game) = fuzzy_match {
                    // Add Steam copy to existing game
                    game.copies.push(GameCopy {
                        platform: Platform::Steam,
                        app_id: app_id_str,
                        account_id: Some(account_id.clone()),
                        installed,
                        playtime,
                        last: last.clone(),
                    });
                    updated += 1;
                } else {
                    // Create new game with Steam copy
                    let next_id = store.data.games.iter().map(|g| g.id).max().unwrap_or(0) + 1;
                    store.data.games.push(Game {
                        id: next_id,
                        title: sg.name.clone(),
                        cover: None,
                        rating: 0,
                        status: GameStatus::Backlog,
                        tags: vec![],
                        note: String::new(),
                        genre: vec![],
                        dev: String::new(),
                        year: 0,
                        metacritic: None,
                        opencritic: None,
                        user_score: None,
                        links: Default::default(),
                        custom: vec![],
                        copies: vec![GameCopy {
                            platform: Platform::Steam,
                            app_id: app_id_str,
                            account_id: Some(account_id.clone()),
                            installed,
                            playtime,
                            last: last.clone(),
                        }],
                    });
                    added += 1;
                }
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
                // Find game with this Steam app_id in any copy
                if let Some(game) = store.data.games.iter_mut().find(|g| {
                    g.copies.iter().any(|c| c.platform == Platform::Steam && c.app_id == app_id_str)
                }) {
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

// ── Epic Games OAuth ───────────────────────────────────────────────────────

// Injected into every page load in the Epic auth webview.
//
// Flow:
//   logout (+ federated logout across Epic domains)
//     → login page  →  (user signs in)  →  store.epicgames.com
//                                                    ↓
//        /id/api/redirect?clientId=LAUNCHER&responseType=code&_=ts
//                                                    ↓  (JSON body)
//        {"redirectUrl":"com.epicgames.launcher://apps?code=CODE"}
//                                                    ↓
//        cartridge-epic://auth?code=CODE  →  on_navigation (Rust)
//
// The webview starts at /id/logout, which fans out across multiple Epic-owned domains
// (unrealengine.com, metahuman.com, etc.) before landing on the login page.  We do
// NOT try to set any cross-origin flags during this sequence because those navigations
// reset window.name.  We simply skip all logout/login pages and act only once the
// user is authenticated and on the store.
const EPIC_AUTH_SCRIPT: &str = r#"
(function () {
    function check() {
        var href = window.location.href;

        // Skip every page that is part of the logout or login flow.
        // Epic's federated logout visits several domains (/id/logout, /id/api/logout)
        // before reaching the login page — let all of them pass unmodified.
        if (href.indexOf('/id/logout')  !== -1) return;
        if (href.indexOf('/id/login')   !== -1) return;
        if (href.indexOf('login.epic')  !== -1) return;

        // Once on the store the user is authenticated.  Request an exchange code
        // scoped to the launcher client.  The timestamp prevents the server
        // returning a cached/already-consumed code from the same session.
        if (href.indexOf('store.epicgames.com') !== -1) {
            window.location.replace(
                'https://www.epicgames.com/id/api/redirect' +
                '?clientId=34a02cf8f4414e29b15921876da36f9a' +
                '&responseType=code' +
                '&_=' + Date.now()
            );
            return;
        }

        // Parse the JSON body returned by /id/api/redirect and extract the code.
        // If Epic instead issues a 302 redirect to com.epicgames.launcher://, that
        // is caught directly in the on_navigation Rust callback (no JS needed).
        if (href.indexOf('/id/api/redirect') === -1) return;
        try {
            var text = (document.body && (document.body.innerText || document.body.textContent)) || '';
            if (!text.trim()) return;
            var data = JSON.parse(text.trim());
            var redirectUrl = data.redirectUrl || '';
            var m = redirectUrl.match(/[?&]code=([^&]+)/);
            if (m && m[1]) {
                window.location.replace('cartridge-epic://auth?code=' + encodeURIComponent(m[1]));
            }
        } catch (_) {}
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', check);
    } else {
        check();
    }
})();
"#;


/// Build an authenticated `EpicGames` client from the stored session for `account_id`.
/// Handles token refresh automatically via egs-api's `login()`.
async fn build_epic_client(state: &Db<'_>, account_id: &str) -> Result<EpicGames, String> {
    let session_json = {
        let store = state.lock().unwrap();
        let acc = store.data.accounts.iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| format!("account {account_id} not found"))?;
        acc.egs_session.clone()
            .ok_or_else(|| "Epic session not found — please re-authenticate in Settings".to_string())?
    };

    let user_data: EgsUserData = serde_json::from_str(&session_json)
        .map_err(|e| format!("Epic session corrupt — please re-authenticate: {e}"))?;

    let mut egs = EpicGames::new();
    egs.set_user_details(user_data);

    eprintln!("[Epic] refreshing session via egs-api");
    if !egs.login().await {
        let mut store = state.lock().unwrap();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.status = AccountStatus::Error;
        }
        store.save().ok();
        return Err("Epic session expired — please re-authenticate in Settings".to_string());
    }

    eprintln!("[Epic] session OK");
    Ok(egs)
}

async fn epic_oauth_flow(state: Db<'_>, app: AppHandle) -> Result<Account, String> {
    use std::sync::{Arc, Mutex};

    // Close any lingering window from a previous attempt before opening a fresh one.
    let label = format!("epic-auth-{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]);

    // Start at logout so any previous Epic web session in the webview is cleared.
    // Logout redirects to login, which redirects to the store after sign-in.
    // The init script then takes over: store → exchange/generate → cartridge-epic://.
    let login_url = {
        let login_target = {
            let mut u = reqwest::Url::parse("https://www.epicgames.com/id/login").unwrap();
            u.query_pairs_mut().append_pair("redirectUrl", "https://store.epicgames.com");
            u.to_string()
        };
        let mut url = reqwest::Url::parse("https://www.epicgames.com/id/logout").unwrap();
        url.query_pairs_mut().append_pair("redirectUrl", &login_target);
        url.to_string()
    };

    eprintln!("[Epic] opening auth window ({label}) → {login_url}");

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_nav = Arc::clone(&tx);

    let webview_url = WebviewUrl::External(
        login_url.parse().map_err(|e| format!("invalid Epic auth URL: {e}"))?,
    );

    let window = WebviewWindowBuilder::new(&app, label, webview_url)
        .title("Sign in to Epic Games")
        .inner_size(480.0, 740.0)
        .center()
        .incognito(true) // fresh cookie jar every attempt
        .initialization_script(EPIC_AUTH_SCRIPT)
        .on_navigation(move |url| {
            let url_str = url.to_string();
            eprintln!("[Epic] webview navigating → {}", &url_str[..url_str.len().min(120)]);

            // Two interception points:
            // 1. Our init script redirects via cartridge-epic:// (JSON body case).
            // 2. Epic may 302-redirect directly to com.epicgames.launcher:// instead
            //    of returning JSON — intercept that too.
            let is_our_scheme    = url.scheme() == "cartridge-epic";
            let is_launcher_url  = url_str.starts_with("com.epicgames.launcher://");

            if is_our_scheme || is_launcher_url {
                let code = url
                    .query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| v.into_owned());
                eprintln!(
                    "[Epic] auth URL intercepted (scheme={}) — code present: {}",
                    url.scheme(), code.is_some()
                );
                if let Ok(mut guard) = tx_nav.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(
                            code.ok_or_else(|| "no auth code in Epic redirect".to_string()),
                        );
                    }
                }
                return false; // block — we've captured what we need
            }
            true
        })
        .build()
        .map_err(|e| format!("cannot open Epic login window: {e}"))?;

    let code = tokio::time::timeout(std::time::Duration::from_secs(300), rx)
        .await
        .map_err(|_| "Epic authentication timed out after 5 minutes".to_string())?
        .map_err(|_| "Epic login window was closed before sign-in completed".to_string())?
        .map_err(|e| e)?;

    let _ = window.close();
    eprintln!("[Epic] auth code received (len={}) — exchanging for tokens via egs-api", code.len());
    eprintln!("[Epic] code snippet: {}…", &code[..code.len().min(12)]);

    // Exchange code for session using egs-api (grant_type=exchange_code)
    let mut egs = EpicGames::new();
    egs.try_auth_code(Some(code), None).await
        .map_err(|e| format!("Epic authentication failed: {e}"))?;

    let user_data = egs.user_details();

    let account_id = user_data.account_id.clone()
        .ok_or("Epic token response missing account_id")?;

    let handle = user_data.display_name.clone()
        .unwrap_or_else(|| account_id.clone());

    eprintln!("[Epic] authenticated — account_id: {account_id}, display_name: {handle}");

    let egs_session = serde_json::to_string(&user_data)
        .map_err(|e| format!("Failed to serialize Epic session: {e}"))?;

    let mut store = state.lock().unwrap();

    let result = match store.data.accounts.iter_mut().find(|a| a.id == account_id) {
        Some(existing) => {
            existing.handle      = handle;
            existing.egs_session = Some(egs_session);
            existing.status      = AccountStatus::Ok;
            stripped(existing.clone())
        }
        None => {
            let account = Account {
                id:            account_id.clone(),
                name:          "Epic Games".into(),
                handle,
                platform:      Platform::Epic,
                count:         0,
                synced:        "never".into(),
                status:        AccountStatus::Ok,
                api_key:       None,
                access_token:  None,
                refresh_token: None,
                expires_at:    None,
                egs_session:   Some(egs_session),
            };
            store.data.accounts.push(account.clone());
            stripped(account)
        }
    };

    store.save().map_err(|e| e.to_string())?;
    Ok(result)
}

// ── Epic library sync ───────────────────────────────────────────────────────

async fn sync_epic(state: Db<'_>, account_id: String) -> Result<SyncResult, String> {
    eprintln!("[Epic] sync starting for account {account_id}");

    let mut egs = build_epic_client(&state, &account_id).await?;

    // Fetch all library records — egs-api paginates internally
    eprintln!("[Epic] fetching library via egs-api");
    let library = egs.library_items(true).await
        .ok_or_else(|| "Epic library fetch failed — please try again".to_string())?;

    let all_records = library.records;
    eprintln!("[Epic] library fetch complete — {} records total", all_records.len());

    // Skip Unreal Engine asset namespace and records with no namespace
    let game_records: Vec<_> = all_records.iter()
        .filter(|r| !r.namespace.is_empty() && r.namespace != "ue")
        .collect();

    // Bulk catalog lookup for title, developer, year, cover images
    let pairs: Vec<(&str, &str)> = game_records.iter()
        .map(|r| (r.namespace.as_str(), r.catalog_item_id.as_str()))
        .collect();

    eprintln!("[Epic] fetching catalog metadata for {} items", pairs.len());
    // Returns HashMap<namespace, HashMap<catalog_item_id, AssetInfo>>
    let catalog = egs.bulk_catalog_items(&pairs).await.unwrap_or_default();
    eprintln!("[Epic] catalog fetch complete — {} namespaces", catalog.len());

    // Capture updated session after API calls so refresh token changes are persisted
    let updated_session_json = serde_json::to_string(&egs.user_details()).ok();

    let (mut added, mut updated) = (0u32, 0u32);
    let mut cover_queue: Vec<(String, String)> = Vec::new();

    let covers_dir = {
        let mut store = state.lock().unwrap();

        for record in &game_records {
            let info = catalog
                .get(record.namespace.as_str())
                .and_then(|ns_map| ns_map.get(record.catalog_item_id.as_str()));

            // Skip non-game catalog items (DLC, tools, etc.) when category data is available
            if let Some(i) = info {
                if let Some(cats) = &i.categories {
                    if !cats.is_empty() && !cats.iter().any(|c| c.path.starts_with("games")) {
                        continue;
                    }
                }
            }

            let title = info
                .and_then(|i| i.title.clone())
                .unwrap_or_else(|| record.app_name.clone());

            let dev = info.and_then(|i| i.developer.clone()).unwrap_or_default();

            let year: u32 = info
                .and_then(|i| i.creation_date)
                .map(|dt| dt.format("%Y").to_string())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let cover_url = info.and_then(|i| {
                let images = i.key_images.as_ref()?;
                images.iter()
                    .find(|img| img.type_field == "DieselGameBox")
                    .or_else(|| images.iter().find(|img| img.type_field == "OfferImageTall"))
                    .map(|img| img.url.to_string())
            });

            // First pass: look for existing game with this Epic app_id in any copy
            let existing_by_app_id = store.data.games.iter_mut().find(|g| {
                g.copies.iter().any(|c| c.platform == Platform::Epic && c.app_id == record.app_name)
            });

            if let Some(game) = existing_by_app_id {
                if let Some(copy) = game.copies.iter_mut().find(|c| {
                    c.platform == Platform::Epic && c.app_id == record.app_name
                }) {
                    copy.account_id = Some(account_id.clone());
                }
                if game.year == 0 && year > 0 { game.year = year; }
                if game.dev.is_empty() && !dev.is_empty() { game.dev = dev.clone(); }
                updated += 1;
            } else {
                // Second pass: fuzzy match by normalized title
                let normalized = normalize_title(&title);
                let fuzzy_match = store.data.games.iter_mut().find(|g| {
                    normalize_title(&g.title) == normalized
                });

                if let Some(game) = fuzzy_match {
                    game.copies.push(GameCopy {
                        platform: Platform::Epic,
                        app_id: record.app_name.clone(),
                        account_id: Some(account_id.clone()),
                        installed: false,
                        playtime: None,
                        last: None,
                    });
                    if game.year == 0 && year > 0 { game.year = year; }
                    if game.dev.is_empty() && !dev.is_empty() { game.dev = dev.clone(); }
                    updated += 1;
                } else {
                    let next_id = store.data.games.iter().map(|g| g.id).max().unwrap_or(0) + 1;
                    store.data.games.push(Game {
                        id: next_id,
                        title: title.clone(),
                        cover: None,
                        rating: 0,
                        status: GameStatus::Backlog,
                        tags: vec![],
                        note: String::new(),
                        genre: vec![],
                        dev: dev.clone(),
                        year,
                        metacritic: None,
                        opencritic: None,
                        user_score: None,
                        links: Default::default(),
                        custom: vec![],
                        copies: vec![GameCopy {
                            platform: Platform::Epic,
                            app_id: record.app_name.clone(),
                            account_id: Some(account_id.clone()),
                            installed: false,
                            playtime: None,
                            last: None,
                        }],
                    });
                    added += 1;
                }
            }

            if let Some(url) = cover_url {
                cover_queue.push((record.app_name.clone(), url));
            }
        }

        eprintln!("[Epic] merge complete — added {added}, updated {updated}");

        let now = chrono::Utc::now().format("%b %d, %Y").to_string();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.synced = now;
            acc.status = AccountStatus::Ok;
            acc.count  = added + updated;
            if let Some(json) = updated_session_json {
                acc.egs_session = Some(json);
            }
        }
        store.save().map_err(|e| e.to_string())?;
        store.covers_dir.clone()
    };

    eprintln!("[Epic] downloading covers for {} games", cover_queue.len());

    let client = reqwest::Client::new();
    let mut errors: Vec<String> = Vec::new();

    for (app_name, url) in &cover_queue {
        let cover_path = covers_dir.join(format!("epic_{app_name}.jpg"));
        if cover_path.exists() {
            eprintln!("[Epic] cover {app_name} already cached — skip");
            continue;
        }
        eprintln!("[Epic] cover {app_name} ← {url}");
        match download_cover(&client, url, &cover_path).await {
            Ok(()) => {
                eprintln!("[Epic] cover {app_name} saved");
                let cover_str = cover_path.to_string_lossy().into_owned();
                let mut store = state.lock().unwrap();
                if let Some(game) = store.data.games.iter_mut().find(|g| {
                    g.copies.iter().any(|c| c.platform == Platform::Epic && c.app_id == *app_name)
                }) {
                    game.cover = Some(cover_str);
                }
                store.save().ok();
            }
            Err(e) => {
                eprintln!("[Epic] cover {app_name} FAILED: {e}");
                errors.push(format!("cover {app_name}: {e}"));
            }
        }
    }

    eprintln!("[Epic] sync done — added {added}, updated {updated}, errors {}", errors.len());
    Ok(SyncResult { added, updated, errors })
}

// ── GOG OAuth ──────────────────────────────────────────────────────────────

const GOG_CLIENT_ID:     &str = "46899977096215655";
const GOG_CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";

// Response shapes

#[derive(serde::Deserialize)]
struct GogTokenResponse {
    access_token:  String,
    refresh_token: Option<String>,
    expires_in:    u64,
    user_id:       Option<String>,
}

#[derive(serde::Deserialize)]
struct GogUserData {
    username: Option<String>,
    #[serde(rename = "userId")]
    user_id:  Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GogProductsPage {
    total_pages: u32,
    products:    Vec<GogProduct>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GogProduct {
    id:           u64,
    title:        String,
    #[serde(default)]
    image:        Option<String>,
    #[serde(default)]
    is_movie:     Option<bool>,
    #[serde(default)]
    release_date: Option<serde_json::Value>,
}

// GOG's auth server only accepts this exact registered redirect URI for client 46899977096215655.
// We open a Tauri WebviewWindow and intercept navigation to this URL instead of using a local
// HTTP server, because a dynamic localhost port would never match the registered URI.
const GOG_REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

// ── Token helpers ───────────────────────────────────────────────────────────

fn stripped(account: Account) -> Account {
    Account { api_key: None, access_token: None, refresh_token: None, egs_session: None, ..account }
}

/// Returns a valid GOG access token for `account_id`, refreshing it first if
/// it is within 5 minutes of expiry.  Saves updated tokens to the store.
async fn ensure_gog_token(state: &Db<'_>, account_id: &str) -> Result<String, String> {
    let (access_token, refresh_token, expires_at) = {
        let store = state.lock().unwrap();
        let acc = store.data.accounts.iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| format!("account {account_id} not found"))?;
        (acc.access_token.clone(), acc.refresh_token.clone(), acc.expires_at)
    };

    let now = chrono::Utc::now().timestamp();
    let still_valid = access_token.is_some()
        && expires_at.map(|exp| exp - now > 300).unwrap_or(false);

    if still_valid {
        return Ok(access_token.unwrap());
    }

    let refresh_token = refresh_token.ok_or(
        "GOG session expired — please re-authenticate in Settings"
    )?;

    eprintln!("[GOG] access token expired — refreshing with refresh_token");

    let client = reqwest::Client::new();
    let resp = client
        .get("https://auth.gog.com/token")
        .query(&[
            ("client_id",     GOG_CLIENT_ID),
            ("client_secret", GOG_CLIENT_SECRET),
            ("grant_type",    "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
        ])
        .send().await.map_err(|e| e.to_string())?;

    eprintln!("[GOG] token refresh → HTTP {}", resp.status());

    let tokens: GogTokenResponse = resp
        .error_for_status()
        .map_err(|e| format!("token refresh failed — please re-authenticate: {e}"))?
        .json().await.map_err(|e| e.to_string())?;

    let new_token     = tokens.access_token.clone();
    let new_expires   = now + tokens.expires_in as i64;
    let new_refresh   = tokens.refresh_token.unwrap_or(refresh_token);

    eprintln!("[GOG] token refreshed — new expiry in {}s", tokens.expires_in);

    {
        let mut store = state.lock().unwrap();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.access_token  = Some(tokens.access_token);
            acc.refresh_token = Some(new_refresh);
            acc.expires_at    = Some(new_expires);
        }
        store.save().ok();
    }

    Ok(new_token)
}

// ── OAuth flow command ──────────────────────────────────────────────────────

/// Open a Tauri WebviewWindow at the GOG login page.  The `on_navigation` hook
/// intercepts the redirect to `on_login_success`, pulls the auth code, and
/// signals the async task via a oneshot channel.  No system browser is involved,
/// so the registered redirect URI never needs to be a localhost address.
async fn gog_oauth_flow(state: Db<'_>, app: AppHandle) -> Result<Account, String> {
    use std::sync::{Arc, Mutex};

    let auth_url = {
        let mut url = reqwest::Url::parse("https://auth.gog.com/auth").unwrap();
        url.query_pairs_mut()
            .append_pair("client_id",     GOG_CLIENT_ID)
            .append_pair("redirect_uri",  GOG_REDIRECT_URI)
            .append_pair("response_type", "code")
            .append_pair("layout",        "client2");
        url.to_string()
    };

    eprintln!("[GOG] opening auth window → {auth_url}");

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_nav = Arc::clone(&tx);

    let webview_url = WebviewUrl::External(
        auth_url.parse().map_err(|e| format!("invalid auth URL: {e}"))?,
    );

    let window = WebviewWindowBuilder::new(&app, "gog-auth", webview_url)
        .title("Sign in to GOG")
        .inner_size(480.0, 740.0)
        .center()
        .incognito(true) // fresh cookie jar every attempt
        // Intercept every navigation attempt in this window.
        // When GOG redirects to on_login_success we grab the code and cancel
        // the navigation (return false) so the WebView doesn't try to load it.
        .on_navigation(move |url| {
            eprintln!("[GOG] webview navigating → {url}");
            if url.host_str() == Some("embed.gog.com")
                && url.path() == "/on_login_success"
            {
                let code = url
                    .query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| v.into_owned());

                eprintln!(
                    "[GOG] redirect intercepted — code present: {}",
                    code.is_some()
                );

                if let Ok(mut guard) = tx_nav.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(
                            code.ok_or_else(|| "no code in GOG redirect URL".to_string()),
                        );
                    }
                }
                false // cancel — we don't want to render the redirect target
            } else {
                true // allow all other navigation inside the login flow
            }
        })
        .build()
        .map_err(|e| format!("cannot open GOG login window: {e}"))?;

    // Wait up to 5 minutes for the user to complete sign-in
    let code = tokio::time::timeout(std::time::Duration::from_secs(300), rx)
        .await
        .map_err(|_| "GOG authentication timed out after 5 minutes".to_string())?
        .map_err(|_| "GOG login window was closed before sign-in completed".to_string())?
        .map_err(|e| e)?;

    let _ = window.close();
    eprintln!("[GOG] auth code received — exchanging for tokens");

    let client = reqwest::Client::new();

    // Exchange authorization code for tokens
    let token_resp = client
        .get("https://auth.gog.com/token")
        .query(&[
            ("client_id",     GOG_CLIENT_ID),
            ("client_secret", GOG_CLIENT_SECRET),
            ("grant_type",    "authorization_code"),
            ("code",          code.as_str()),
            ("redirect_uri",  GOG_REDIRECT_URI),
        ])
        .send().await.map_err(|e| e.to_string())?;

    eprintln!("[GOG] token exchange → HTTP {}", token_resp.status());

    let tokens: GogTokenResponse = token_resp
        .error_for_status()
        .map_err(|e| format!("token exchange failed: {e}"))?
        .json().await.map_err(|e| format!("token parse error: {e}"))?;

    eprintln!(
        "[GOG] tokens received — user_id: {:?}, expires_in: {}s",
        tokens.user_id, tokens.expires_in
    );

    let expires_at = chrono::Utc::now().timestamp() + tokens.expires_in as i64;

    // Fetch user profile
    eprintln!("[GOG] fetching user profile → embed.gog.com/userData.json");
    let user_resp = client
        .get("https://embed.gog.com/userData.json")
        .bearer_auth(&tokens.access_token)
        .send().await.map_err(|e| e.to_string())?;

    eprintln!("[GOG] userData → HTTP {}", user_resp.status());

    let user: GogUserData = user_resp
        .error_for_status()
        .map_err(|e| format!("user data fetch failed: {e}"))?
        .json().await.map_err(|e| format!("user data parse error: {e}"))?;

    eprintln!("[GOG] user: {:?} (id: {:?})", user.username, user.user_id);

    let handle = user.username.clone()
        .or_else(|| user.user_id.clone())
        .unwrap_or_else(|| "GOG account".to_string());

    let account_id = tokens.user_id.clone()
        .or_else(|| user.user_id.clone())
        .unwrap_or_else(|| "gog_local".to_string());

    let mut store = state.lock().unwrap();

    let result = match store.data.accounts.iter_mut().find(|a| a.id == account_id) {
        Some(existing) => {
            existing.handle       = handle;
            existing.access_token = Some(tokens.access_token);
            // Preserve the previously stored refresh token when the server doesn't
            // issue a new one (shouldn't happen on a fresh auth_code exchange, but
            // is defensive for future refresh-token rotations).
            if let Some(rt) = tokens.refresh_token {
                existing.refresh_token = Some(rt);
            }
            existing.expires_at   = Some(expires_at);
            existing.status       = AccountStatus::Ok;
            stripped(existing.clone())
        }
        None => {
            let account = Account {
                id:            account_id.clone(),
                name:          "GOG".into(),
                handle,
                platform:      Platform::Gog,
                count:         0,
                synced:        "never".into(),
                status:        AccountStatus::Ok,
                api_key:       None,
                access_token:  Some(tokens.access_token),
                refresh_token: tokens.refresh_token,
                expires_at:    Some(expires_at),
                egs_session:   None,
            };
            store.data.accounts.push(account.clone());
            stripped(account)
        }
    };

    store.save().map_err(|e| e.to_string())?;
    Ok(result)
}

/// Dispatch OAuth flow for the requested platform.
#[tauri::command]
pub async fn start_oauth_flow(
    state: Db<'_>,
    app: AppHandle,
    platform: Platform,
) -> Result<Account, String> {
    match platform {
        Platform::Gog  => gog_oauth_flow(state, app).await,
        Platform::Epic => epic_oauth_flow(state, app).await,
        p => Err(format!("{p:?} OAuth not yet implemented")),
    }
}

// ── GOG library sync ────────────────────────────────────────────────────────

fn gog_cover_url(image: &str) -> String {
    let base = if image.starts_with("//") {
        format!("https:{image}")
    } else {
        image.to_string()
    };
    // Portrait format (600×900), same aspect ratio as Steam library art
    format!("{base}_product_card_v2_mobile_slider_639.jpg")
}

fn gog_year(release_date: &Option<serde_json::Value>) -> u32 {
    // GOG sends {"date": "2015-05-19 00:00:00.000000", ...}
    release_date
        .as_ref()
        .and_then(|v| v.get("date"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.get(..4))
        .and_then(|y| y.parse().ok())
        .unwrap_or(0)
}

async fn sync_gog(state: Db<'_>, account_id: String) -> Result<SyncResult, String> {
    eprintln!("[GOG] sync starting for account {account_id}");

    // Refresh token if needed; mark account Error if we cannot
    let access_token = match ensure_gog_token(&state, &account_id).await {
        Ok(t) => { eprintln!("[GOG] access token OK"); t }
        Err(e) => {
            eprintln!("[GOG] token error: {e}");
            let mut store = state.lock().unwrap();
            if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
                acc.status = AccountStatus::Error;
            }
            store.save().ok();
            return Err(e);
        }
    };

    let client = reqwest::Client::new();
    let mut all_products: Vec<GogProduct> = Vec::new();
    let mut page = 1u32;

    // Paginate through the full library
    loop {
        eprintln!("[GOG] fetching library page {page}");
        let raw = client
            .get("https://embed.gog.com/account/getFilteredProducts")
            .query(&[("mediaType", "1"), ("page", page.to_string().as_str())])
            .bearer_auth(&access_token)
            .send().await.map_err(|e| e.to_string())?;

        eprintln!("[GOG] library page {page} → HTTP {}", raw.status());

        let resp: GogProductsPage = raw
            .error_for_status()
            .map_err(|e| format!("library fetch failed: {e}"))?
            .json().await.map_err(|e| format!("library parse failed: {e}"))?;

        let total = resp.total_pages;
        let count = resp.products.len();
        // Filter out movies; mediaType=1 already targets games but double-check
        all_products.extend(
            resp.products.into_iter().filter(|p| p.is_movie != Some(true))
        );

        eprintln!("[GOG] page {page}/{total} — {count} products");

        if page >= total { break; }
        page += 1;
    }

    eprintln!("[GOG] library fetch complete — {} games total", all_products.len());

    let (mut added, mut updated) = (0u32, 0u32);
    let mut cover_queue: Vec<(String, String)> = Vec::new(); // (app_id_str, url)

    let covers_dir = {
        let mut store = state.lock().unwrap();

        for p in &all_products {
            let app_id_str = p.id.to_string();
            let year = gog_year(&p.release_date);

            // First pass: look for existing game with this GOG app_id in any copy
            let existing_by_app_id = store.data.games.iter_mut().find(|g| {
                g.copies.iter().any(|c| c.platform == Platform::Gog && c.app_id == app_id_str)
            });

            if let Some(game) = existing_by_app_id {
                // Update GOG copy metadata if available
                if let Some(copy) = game.copies.iter_mut().find(|c| c.platform == Platform::Gog && c.app_id == app_id_str) {
                    copy.account_id = Some(account_id.clone());
                }
                // Update game-level metadata if missing
                if game.year == 0 && year > 0 { game.year = year; }
                updated += 1;
            } else {
                // Second pass: try fuzzy match by normalized title
                let normalized = normalize_title(&p.title);
                let fuzzy_match = store.data.games.iter_mut().find(|g| {
                    normalize_title(&g.title) == normalized
                });

                if let Some(game) = fuzzy_match {
                    // Add GOG copy to existing game
                    game.copies.push(GameCopy {
                        platform: Platform::Gog,
                        app_id: app_id_str.clone(),
                        account_id: Some(account_id.clone()),
                        installed: false,
                        playtime: None,
                        last: None,
                    });
                    // Update game-level metadata if missing
                    if game.year == 0 && year > 0 { game.year = year; }
                    updated += 1;
                } else {
                    // Create new game with GOG copy
                    let next_id = store.data.games.iter().map(|g| g.id).max().unwrap_or(0) + 1;
                    store.data.games.push(Game {
                        id: next_id,
                        title: p.title.clone(),
                        cover: None,
                        rating: 0,
                        status: GameStatus::Backlog,
                        tags: vec![],
                        note: String::new(),
                        genre: vec![],
                        dev: String::new(),
                        year,
                        metacritic: None,
                        opencritic: None,
                        user_score: None,
                        links: Default::default(),
                        custom: vec![],
                        copies: vec![GameCopy {
                            platform: Platform::Gog,
                            app_id: app_id_str.clone(),
                            account_id: Some(account_id.clone()),
                            installed: false,
                            playtime: None,
                            last: None,
                        }],
                    });
                    added += 1;
                }
            }

            if let Some(ref img) = p.image {
                cover_queue.push((app_id_str, gog_cover_url(img)));
            }
        }

        eprintln!("[GOG] merge complete — added {added}, updated {updated}");

        let now = chrono::Utc::now().format("%b %d, %Y").to_string();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.synced = now;
            acc.status = AccountStatus::Ok;
            acc.count  = all_products.len() as u32;
        }
        store.save().map_err(|e| e.to_string())?;
        store.covers_dir.clone()
    }; // lock released

    eprintln!("[GOG] downloading covers for {} games", cover_queue.len());

    // Download cover art (async, no lock held)
    let mut errors: Vec<String> = Vec::new();

    for (app_id_str, url) in &cover_queue {
        let cover_path = covers_dir.join(format!("gog_{app_id_str}.jpg"));
        if cover_path.exists() {
            eprintln!("[GOG] cover {app_id_str} already cached — skip");
            continue;
        }

        eprintln!("[GOG] cover {app_id_str} ← {url}");
        match download_cover(&client, url, &cover_path).await {
            Ok(()) => {
                eprintln!("[GOG] cover {app_id_str} saved");
                let cover_str = cover_path.to_string_lossy().into_owned();
                let mut store = state.lock().unwrap();
                // Find game with this GOG app_id in any copy
                if let Some(game) = store.data.games.iter_mut().find(|g| {
                    g.copies.iter().any(|c| c.platform == Platform::Gog && c.app_id == *app_id_str)
                }) {
                    game.cover = Some(cover_str);
                }
                store.save().ok();
            }
            Err(e) => {
                eprintln!("[GOG] cover {app_id_str} FAILED: {e}");
                errors.push(format!("cover {app_id_str}: {e}"));
            }
        }
    }

    eprintln!("[GOG] sync done — added {added}, updated {updated}, errors {}", errors.len());
    Ok(SyncResult { added, updated, errors })
}
