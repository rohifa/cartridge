use std::sync::Mutex;
use tauri::State;

use egs_api::api::types::account::UserData as EgsUserData;
use egs_api::api::types::asset_info::AssetInfo as EgsAssetInfo;
use egs_api::api::types::epic_asset::EpicAsset as EgsAsset;
use egs_api::api::types::library::Record as EgsRecord;
use egs_api::EpicGames;

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
///
/// Keeps only letters and digits (Unicode-aware), so punctuation, ™/®,
/// typographic quotes (’) and spacing differences don't matter; `&` == `and`.
/// Editions ("GOTY", "Director's Cut") are deliberately NOT stripped — that
/// would also merge distinct games like "Tomb Raider" / "Tomb Raider: Anniversary".
fn normalize_title(s: &str) -> String {
    s.replace('&', " and ")
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn has_user_data(g: &Game) -> bool {
    g.rating > 0
        || !g.tags.is_empty()
        || !g.note.is_empty()
        || !g.custom.is_empty()
        || !matches!(g.status, GameStatus::Backlog)
}

/// Fold `other` into `primary`: copies are combined, user data and metadata
/// fill in wherever `primary` has none.
fn absorb_game(primary: &mut Game, other: Game) {
    for c in other.copies {
        match primary
            .copies
            .iter_mut()
            .find(|p| p.platform == c.platform && p.app_id == c.app_id)
        {
            // Same store copy on both entries: keep the most recent session.
            Some(p) => {
                if c.last_played_at > p.last_played_at {
                    p.last_played_at = c.last_played_at;
                }
            }
            None => primary.copies.push(c),
        }
    }
    if primary.rating == 0 {
        primary.rating = other.rating;
    }
    if matches!(primary.status, GameStatus::Backlog) {
        primary.status = other.status;
    }
    for t in other.tags {
        if !primary.tags.contains(&t) {
            primary.tags.push(t);
        }
    }
    if !other.note.is_empty() && other.note != primary.note {
        if primary.note.is_empty() {
            primary.note = other.note;
        } else {
            primary.note.push_str("\n\n");
            primary.note.push_str(&other.note);
        }
    }
    primary.custom.extend(other.custom);
    if primary.cover.is_none() {
        primary.cover = other.cover;
    }
    if primary.genre.is_empty() {
        primary.genre = other.genre;
    }
    if primary.dev.is_empty() {
        primary.dev = other.dev;
    }
    if primary.year == 0 {
        primary.year = other.year;
    }
    if primary.steam_match.is_none() && !primary.steam_match_manual {
        primary.steam_match = other.steam_match;
        primary.steam_match_checked_at = other.steam_match_checked_at;
        primary.steam_match_manual = other.steam_match_manual;
        primary.steam_match_rev = other.steam_match_rev;
    }
    if primary.metacritic.is_none() {
        primary.metacritic = other.metacritic;
        primary.metacritic_checked_at = other.metacritic_checked_at;
    }
    if primary.opencritic.is_none() {
        primary.opencritic = other.opencritic;
    }
    if primary.user_score_reviews.is_none() {
        primary.user_score = other.user_score;
        primary.user_score_reviews = other.user_score_reviews;
    }
    for (k, v) in other.links {
        primary.links.entry(k).or_insert(v);
    }
}

/// Merge library entries that are the same game owned on several platforms
/// (same normalized title) into one entry with multiple copies.
///
/// Title matching at insert time only catches games that are new to the store;
/// entries that were created separately (older builds, Epic placeholders that got
/// their real title later) would otherwise stay duplicated forever.  Runs after
/// every sync.  The surviving entry is the one with user data, then a cover, then
/// the oldest id; list memberships are remapped to it.  Returns entries removed.
fn merge_duplicate_games(data: &mut crate::models::CartridgeData) -> u32 {
    use std::collections::{HashMap, HashSet};

    // Epic records without catalog data are titled with their internal app_name
    // (e.g. "Marigold") — never match on those.
    let is_placeholder = |g: &Game| {
        g.copies
            .iter()
            .any(|c| c.platform == Platform::Epic && c.app_id == g.title)
    };

    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, g) in data.games.iter().enumerate() {
        if is_placeholder(g) {
            continue;
        }
        let key = normalize_title(&g.title);
        if !key.is_empty() {
            groups.entry(key).or_default().push(i);
        }
    }

    let mut to_remove: Vec<usize> = Vec::new();
    let mut remap: HashMap<u64, u64> = HashMap::new();

    for idxs in groups.into_values().filter(|v| v.len() > 1) {
        let primary = *idxs
            .iter()
            .max_by_key(|&&i| {
                let g = &data.games[i];
                (has_user_data(g), g.cover.is_some(), std::cmp::Reverse(g.id))
            })
            .unwrap();

        for &i in idxs.iter().filter(|&&i| i != primary) {
            let other = data.games[i].clone();
            remap.insert(other.id, data.games[primary].id);
            absorb_game(&mut data.games[primary], other);
            to_remove.push(i);
        }
    }

    to_remove.sort_unstable();
    for &i in to_remove.iter().rev() {
        data.games.remove(i);
    }

    for list in &mut data.lists {
        if let Some(ids) = &mut list.game_ids {
            for id in ids.iter_mut() {
                if let Some(&new_id) = remap.get(id) {
                    *id = new_id;
                }
            }
            let mut seen = HashSet::new();
            ids.retain(|id| seen.insert(*id));
        }
    }

    to_remove.len() as u32
}

async fn download_cover(
    client: &reqwest::Client,
    url: &str,
    dest: &std::path::Path,
) -> anyhow::Result<()> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    std::fs::write(dest, &bytes)?;
    Ok(())
}

/// Download `url` to `dest`. Returns Ok(false) when the image doesn't exist
/// (4xx), so callers can fall through to the next candidate.
async fn try_download(
    client: &reqwest::Client,
    url: &str,
    dest: &std::path::Path,
) -> anyhow::Result<bool> {
    let resp = client.get(url).send().await?;
    if resp.status().is_client_error() {
        return Ok(false);
    }
    let bytes = resp.error_for_status()?.bytes().await?;
    std::fs::write(dest, &bytes)?;
    Ok(true)
}

/// Some Steam app IDs (multiplayer components, old sub-apps) redirect to a
/// parent app in the store, e.g. 2130 → 2100. Returns the parent if different.
async fn steam_parent_appid(client: &reqwest::Client, appid: u64) -> Option<u64> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic");
    let v: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let parent = v
        .get(appid.to_string())?
        .get("data")?
        .get("steam_appid")?
        .as_u64()?;
    (parent != appid).then_some(parent)
}

/// Fetch the best available Steam cover for `appid`, in order:
/// portrait 2x → portrait 1x → parent app's portraits → landscape header.
/// Returns Ok(false) if none exist.
async fn download_steam_cover(
    client: &reqwest::Client,
    appid: u64,
    dest: &std::path::Path,
) -> anyhow::Result<bool> {
    const CDN: &str = "https://cdn.akamai.steamstatic.com/steam/apps";
    const PORTRAITS: [&str; 2] = ["library_600x900_2x.jpg", "library_600x900.jpg"];

    for file in PORTRAITS {
        if try_download(client, &format!("{CDN}/{appid}/{file}"), dest).await? {
            return Ok(true);
        }
    }
    if let Some(parent) = steam_parent_appid(client, appid).await {
        for file in PORTRAITS {
            if try_download(client, &format!("{CDN}/{parent}/{file}"), dest).await? {
                return Ok(true);
            }
        }
    }
    try_download(client, &format!("{CDN}/{appid}/header.jpg"), dest).await
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
            existing.handle = player.personaname;
            existing.api_key = Some(api_key);
            Account {
                api_key: None,
                ..existing.clone()
            }
        }
        None => {
            let account = Account {
                id: steam_id.clone(),
                name: "Steam".into(),
                handle: player.personaname,
                platform: Platform::Steam,
                count: 0,
                synced: "never".into(),
                status: AccountStatus::Ok,
                api_key: Some(api_key),
                access_token: None,
                refresh_token: None,
                expires_at: None,
                egs_session: None,
            };
            store.data.accounts.push(account.clone());
            Account {
                api_key: None,
                ..account
            }
        }
    };

    store.save().map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub async fn sync_account(
    app: AppHandle,
    state: Db<'_>,
    account_id: String,
) -> Result<SyncResult, String> {
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

    let result = match platform {
        Platform::Steam => sync_steam(state, api_key, account_id).await,
        Platform::Gog => sync_gog(state, account_id).await,
        Platform::Epic => sync_epic(state, account_id).await,
    };
    // Paced background job: title matching for new GOG/Epic games, reviews,
    // Metacritic. Starts after covers so the two don't compete for the store
    // API's rate limit (the cover fallback uses appdetails too).
    crate::commands::scores::spawn_score_refresh(app);
    result
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
            let last_played_at = sg.rtime_last_played.filter(|&t| t > 0).map(|t| t as i64);
            let installed = playtime.map(|h| h > 0.0).unwrap_or(false);

            // First pass: look for existing game with this Steam app_id in any copy
            let existing_by_app_id = store.data.games.iter_mut().find(|g| {
                g.copies
                    .iter()
                    .any(|c| c.platform == Platform::Steam && c.app_id == app_id_str)
            });

            if let Some(game) = existing_by_app_id {
                // Update Steam copy
                if let Some(copy) = game
                    .copies
                    .iter_mut()
                    .find(|c| c.platform == Platform::Steam && c.app_id == app_id_str)
                {
                    copy.playtime = playtime;
                    copy.last_played_at = last_played_at;
                    copy.installed = installed;
                }
                updated += 1;
            } else {
                // Second pass: try fuzzy match by normalized title
                let normalized = normalize_title(&sg.name);
                let fuzzy_match = store
                    .data
                    .games
                    .iter_mut()
                    .find(|g| normalize_title(&g.title) == normalized);

                if let Some(game) = fuzzy_match {
                    // Add Steam copy to existing game
                    game.copies.push(GameCopy {
                        platform: Platform::Steam,
                        app_id: app_id_str,
                        account_id: Some(account_id.clone()),
                        installed,
                        playtime,
                        last_played_at,
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
                        user_score_reviews: None,
                        metacritic_checked_at: None,
                        steam_match: None,
                        steam_match_checked_at: None,
                        steam_match_manual: false,
                        steam_match_rev: 0,
                        links: Default::default(),
                        custom: vec![],
                        copies: vec![GameCopy {
                            platform: Platform::Steam,
                            app_id: app_id_str,
                            account_id: Some(account_id.clone()),
                            installed,
                            playtime,
                            last_played_at,
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
        let merged = merge_duplicate_games(&mut store.data);
        if merged > 0 {
            eprintln!("[Steam] merged {merged} cross-platform duplicates");
        }
        store.save().map_err(|e| e.to_string())?;

        store.covers_dir.clone()
    }; // lock released here

    let mut errors: Vec<String> = Vec::new();
    let client = reqwest::Client::new();

    // Review scores: a few batched requests, so done before the slower covers.
    if let Err(e) = crate::commands::scores::refresh_steam_reviews(state.inner(), &client).await {
        errors.push(format!("Steam review scores: {e:#}"));
    }

    // Download cover art (async, no lock held)

    for sg in &steam_games {
        let cover_path = covers_dir.join(format!("{}.jpg", sg.appid));
        if cover_path.exists() {
            continue;
        }
        match download_steam_cover(&client, sg.appid, &cover_path).await {
            Ok(false) => {} // no art on Steam for this app — leave "No cover art"
            Ok(true) => {
                let app_id_str = sg.appid.to_string();
                let cover_str = cover_path.to_string_lossy().into_owned();
                let mut store = state.lock().unwrap();
                // Find game with this Steam app_id in any copy; keep a cover it
                // already has from another platform.
                if let Some(game) = store.data.games.iter_mut().find(|g| {
                    g.copies
                        .iter()
                        .any(|c| c.platform == Platform::Steam && c.app_id == app_id_str)
                }) {
                    if game.cover.is_none() {
                        game.cover = Some(cover_str);
                    }
                }
                store.save().ok();
            }
            Err(e) => errors.push(format!("cover {}: {e}", sg.appid)),
        }
    }

    Ok(SyncResult {
        added,
        updated,
        errors,
    })
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
//        /id/api/redirect?clientId=LAUNCHER&responseType=code
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
        // scoped to the launcher client.  Do NOT add extra query params (e.g. a
        // cache-busting `_=ts`): Epic validates the query strictly and rejects
        // unknown keys with "errors.com.epicgames.accountportal.validation.unknown".
        // Each request mints a fresh code anyway, and the webview is incognito.
        if (href.indexOf('store.epicgames.com') !== -1) {
            window.location.replace(
                'https://www.epicgames.com/id/api/redirect' +
                '?clientId=34a02cf8f4414e29b15921876da36f9a' +
                '&responseType=code'
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
        let acc = store
            .data
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| format!("account {account_id} not found"))?;
        acc.egs_session.clone().ok_or_else(|| {
            "Epic session not found — please re-authenticate in Settings".to_string()
        })?
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
    let label = format!(
        "epic-auth-{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    // Start at logout so any previous Epic web session in the webview is cleared.
    // Logout redirects to login, which redirects to the store after sign-in.
    // The init script then takes over: store → exchange/generate → cartridge-epic://.
    let login_url = {
        let login_target = {
            let mut u = reqwest::Url::parse("https://www.epicgames.com/id/login").unwrap();
            u.query_pairs_mut()
                .append_pair("redirectUrl", "https://store.epicgames.com");
            u.to_string()
        };
        let mut url = reqwest::Url::parse("https://www.epicgames.com/id/logout").unwrap();
        url.query_pairs_mut()
            .append_pair("redirectUrl", &login_target);
        url.to_string()
    };

    eprintln!("[Epic] opening auth window ({label}) → {login_url}");

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_nav = Arc::clone(&tx);

    let webview_url = WebviewUrl::External(
        login_url
            .parse()
            .map_err(|e| format!("invalid Epic auth URL: {e}"))?,
    );

    let window = WebviewWindowBuilder::new(&app, label, webview_url)
        .title("Sign in to Epic Games")
        .inner_size(480.0, 740.0)
        .center()
        .incognito(true) // fresh cookie jar every attempt
        .initialization_script(EPIC_AUTH_SCRIPT)
        .on_navigation(move |url| {
            let url_str = url.to_string();
            eprintln!(
                "[Epic] webview navigating → {}",
                &url_str[..url_str.len().min(120)]
            );

            // Two interception points:
            // 1. Our init script redirects via cartridge-epic:// (JSON body case).
            // 2. Epic may 302-redirect directly to com.epicgames.launcher:// instead
            //    of returning JSON — intercept that too.
            let is_our_scheme = url.scheme() == "cartridge-epic";
            let is_launcher_url = url_str.starts_with("com.epicgames.launcher://");

            if is_our_scheme || is_launcher_url {
                let code = url
                    .query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| v.into_owned());
                eprintln!(
                    "[Epic] auth URL intercepted (scheme={}) — code present: {}",
                    url.scheme(),
                    code.is_some()
                );
                if let Ok(mut guard) = tx_nav.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender
                            .send(code.ok_or_else(|| "no auth code in Epic redirect".to_string()));
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
        .map_err(|_| "Epic login window was closed before sign-in completed".to_string())??;

    let _ = window.close();
    eprintln!(
        "[Epic] auth code received (len={}) — exchanging for tokens via egs-api",
        code.len()
    );
    eprintln!("[Epic] code snippet: {}…", &code[..code.len().min(12)]);

    // /id/api/redirect?responseType=code returns an *authorization* code, not an
    // exchange code — pass it as the 2nd arg (grant_type=authorization_code).
    // Passing it as exchange_token makes Epic reply "Id must not be null".
    let mut egs = EpicGames::new();
    egs.try_auth_code(None, Some(code))
        .await
        .map_err(|e| format!("Epic authentication failed: {e}"))?;

    let user_data = egs.user_details();

    let account_id = user_data
        .account_id
        .clone()
        .ok_or("Epic token response missing account_id")?;

    let handle = user_data
        .display_name
        .clone()
        .unwrap_or_else(|| account_id.clone());

    eprintln!("[Epic] authenticated — account_id: {account_id}, display_name: {handle}");

    let egs_session = serde_json::to_string(&user_data)
        .map_err(|e| format!("Failed to serialize Epic session: {e}"))?;

    let mut store = state.lock().unwrap();

    let result = match store.data.accounts.iter_mut().find(|a| a.id == account_id) {
        Some(existing) => {
            existing.handle = handle;
            existing.egs_session = Some(egs_session);
            existing.status = AccountStatus::Ok;
            stripped(existing.clone())
        }
        None => {
            let account = Account {
                id: account_id.clone(),
                name: "Epic Games".into(),
                handle,
                platform: Platform::Epic,
                count: 0,
                synced: "never".into(),
                status: AccountStatus::Ok,
                api_key: None,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                egs_session: Some(egs_session),
            };
            store.data.accounts.push(account.clone());
            stripped(account)
        }
    };

    store.save().map_err(|e| e.to_string())?;
    Ok(result)
}

// ── Epic library sync ───────────────────────────────────────────────────────

/// Fetch catalog metadata for every library record, keyed by catalog_item_id.
///
/// Uses the per-namespace GET endpoint (`try_asset_info`).  egs-api's
/// `bulk_catalog_items` POST is rejected by Epic with 415 unsupported_media_type.
/// Failures are logged and skipped; the game still imports without metadata.
async fn fetch_epic_catalog(
    egs: &EpicGames,
    records: &[&EgsRecord],
) -> std::collections::HashMap<String, EgsAssetInfo> {
    use std::sync::Arc;
    const CONCURRENCY: usize = 8;

    let sem = Arc::new(tokio::sync::Semaphore::new(CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();

    for r in records {
        let asset = EgsAsset {
            app_name: r.app_name.clone(),
            catalog_item_id: r.catalog_item_id.clone(),
            namespace: r.namespace.clone(),
            ..Default::default()
        };
        let mut egs = egs.clone();
        let sem = Arc::clone(&sem);
        tasks.spawn(async move {
            let _permit = sem.acquire_owned().await;
            let res = egs.try_asset_info(&asset).await;
            (asset, res)
        });
    }

    let mut out = std::collections::HashMap::new();
    let mut failures = 0usize;
    while let Some(joined) = tasks.join_next().await {
        let Ok((asset, res)) = joined else {
            failures += 1;
            continue;
        };
        match res {
            Ok(Some(info)) => {
                out.insert(asset.catalog_item_id, info);
            }
            Ok(None) => {}
            Err(e) => {
                failures += 1;
                if failures <= 5 {
                    eprintln!(
                        "[Epic] catalog {} ({}) FAILED: {e:?}",
                        asset.app_name, asset.namespace
                    );
                }
            }
        }
    }
    if failures > 0 {
        eprintln!("[Epic] catalog lookups failed for {failures} items");
    }
    out
}

/// Remove an Epic copy imported as a placeholder game for a non-game record
/// (DLC, hidden content).  The game itself is dropped only when the copy was its
/// last one and it carries no user data.  Returns 1 if a game was removed.
fn remove_epic_placeholder(data: &mut crate::models::CartridgeData, app_name: &str) -> u32 {
    let Some(idx) = data.games.iter().position(|g| {
        g.copies
            .iter()
            .any(|c| c.platform == Platform::Epic && c.app_id == app_name)
    }) else {
        return 0;
    };

    let game = &mut data.games[idx];
    // Only touch entries that never got real metadata (title is still the app_name).
    if game.title != app_name {
        return 0;
    }
    game.copies
        .retain(|c| !(c.platform == Platform::Epic && c.app_id == app_name));

    let untouched = game.copies.is_empty()
        && game.rating == 0
        && game.tags.is_empty()
        && game.note.is_empty()
        && game.custom.is_empty()
        && matches!(game.status, GameStatus::Backlog);
    let id = game.id;
    let in_list = data
        .lists
        .iter()
        .any(|l| l.game_ids.as_ref().is_some_and(|ids| ids.contains(&id)));

    if untouched && !in_list {
        data.games.remove(idx);
        1
    } else {
        0
    }
}

async fn sync_epic(state: Db<'_>, account_id: String) -> Result<SyncResult, String> {
    eprintln!("[Epic] sync starting for account {account_id}");

    let mut egs = build_epic_client(&state, &account_id).await?;

    // Fetch all library records — egs-api paginates internally
    eprintln!("[Epic] fetching library via egs-api");
    let library = egs
        .library_items(true)
        .await
        .ok_or_else(|| "Epic library fetch failed — please try again".to_string())?;

    let all_records = library.records;
    eprintln!(
        "[Epic] library fetch complete — {} records total",
        all_records.len()
    );

    // Skip Unreal Engine asset namespace and records with no namespace
    let game_records: Vec<_> = all_records
        .iter()
        .filter(|r| !r.namespace.is_empty() && r.namespace != "ue")
        .collect();

    // Catalog lookup for title, developer, year, cover images, categories.
    // Games that already have a real title and a cover don't need it again —
    // only new or incomplete records are looked up.
    let to_lookup: Vec<&EgsRecord> = {
        let store = state.lock().unwrap();
        game_records
            .iter()
            .copied()
            .filter(|r| {
                !store.data.games.iter().any(|g| {
                    g.cover.is_some()
                        && g.title != r.app_name
                        && g.copies
                            .iter()
                            .any(|c| c.platform == Platform::Epic && c.app_id == r.app_name)
                })
            })
            .collect()
    };
    eprintln!(
        "[Epic] fetching catalog metadata for {} items ({} already complete)",
        to_lookup.len(),
        game_records.len() - to_lookup.len()
    );
    let catalog = fetch_epic_catalog(&egs, &to_lookup).await;
    eprintln!(
        "[Epic] catalog fetch complete — {} of {} items",
        catalog.len(),
        to_lookup.len()
    );

    // Capture updated session after API calls so refresh token changes are persisted
    let updated_session_json = serde_json::to_string(&egs.user_details()).ok();

    let (mut added, mut updated, mut removed) = (0u32, 0u32, 0u32);
    let mut cover_queue: Vec<(String, String)> = Vec::new();

    let covers_dir = {
        let mut store = state.lock().unwrap();

        for record in &game_records {
            let info = catalog.get(record.catalog_item_id.as_str());

            // Skip non-game catalog items (DLC, add-ons, hidden content, tools)
            // when category data is available.  Earlier syncs without catalog
            // data imported these as placeholder games — drop those leftovers.
            let is_non_game = info
                .and_then(|i| i.categories.as_ref())
                .is_some_and(|cats| {
                    !cats.is_empty() && !cats.iter().any(|c| c.path.starts_with("games"))
                });
            if is_non_game {
                removed += remove_epic_placeholder(&mut store.data, &record.app_name);
                continue;
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

            // Covers are 3:4 portrait — prefer tall art, fall back to the wide box.
            let cover_url = info.and_then(|i| {
                let images = i.key_images.as_ref()?;
                [
                    "DieselGameBoxTall",
                    "OfferImageTall",
                    "Thumbnail",
                    "DieselGameBox",
                ]
                .iter()
                .find_map(|t| images.iter().find(|img| img.type_field == *t))
                .map(|img| {
                    let mut url = img.url.clone();
                    // Epic's CDN resizes on request; originals can be several MB.
                    if url.query().is_none() {
                        url.set_query(Some("w=480&h=640&resize=1"));
                    }
                    url.to_string()
                })
            });

            // First pass: look for existing game with this Epic app_id in any copy
            let existing_by_app_id = store.data.games.iter_mut().find(|g| {
                g.copies
                    .iter()
                    .any(|c| c.platform == Platform::Epic && c.app_id == record.app_name)
            });

            if let Some(game) = existing_by_app_id {
                if let Some(copy) = game
                    .copies
                    .iter_mut()
                    .find(|c| c.platform == Platform::Epic && c.app_id == record.app_name)
                {
                    copy.account_id = Some(account_id.clone());
                }
                // Replace the app_name placeholder from a sync without catalog data.
                if game.title == record.app_name && title != record.app_name {
                    game.title = title.clone();
                }
                if game.year == 0 && year > 0 {
                    game.year = year;
                }
                if game.dev.is_empty() && !dev.is_empty() {
                    game.dev = dev.clone();
                }
                updated += 1;
            } else {
                // Second pass: fuzzy match by normalized title
                let normalized = normalize_title(&title);
                let fuzzy_match = store
                    .data
                    .games
                    .iter_mut()
                    .find(|g| normalize_title(&g.title) == normalized);

                if let Some(game) = fuzzy_match {
                    game.copies.push(GameCopy {
                        platform: Platform::Epic,
                        app_id: record.app_name.clone(),
                        account_id: Some(account_id.clone()),
                        installed: false,
                        playtime: None,
                        last_played_at: None,
                    });
                    if game.year == 0 && year > 0 {
                        game.year = year;
                    }
                    if game.dev.is_empty() && !dev.is_empty() {
                        game.dev = dev.clone();
                    }
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
                        user_score_reviews: None,
                        metacritic_checked_at: None,
                        steam_match: None,
                        steam_match_checked_at: None,
                        steam_match_manual: false,
                        steam_match_rev: 0,
                        links: Default::default(),
                        custom: vec![],
                        copies: vec![GameCopy {
                            platform: Platform::Epic,
                            app_id: record.app_name.clone(),
                            account_id: Some(account_id.clone()),
                            installed: false,
                            playtime: None,
                            last_played_at: None,
                        }],
                    });
                    added += 1;
                }
            }

            if let Some(url) = cover_url {
                cover_queue.push((record.app_name.clone(), url));
            }
        }

        eprintln!("[Epic] merge complete — added {added}, updated {updated}, removed {removed} non-game leftovers");

        let now = chrono::Utc::now().format("%b %d, %Y").to_string();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.synced = now;
            acc.status = AccountStatus::Ok;
            acc.count = added + updated;
            if let Some(json) = updated_session_json {
                acc.egs_session = Some(json);
            }
        }
        let merged = merge_duplicate_games(&mut store.data);
        if merged > 0 {
            eprintln!("[Epic] merged {merged} cross-platform duplicates");
        }
        store.save().map_err(|e| e.to_string())?;
        store.covers_dir.clone()
    };

    eprintln!("[Epic] downloading covers for {} games", cover_queue.len());

    // Download in parallel; files already on disk (from an earlier, possibly
    // interrupted sync) are just linked.  The store is saved once at the end
    // rather than after every cover.
    const COVER_CONCURRENCY: usize = 8;
    let client = reqwest::Client::new();
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(COVER_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();

    for (app_name, url) in cover_queue {
        let cover_path = covers_dir.join(format!("epic_{app_name}.jpg"));
        let client = client.clone();
        let sem = std::sync::Arc::clone(&sem);
        tasks.spawn(async move {
            if cover_path.exists() {
                return (app_name, cover_path, Ok(()));
            }
            let _permit = sem.acquire_owned().await;
            let res = download_cover(&client, &url, &cover_path).await;
            (app_name, cover_path, res)
        });
    }

    let mut errors: Vec<String> = Vec::new();
    let mut linked: Vec<(String, String)> = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        let Ok((app_name, cover_path, res)) = joined else {
            continue;
        };
        match res {
            Ok(()) => linked.push((app_name, cover_path.to_string_lossy().into_owned())),
            Err(e) => {
                eprintln!("[Epic] cover {app_name} FAILED: {e}");
                errors.push(format!("cover {app_name}: {e}"));
            }
        }
    }

    {
        let mut store = state.lock().unwrap();
        for (app_name, cover_str) in &linked {
            if let Some(game) = store.data.games.iter_mut().find(|g| {
                g.cover.is_none()
                    && g.copies
                        .iter()
                        .any(|c| c.platform == Platform::Epic && c.app_id == *app_name)
            }) {
                game.cover = Some(cover_str.clone());
            }
        }
        store.save().map_err(|e| e.to_string())?;
    }

    eprintln!(
        "[Epic] sync done — added {added}, updated {updated}, covers {}, errors {}",
        linked.len(),
        errors.len()
    );
    Ok(SyncResult {
        added,
        updated,
        errors,
    })
}

// ── GOG OAuth ──────────────────────────────────────────────────────────────

const GOG_CLIENT_ID: &str = "46899977096215655";
const GOG_CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";

// Response shapes

#[derive(serde::Deserialize)]
struct GogTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    user_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct GogUserData {
    username: Option<String>,
    #[serde(rename = "userId")]
    user_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GogProductsPage {
    total_pages: u32,
    products: Vec<GogProduct>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GogProduct {
    id: u64,
    title: String,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    is_movie: Option<bool>,
    #[serde(default)]
    release_date: Option<serde_json::Value>,
}

// GOG's auth server only accepts this exact registered redirect URI for client 46899977096215655.
// We open a Tauri WebviewWindow and intercept navigation to this URL instead of using a local
// HTTP server, because a dynamic localhost port would never match the registered URI.
const GOG_REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

// ── Token helpers ───────────────────────────────────────────────────────────

fn stripped(account: Account) -> Account {
    Account {
        api_key: None,
        access_token: None,
        refresh_token: None,
        egs_session: None,
        ..account
    }
}

/// Returns a valid GOG access token for `account_id`, refreshing it first if
/// it is within 5 minutes of expiry.  Saves updated tokens to the store.
async fn ensure_gog_token(state: &Db<'_>, account_id: &str) -> Result<String, String> {
    let (access_token, refresh_token, expires_at) = {
        let store = state.lock().unwrap();
        let acc = store
            .data
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| format!("account {account_id} not found"))?;
        (
            acc.access_token.clone(),
            acc.refresh_token.clone(),
            acc.expires_at,
        )
    };

    let now = chrono::Utc::now().timestamp();
    let still_valid =
        access_token.is_some() && expires_at.map(|exp| exp - now > 300).unwrap_or(false);

    if still_valid {
        return Ok(access_token.unwrap());
    }

    let refresh_token =
        refresh_token.ok_or("GOG session expired — please re-authenticate in Settings")?;

    eprintln!("[GOG] access token expired — refreshing with refresh_token");

    let client = reqwest::Client::new();
    let resp = client
        .get("https://auth.gog.com/token")
        .query(&[
            ("client_id", GOG_CLIENT_ID),
            ("client_secret", GOG_CLIENT_SECRET),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    eprintln!("[GOG] token refresh → HTTP {}", resp.status());

    let tokens: GogTokenResponse = resp
        .error_for_status()
        .map_err(|e| format!("token refresh failed — please re-authenticate: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let new_token = tokens.access_token.clone();
    let new_expires = now + tokens.expires_in as i64;
    let new_refresh = tokens.refresh_token.unwrap_or(refresh_token);

    eprintln!(
        "[GOG] token refreshed — new expiry in {}s",
        tokens.expires_in
    );

    {
        let mut store = state.lock().unwrap();
        if let Some(acc) = store.data.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.access_token = Some(tokens.access_token);
            acc.refresh_token = Some(new_refresh);
            acc.expires_at = Some(new_expires);
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
            .append_pair("client_id", GOG_CLIENT_ID)
            .append_pair("redirect_uri", GOG_REDIRECT_URI)
            .append_pair("response_type", "code")
            .append_pair("layout", "client2");
        url.to_string()
    };

    eprintln!("[GOG] opening auth window → {auth_url}");

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_nav = Arc::clone(&tx);

    let webview_url = WebviewUrl::External(
        auth_url
            .parse()
            .map_err(|e| format!("invalid auth URL: {e}"))?,
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
            if url.host_str() == Some("embed.gog.com") && url.path() == "/on_login_success" {
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
                        let _ = sender
                            .send(code.ok_or_else(|| "no code in GOG redirect URL".to_string()));
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
        .map_err(|_| "GOG login window was closed before sign-in completed".to_string())??;

    let _ = window.close();
    eprintln!("[GOG] auth code received — exchanging for tokens");

    let client = reqwest::Client::new();

    // Exchange authorization code for tokens
    let token_resp = client
        .get("https://auth.gog.com/token")
        .query(&[
            ("client_id", GOG_CLIENT_ID),
            ("client_secret", GOG_CLIENT_SECRET),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", GOG_REDIRECT_URI),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    eprintln!("[GOG] token exchange → HTTP {}", token_resp.status());

    let tokens: GogTokenResponse = token_resp
        .error_for_status()
        .map_err(|e| format!("token exchange failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("token parse error: {e}"))?;

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
        .send()
        .await
        .map_err(|e| e.to_string())?;

    eprintln!("[GOG] userData → HTTP {}", user_resp.status());

    let user: GogUserData = user_resp
        .error_for_status()
        .map_err(|e| format!("user data fetch failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("user data parse error: {e}"))?;

    eprintln!("[GOG] user: {:?} (id: {:?})", user.username, user.user_id);

    let handle = user
        .username
        .clone()
        .or_else(|| user.user_id.clone())
        .unwrap_or_else(|| "GOG account".to_string());

    let account_id = tokens
        .user_id
        .clone()
        .or_else(|| user.user_id.clone())
        .unwrap_or_else(|| "gog_local".to_string());

    let mut store = state.lock().unwrap();

    let result = match store.data.accounts.iter_mut().find(|a| a.id == account_id) {
        Some(existing) => {
            existing.handle = handle;
            existing.access_token = Some(tokens.access_token);
            // Preserve the previously stored refresh token when the server doesn't
            // issue a new one (shouldn't happen on a fresh auth_code exchange, but
            // is defensive for future refresh-token rotations).
            if let Some(rt) = tokens.refresh_token {
                existing.refresh_token = Some(rt);
            }
            existing.expires_at = Some(expires_at);
            existing.status = AccountStatus::Ok;
            stripped(existing.clone())
        }
        None => {
            let account = Account {
                id: account_id.clone(),
                name: "GOG".into(),
                handle,
                platform: Platform::Gog,
                count: 0,
                synced: "never".into(),
                status: AccountStatus::Ok,
                api_key: None,
                access_token: Some(tokens.access_token),
                refresh_token: tokens.refresh_token,
                expires_at: Some(expires_at),
                egs_session: None,
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
        Platform::Gog => gog_oauth_flow(state, app).await,
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
        Ok(t) => {
            eprintln!("[GOG] access token OK");
            t
        }
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
            .send()
            .await
            .map_err(|e| e.to_string())?;

        eprintln!("[GOG] library page {page} → HTTP {}", raw.status());

        let resp: GogProductsPage = raw
            .error_for_status()
            .map_err(|e| format!("library fetch failed: {e}"))?
            .json()
            .await
            .map_err(|e| format!("library parse failed: {e}"))?;

        let total = resp.total_pages;
        let count = resp.products.len();
        // Filter out movies; mediaType=1 already targets games but double-check
        all_products.extend(
            resp.products
                .into_iter()
                .filter(|p| p.is_movie != Some(true)),
        );

        eprintln!("[GOG] page {page}/{total} — {count} products");

        if page >= total {
            break;
        }
        page += 1;
    }

    eprintln!(
        "[GOG] library fetch complete — {} games total",
        all_products.len()
    );

    let (mut added, mut updated) = (0u32, 0u32);
    let mut cover_queue: Vec<(String, String)> = Vec::new(); // (app_id_str, url)

    let covers_dir = {
        let mut store = state.lock().unwrap();

        for p in &all_products {
            let app_id_str = p.id.to_string();
            let year = gog_year(&p.release_date);

            // First pass: look for existing game with this GOG app_id in any copy
            let existing_by_app_id = store.data.games.iter_mut().find(|g| {
                g.copies
                    .iter()
                    .any(|c| c.platform == Platform::Gog && c.app_id == app_id_str)
            });

            if let Some(game) = existing_by_app_id {
                // Update GOG copy metadata if available
                if let Some(copy) = game
                    .copies
                    .iter_mut()
                    .find(|c| c.platform == Platform::Gog && c.app_id == app_id_str)
                {
                    copy.account_id = Some(account_id.clone());
                }
                // Update game-level metadata if missing
                if game.year == 0 && year > 0 {
                    game.year = year;
                }
                updated += 1;
            } else {
                // Second pass: try fuzzy match by normalized title
                let normalized = normalize_title(&p.title);
                let fuzzy_match = store
                    .data
                    .games
                    .iter_mut()
                    .find(|g| normalize_title(&g.title) == normalized);

                if let Some(game) = fuzzy_match {
                    // Add GOG copy to existing game
                    game.copies.push(GameCopy {
                        platform: Platform::Gog,
                        app_id: app_id_str.clone(),
                        account_id: Some(account_id.clone()),
                        installed: false,
                        playtime: None,
                        last_played_at: None,
                    });
                    // Update game-level metadata if missing
                    if game.year == 0 && year > 0 {
                        game.year = year;
                    }
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
                        user_score_reviews: None,
                        metacritic_checked_at: None,
                        steam_match: None,
                        steam_match_checked_at: None,
                        steam_match_manual: false,
                        steam_match_rev: 0,
                        links: Default::default(),
                        custom: vec![],
                        copies: vec![GameCopy {
                            platform: Platform::Gog,
                            app_id: app_id_str.clone(),
                            account_id: Some(account_id.clone()),
                            installed: false,
                            playtime: None,
                            last_played_at: None,
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
            acc.count = all_products.len() as u32;
        }
        let merged = merge_duplicate_games(&mut store.data);
        if merged > 0 {
            eprintln!("[GOG] merged {merged} cross-platform duplicates");
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
                // Find game with this GOG app_id in any copy; keep a cover it
                // already has from another platform.
                if let Some(game) = store.data.games.iter_mut().find(|g| {
                    g.copies
                        .iter()
                        .any(|c| c.platform == Platform::Gog && c.app_id == *app_id_str)
                }) {
                    if game.cover.is_none() {
                        game.cover = Some(cover_str);
                    }
                }
                store.save().ok();
            }
            Err(e) => {
                eprintln!("[GOG] cover {app_id_str} FAILED: {e}");
                errors.push(format!("cover {app_id_str}: {e}"));
            }
        }
    }

    eprintln!(
        "[GOG] sync done — added {added}, updated {updated}, errors {}",
        errors.len()
    );
    Ok(SyncResult {
        added,
        updated,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CartridgeData;

    fn game(id: u64, title: &str, platform: Platform, app_id: &str) -> Game {
        Game {
            id,
            title: title.into(),
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
            user_score_reviews: None,
            metacritic_checked_at: None,
            steam_match: None,
            steam_match_checked_at: None,
            steam_match_manual: false,
            steam_match_rev: 0,
            links: Default::default(),
            custom: vec![],
            copies: vec![GameCopy {
                platform,
                app_id: app_id.into(),
                account_id: None,
                installed: false,
                playtime: None,
                last_played_at: None,
            }],
        }
    }

    fn data(games: Vec<Game>, list_ids: Vec<u64>) -> CartridgeData {
        let mut d: CartridgeData = serde_json::from_value(serde_json::json!({
            "accounts": [], "games": [], "genres": [], "statuses": [],
            "lists": [{ "id": "l1", "name": "Favs", "color": "green", "kind": "manual",
                        "icon": "star", "gameIds": list_ids }],
        }))
        .unwrap();
        d.games = games;
        d
    }

    #[test]
    fn normalize_ignores_punctuation_symbols_and_case() {
        assert_eq!(normalize_title("XCOM® 2"), normalize_title("XCOM 2"));
        assert_eq!(
            normalize_title("Amnesia:  The Dark Descent"),
            normalize_title("Amnesia: The Dark Descent")
        );
        assert_eq!(
            normalize_title("Director’s Cut"),
            normalize_title("Director's Cut")
        );
        assert_eq!(
            normalize_title("Sword & Sworcery"),
            normalize_title("Sword and Sworcery")
        );
        assert_eq!(normalize_title("LIMBO"), normalize_title("Limbo"));
        // Editions stay distinct on purpose.
        assert_ne!(
            normalize_title("Tomb Raider"),
            normalize_title("Tomb Raider: Anniversary")
        );
    }

    #[test]
    fn merges_same_game_across_platforms() {
        let mut steam = game(1, "XCOM 2", Platform::Steam, "268500");
        steam.cover = Some("steam.jpg".into());
        let mut gog = game(2, "XCOM® 2", Platform::Gog, "1482002159");
        gog.rating = 4;
        gog.tags = vec!["strategy".into()];
        gog.cover = Some("gog.jpg".into());
        let epic = game(3, "XCOM 2", Platform::Epic, "3be3c4d6");
        let other = game(4, "Psychonauts", Platform::Steam, "3830");
        let mut d = data(vec![steam, gog, epic, other], vec![3, 2]);

        assert_eq!(merge_duplicate_games(&mut d), 2);
        assert_eq!(d.games.len(), 2);
        // The entry with user data survives and keeps everything.
        let xcom = d.games.iter().find(|g| g.id == 2).unwrap();
        assert_eq!(xcom.copies.len(), 3);
        assert_eq!(xcom.rating, 4);
        assert_eq!(xcom.cover.as_deref(), Some("gog.jpg"));
        // List memberships are remapped and deduplicated.
        assert_eq!(d.lists[0].game_ids.as_deref(), Some(&[2u64][..]));
        // Idempotent.
        assert_eq!(merge_duplicate_games(&mut d), 0);
    }

    #[test]
    fn merge_keeps_last_played_at() {
        let mut steam = game(1, "Hades", Platform::Steam, "1145360");
        steam.copies[0].last_played_at = Some(1_700_000_000);
        let mut gog = game(2, "Hades", Platform::Gog, "2073486541");
        gog.rating = 5;
        let mut d = data(vec![steam, gog], vec![]);

        assert_eq!(merge_duplicate_games(&mut d), 1);
        let hades = &d.games[0];
        assert_eq!(hades.id, 2);
        let steam_copy = hades
            .copies
            .iter()
            .find(|c| c.platform == Platform::Steam)
            .unwrap();
        assert_eq!(steam_copy.last_played_at, Some(1_700_000_000));
    }

    #[test]
    fn merge_takes_newer_session_for_the_same_copy() {
        let mut a = game(1, "Hades", Platform::Steam, "1145360");
        a.rating = 3;
        a.copies[0].last_played_at = Some(100);
        let mut b = game(2, "Hades", Platform::Steam, "1145360");
        b.copies[0].last_played_at = Some(200);
        let mut d = data(vec![a, b], vec![]);

        assert_eq!(merge_duplicate_games(&mut d), 1);
        let copies = &d.games[0].copies;
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].last_played_at, Some(200));
    }

    #[test]
    fn copy_without_last_played_at_deserializes() {
        let c: GameCopy = serde_json::from_value(serde_json::json!({
            "platform": "steam", "appId": "620", "installed": true,
            "playtime": 12.5,
        }))
        .unwrap();
        assert_eq!(c.last_played_at, None);
        let v = serde_json::to_value(&c).unwrap();
        assert!(v.get("lastPlayedAt").is_some());
    }

    #[test]
    fn never_merges_epic_placeholders() {
        let real = game(1, "Anemone", Platform::Steam, "111");
        let placeholder = game(2, "Anemone", Platform::Epic, "Anemone");
        let mut d = data(vec![real, placeholder], vec![]);
        assert_eq!(merge_duplicate_games(&mut d), 0);
        assert_eq!(d.games.len(), 2);
    }
}
