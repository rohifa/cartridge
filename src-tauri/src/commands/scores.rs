//! Scores from Steam: user reviews (% positive) and Metacritic (Metascore).
//!
//! Games owned on Steam use their own app IDs. Games owned only on GOG/Epic are
//! matched to a Steam app by title (`Game::steam_match`) and then get the same
//! scores. Everything runs in one paced background job (`spawn_score_refresh`):
//!
//!   1. title matching  — `IStoreQueryService/SearchSuggestions`, 1 per game, once
//!   2. user reviews    — `IStoreBrowseService/GetItems`, batched, every run
//!   3. Metacritic      — `store/api/appdetails`, 1 per game, every 30 days
//!
//! None of these need an API key. (`store.steampowered.com/appreviews` now
//! redirects to the store front page and `ISteamApps/GetAppList` is gone.)

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Context;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::models::{Game, Platform, SteamMatch};
use crate::store::AppStore;

/// Steam shows no percentage below this many reviews; neither do we.
pub(crate) const MIN_REVIEWS: u32 = 10;
const RECHECK_SECS: i64 = 30 * 24 * 3600;
const BACKOFF: Duration = Duration::from_secs(60);
const MAX_BACKOFFS: u32 = 3;
const SAVE_EVERY: u32 = 10;
/// Frontend event carrying `Vec<Game>` (full, updated entries).
const EVENT: &str = "games-updated";

/// Steam app IDs to take scores from: owned Steam copies first, then the
/// title match (only ever set for games with no Steam copy).
fn steam_appids(g: &Game) -> Vec<u64> {
    let mut ids: Vec<u64> = g
        .copies
        .iter()
        .filter(|c| c.platform == Platform::Steam)
        .filter_map(|c| c.app_id.parse().ok())
        .collect();
    if let Some(m) = &g.steam_match {
        if !ids.contains(&m.appid) { ids.push(m.appid); }
    }
    ids
}

fn has_steam_copy(g: &Game) -> bool {
    g.copies.iter().any(|c| c.platform == Platform::Steam)
}

fn due(checked_at: Option<i64>, now: i64) -> bool {
    checked_at.is_none_or(|t| now - t >= RECHECK_SECS)
}

// ───────────────────────────── HTTP helpers ─────────────────────────────

enum FetchError {
    RateLimited,
    Other(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for FetchError {
    fn from(e: E) -> Self { FetchError::Other(e.into()) }
}

async fn get_json(req: reqwest::RequestBuilder) -> Result<serde_json::Value, FetchError> {
    let resp = req.send().await?;
    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status == reqwest::StatusCode::FORBIDDEN {
        return Err(FetchError::RateLimited);
    }
    Ok(resp.error_for_status()?.json().await?)
}

/// Wait `delay`, run `f`; on rate limiting wait `BACKOFF` and retry, up to
/// `MAX_BACKOFFS` times. A final `RateLimited` means: stop this phase.
async fn paced<T, F, Fut>(delay: Duration, mut f: F) -> Result<T, FetchError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, FetchError>>,
{
    let mut backoffs = 0;
    loop {
        tokio::time::sleep(delay).await;
        match f().await {
            Err(FetchError::RateLimited) if backoffs < MAX_BACKOFFS => {
                backoffs += 1;
                eprintln!("[Scores] rate limited, waiting {}s", BACKOFF.as_secs());
                tokio::time::sleep(BACKOFF).await;
            }
            r => return r,
        }
    }
}

// ───────────────────────────── 1. Title matching ─────────────────────────────

const SEARCH_URL: &str = "https://api.steampowered.com/IStoreQueryService/SearchSuggestions/v1/";
const SEARCH_DELAY: Duration = Duration::from_millis(1000);

/// Trailing edition labels that differ between stores for the same game
/// ("…: Game of the Year Edition" on GOG, "… Game of the Year" on Steam).
/// Longer phrases first where one ends with another.
const EDITION_SUFFIXES: &[&str] = &[
    "game of the year edition", "game of the year", "goty edition", "goty",
    "definitive edition", "enhanced edition", "complete edition", "ultimate edition",
    "digital deluxe edition", "deluxe edition", "gold edition", "premium edition",
    "special edition", "anniversary edition", "standard edition", "collector's edition",
    "hd edition", "remastered", "remaster", "director's cut", "director\u{2019}s cut",
    "the final cut", "final cut", "complete", "hd",
];

/// Roman numerals II–IX as whole words → digits ("III" == "3").
/// I and X are left alone ("I Am Bread", "Mega Man X").
fn numeral(word: &str) -> Option<&'static str> {
    Some(match word {
        "ii" => "2", "iii" => "3", "iv" => "4", "v" => "5",
        "vi" => "6", "vii" => "7", "viii" => "8", "ix" => "9",
        _ => return None,
    })
}

/// Lowercase alphanumerics only, `&` = "and", roman numerals as digits.
fn normalize(s: &str) -> String {
    s.replace('&', " and ")
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| numeral(w).unwrap_or(w))
        .collect()
}

/// Title without ™/® and without trailing edition labels; used as search term.
pub(crate) fn strip_edition(title: &str) -> String {
    let mut s: String = title.chars().filter(|c| !matches!(c, '™' | '®' | '©')).collect();
    s = s.trim().to_string();
    loop {
        let t = s.trim_end_matches(|c: char| c == ')' || c.is_whitespace());
        let cut = EDITION_SUFFIXES.iter().find_map(|suf| {
            let start = t.len().checked_sub(suf.len())?;
            let ok = t.is_char_boundary(start)
                && t[start..].eq_ignore_ascii_case(suf)
                && t[..start].ends_with(|c: char| !c.is_alphanumeric());
            ok.then_some(start)
        });
        let Some(start) = cut else { break };
        let rest = t[..start]
            .trim_end_matches(|c: char| c.is_whitespace() || "-:–—,(".contains(c))
            .to_string();
        if rest.is_empty() { break; }
        s = rest;
    }
    s
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Candidate {
    pub appid: u64,
    pub name: String,
    pub reviews: u32,
}

/// Pick the Steam app for `title` among search results: an exact (normalized)
/// title match wins; otherwise the same title once edition labels are dropped
/// on both sides. Anything looser is rejected — no score beats a wrong one.
/// Ties go to the app with most reviews, i.e. the main listing
/// (HoMM3 HD Edition, 20k reviews, over a 2026 re-release with 268).
pub(crate) fn pick_match(title: &str, candidates: &[Candidate]) -> Option<SteamMatch> {
    let full = normalize(title);
    let base = normalize(&strip_edition(title));
    let best = |pred: &dyn Fn(&Candidate) -> bool| {
        candidates.iter().filter(|c| pred(c)).max_by_key(|c| c.reviews)
    };
    best(&|c| normalize(&c.name) == full)
        .or_else(|| best(&|c| normalize(&strip_edition(&c.name)) == base))
        .map(|c| SteamMatch { appid: c.appid, name: c.name.clone() })
}

async fn search_steam(client: &reqwest::Client, term: &str) -> Result<Vec<Candidate>, FetchError> {
    let input = serde_json::json!({
        "search_term": term,
        "max_results": 10,
        "context": { "language": "english", "country_code": "US" },
        "data_request": { "include_reviews": true },
    });
    let v = get_json(client.get(SEARCH_URL).query(&[("input_json", input.to_string())])).await?;
    let items = v["response"]["store_items"].as_array().cloned().unwrap_or_default();
    Ok(items
        .iter()
        .filter(|i| i["success"].as_i64() == Some(1))
        .filter_map(|i| Some(Candidate {
            appid: i["id"].as_u64()?,
            name: i["name"].as_str()?.to_string(),
            reviews: i["reviews"]["summary_filtered"]["review_count"].as_u64().unwrap_or(0) as u32,
        }))
        .collect())
}

/// Match GOG/Epic-only games to Steam apps. Returns how many got a match.
async fn match_titles(app: &AppHandle, client: &reqwest::Client, now: i64) -> anyhow::Result<u32> {
    let state = app.state::<Mutex<AppStore>>();
    let todo: Vec<(u64, String)> = {
        let store = state.lock().unwrap();
        store.data.games.iter()
            .filter(|g| !has_steam_copy(g) && !g.steam_match_manual && g.steam_match.is_none()
                && due(g.steam_match_checked_at, now))
            .map(|g| (g.id, g.title.clone()))
            .collect()
    };
    if todo.is_empty() { return Ok(0); }
    eprintln!("[Scores] matching {} titles on Steam", todo.len());

    let (mut matched, mut unsaved) = (0u32, 0u32);
    for (id, title) in todo {
        let term = strip_edition(&title);
        let candidates = match paced(SEARCH_DELAY, || search_steam(client, &term)).await {
            Ok(c) => c,
            Err(FetchError::RateLimited) => {
                if unsaved > 0 { state.lock().unwrap().save()?; }
                anyhow::bail!("title matching: still rate limited after {MAX_BACKOFFS} waits");
            }
            Err(FetchError::Other(e)) => { eprintln!("[Scores] search {title:?}: {e:#}"); continue; }
        };
        let found = pick_match(&title, &candidates);

        let mut store = state.lock().unwrap();
        // The entry may have been merged away by a sync meanwhile; next run redoes it.
        let Some(game) = store.data.games.iter_mut().find(|g| g.id == id) else { continue };
        game.steam_match_checked_at = Some(now);
        if let Some(m) = found {
            eprintln!("[Scores] {title:?} → {} ({})", m.name, m.appid);
            game.steam_match = Some(m);
            matched += 1;
        }
        unsaved += 1;
        if unsaved >= SAVE_EVERY { store.save()?; unsaved = 0; }
    }
    if unsaved > 0 { state.lock().unwrap().save()?; }
    Ok(matched)
}

// ───────────────────────────── 2. User reviews ─────────────────────────────

const GET_ITEMS_URL: &str = "https://api.steampowered.com/IStoreBrowseService/GetItems/v1/";
/// Apps per GetItems request (200 verified to work; 100 keeps URLs short).
const BATCH: usize = 100;

#[derive(serde::Deserialize)]
struct ItemsResponse {
    response: ItemsInner,
}
#[derive(serde::Deserialize)]
struct ItemsInner {
    #[serde(default)]
    store_items: Vec<StoreItem>,
}
#[derive(serde::Deserialize)]
struct StoreItem {
    id: u64,
    #[serde(default)]
    success: i32,
    #[serde(default)]
    name: String,
    reviews: Option<Reviews>,
}
#[derive(serde::Deserialize)]
struct Reviews {
    /// "All reviews" as on the store page (off-topic review bombs excluded).
    summary_filtered: Option<ReviewSummary>,
}
#[derive(serde::Deserialize)]
struct ReviewSummary {
    #[serde(default)]
    review_count: u32,
    #[serde(default)]
    percent_positive: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SteamReviews {
    pub percent: i32,
    pub count: u32,
}

impl StoreItem {
    fn reviews(&self) -> SteamReviews {
        let s = self.reviews.as_ref().and_then(|r| r.summary_filtered.as_ref());
        SteamReviews {
            percent: s.map_or(0, |s| s.percent_positive),
            count:   s.map_or(0, |s| s.review_count),
        }
    }
}

/// One GetItems request (≤ `BATCH` apps). Only apps Steam knows are returned.
async fn get_items(client: &reqwest::Client, appids: &[u64]) -> anyhow::Result<Vec<StoreItem>> {
    let input = serde_json::json!({
        "ids": appids.iter().map(|a| serde_json::json!({ "appid": a })).collect::<Vec<_>>(),
        "context": { "language": "english", "country_code": "US" },
        "data_request": { "include_reviews": true },
    });
    let resp: ItemsResponse = client
        .get(GET_ITEMS_URL)
        .query(&[("input_json", input.to_string())])
        .send()
        .await
        .context("request failed")?
        .error_for_status()?
        .json()
        .await
        .context("unexpected response")?;
    Ok(resp.response.store_items.into_iter().filter(|i| i.success == 1).collect())
}

/// Fetch review summaries for `appids`. Apps Steam doesn't know are left out;
/// known apps without reviews come back with `count == 0`.
async fn fetch_steam_reviews(
    client: &reqwest::Client,
    appids: &[u64],
) -> anyhow::Result<HashMap<u64, SteamReviews>> {
    let mut out = HashMap::new();
    for chunk in appids.chunks(BATCH) {
        for item in get_items(client, chunk).await? {
            out.insert(item.id, item.reviews());
        }
    }
    Ok(out)
}

/// Write Steam review scores into `user_score` / `user_score_reviews`.
/// A game with several Steam apps takes the one with the most reviews.
/// Games with no Steam app, or whose apps Steam didn't return, are untouched.
/// Returns the IDs of games whose values changed.
pub(crate) fn apply_steam_reviews(games: &mut [Game], reviews: &HashMap<u64, SteamReviews>) -> Vec<u64> {
    let mut changed = Vec::new();
    for g in games {
        let best = steam_appids(g)
            .iter()
            .filter_map(|id| reviews.get(id))
            .max_by_key(|r| r.count)
            .copied();
        let Some(r) = best else { continue };

        let score = (r.count >= MIN_REVIEWS).then_some(r.percent);
        if g.user_score != score || g.user_score_reviews != Some(r.count) {
            g.user_score = score;
            g.user_score_reviews = Some(r.count);
            changed.push(g.id);
        }
    }
    changed
}

/// Refresh Steam review scores for every game with a Steam app (owned or
/// matched). A few batched requests; never holds the lock across a request.
/// Returns the updated games.
pub(crate) async fn refresh_steam_reviews(
    store: &Mutex<AppStore>,
    client: &reqwest::Client,
) -> anyhow::Result<Vec<Game>> {
    let appids: Vec<u64> = {
        let store = store.lock().unwrap();
        let mut seen = HashSet::new();
        store.data.games.iter()
            .flat_map(steam_appids)
            .filter(|id| seen.insert(*id))
            .collect()
    };
    if appids.is_empty() {
        return Ok(vec![]);
    }

    let reviews = fetch_steam_reviews(client, &appids).await?;

    let mut store = store.lock().unwrap();
    let changed = apply_steam_reviews(&mut store.data.games, &reviews);
    if changed.is_empty() {
        return Ok(vec![]);
    }
    store.save()?;
    Ok(store.data.games.iter().filter(|g| changed.contains(&g.id)).cloned().collect())
}

// ───────────────────────────── 3. Metacritic ─────────────────────────────
//
// `appdetails?filters=metacritic` gives the Metascore and the Metacritic URL,
// ONE app per request, rate-limited (~200 requests / 5 min). Each game is
// checked once, then every `RECHECK_SECS`; `metacritic_checked_at` lets an
// interrupted run resume.

const MC_DELAY: Duration = Duration::from_millis(1600);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Metascore {
    pub score: i32,
    pub url: Option<String>,
}

/// Parse an appdetails response. Apps without a Metascore come back as
/// `"data": []` (an empty array), unknown apps as `"success": false`.
pub(crate) fn parse_metacritic(v: &serde_json::Value, appid: u64) -> Option<Metascore> {
    let entry = v.get(appid.to_string())?;
    if entry.get("success")?.as_bool() != Some(true) {
        return None;
    }
    let mc = entry.get("data")?.get("metacritic")?;
    let score = i32::try_from(mc.get("score")?.as_i64()?).ok()?;
    // Drop Steam's tracking query (`?ftag=…`).
    let url = mc.get("url").and_then(|u| u.as_str()).map(|u| u.split('?').next().unwrap_or(u).to_string());
    Some(Metascore { score, url })
}

async fn fetch_metacritic(client: &reqwest::Client, appid: u64) -> Result<Option<Metascore>, FetchError> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=metacritic");
    let v = get_json(client.get(&url)).await?;
    Ok(parse_metacritic(&v, appid))
}

/// Record a check result. A found score overwrites; "no score" keeps whatever
/// the game already had (never clears data). Returns true if anything changed
/// that the UI shows.
pub(crate) fn apply_metacritic(g: &mut Game, found: Option<&Metascore>, now: i64) -> bool {
    g.metacritic_checked_at = Some(now);
    let Some(mc) = found else { return false };
    let mut changed = false;
    if g.metacritic != Some(mc.score) {
        g.metacritic = Some(mc.score);
        changed = true;
    }
    if let Some(url) = &mc.url {
        if !g.links.contains_key("metacritic") {
            g.links.insert("metacritic".into(), url.clone());
            changed = true;
        }
    }
    changed
}

async fn refresh_metacritic(app: &AppHandle, client: &reqwest::Client, now: i64) -> anyhow::Result<u32> {
    let state = app.state::<Mutex<AppStore>>();

    // Keyed by Steam app IDs, not game IDs: a sync running meanwhile may merge
    // duplicates and change which entry holds a copy.
    let todo: Vec<Vec<u64>> = {
        let store = state.lock().unwrap();
        store.data.games.iter()
            .filter(|g| due(g.metacritic_checked_at, now))
            .map(steam_appids)
            .filter(|ids| !ids.is_empty())
            .collect()
    };
    if todo.is_empty() {
        return Ok(0);
    }
    eprintln!("[Scores] Metacritic: checking {} games", todo.len());

    let (mut checked, mut unsaved) = (0u32, 0u32);
    for appids in todo {
        // Try each Steam app until one has a Metascore.
        let mut found = None;
        let mut complete = true;
        for &appid in &appids {
            match paced(MC_DELAY, || fetch_metacritic(client, appid)).await {
                Ok(r) => found = r,
                Err(FetchError::RateLimited) => {
                    if unsaved > 0 { state.lock().unwrap().save()?; }
                    anyhow::bail!("Metacritic: still rate limited after {MAX_BACKOFFS} waits");
                }
                Err(FetchError::Other(e)) => {
                    eprintln!("[Scores] Metacritic app {appid}: {e:#}");
                    complete = false; // don't mark checked; retry next run
                }
            }
            if found.is_some() { break; }
        }
        if !complete && found.is_none() {
            continue;
        }

        let mut store = state.lock().unwrap();
        let Some(game) = store.data.games.iter_mut().find(|g| {
            steam_appids(g).iter().any(|id| appids.contains(id))
        }) else { continue };

        let snapshot = apply_metacritic(game, found.as_ref(), now).then(|| game.clone());
        checked += 1;
        unsaved += 1;
        if unsaved >= SAVE_EVERY {
            store.save()?;
            unsaved = 0;
        }
        drop(store);
        if let Some(g) = snapshot {
            app.emit(EVENT, vec![g]).ok();
        }
    }

    if unsaved > 0 {
        state.lock().unwrap().save()?;
    }
    Ok(checked)
}

// ───────────────────────────── Manual matching ─────────────────────────────

/// App ID from a Steam store / SteamDB / steam:// link, or a bare ID.
pub(crate) fn parse_steam_appid(input: &str) -> Option<u64> {
    let s = input.trim();
    let positive = |id: u64| (id > 0).then_some(id);
    if let Ok(id) = s.parse::<u64>() {
        return positive(id);
    }
    const MARKERS: [&str; 6] = [
        "/app/",              // store.steampowered.com/app/620/…, steamdb.info/app/620/
        "/apps/",             // cdn …/steam/apps/620/header.jpg
        "s.team/a/",          // short share links
        "steam://store/",
        "steam://run/",
        "steam://rungameid/",
    ];
    MARKERS.iter().find_map(|m| {
        let rest = &s[s.find(m)? + m.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        positive(digits.parse().ok()?)
    })
}

/// Drop every score that came from a Steam match. The Metacritic link goes
/// only if it is the one Steam gave us (old `/game/pc/` form), not one the
/// user typed in.
fn clear_steam_scores(g: &mut Game) {
    g.user_score = None;
    g.user_score_reviews = None;
    g.metacritic = None;
    g.metacritic_checked_at = None;
    if g.links.get("metacritic").is_some_and(|u| u.contains("metacritic.com/game/pc/")) {
        g.links.remove("metacritic");
    }
}

fn find_unowned(games: &mut [Game], id: u64) -> Result<&mut Game, String> {
    let game = games.iter_mut().find(|g| g.id == id).ok_or("Game not found.")?;
    if has_steam_copy(game) {
        return Err("This game is owned on Steam; its scores come from that copy.".into());
    }
    Ok(game)
}

/// Run `f` on game `id` (which must have no Steam copy), save, return it.
fn edit_unowned(
    state: &Mutex<AppStore>,
    id: u64,
    f: impl FnOnce(&mut Game),
) -> Result<Game, String> {
    let mut store = state.lock().unwrap();
    let game = find_unowned(&mut store.data.games, id)?;
    f(game);
    let updated = game.clone();
    store.save().map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Use the Steam app from a pasted link (or ID) for this game's scores.
/// Reviews are applied right away; Metacritic follows via the background job.
#[tauri::command]
pub async fn set_steam_match(
    app: AppHandle,
    state: State<'_, Mutex<AppStore>>,
    game_id: u64,
    input: String,
) -> Result<Game, String> {
    let appid = parse_steam_appid(&input)
        .ok_or("Not a Steam store link or app ID.")?;
    find_unowned(&mut state.lock().unwrap().data.games, game_id)?; // fail fast, before the request

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let item = get_items(&client, &[appid])
        .await
        .map_err(|e| format!("Couldn't reach Steam: {e:#}"))?
        .into_iter()
        .find(|i| i.id == appid)
        .ok_or_else(|| format!("Steam has no app {appid}."))?;

    let now = chrono::Utc::now().timestamp();
    let reviews = HashMap::from([(appid, item.reviews())]);
    let game = edit_unowned(state.inner(), game_id, |g| {
        clear_steam_scores(g);
        g.steam_match = Some(SteamMatch { appid, name: item.name.clone() });
        g.steam_match_manual = true;
        g.steam_match_checked_at = Some(now);
        apply_steam_reviews(std::slice::from_mut(g), &reviews);
    })?;
    spawn_score_refresh(app);
    Ok(game)
}

/// "Not this game": drop the match and its scores; never match automatically again.
#[tauri::command]
pub fn reject_steam_match(state: State<'_, Mutex<AppStore>>, game_id: u64) -> Result<Game, String> {
    edit_unowned(state.inner(), game_id, |g| {
        clear_steam_scores(g);
        g.steam_match = None;
        g.steam_match_manual = true;
    })
}

/// Undo a manual decision: forget it and let automatic matching search again.
#[tauri::command]
pub fn auto_steam_match(
    app: AppHandle,
    state: State<'_, Mutex<AppStore>>,
    game_id: u64,
) -> Result<Game, String> {
    let game = edit_unowned(state.inner(), game_id, |g| {
        clear_steam_scores(g);
        g.steam_match = None;
        g.steam_match_manual = false;
        g.steam_match_checked_at = None;
    })?;
    spawn_score_refresh(app);
    Ok(game)
}

// ───────────────────────────── Background job ─────────────────────────────

static RUNNING: AtomicBool = AtomicBool::new(false);
/// Set when a refresh is requested while one runs; the job then goes again,
/// so games added by a sync mid-run aren't left until the next launch.
static PENDING: AtomicBool = AtomicBool::new(false);

struct RunningGuard;
impl Drop for RunningGuard {
    fn drop(&mut self) { RUNNING.store(false, Ordering::SeqCst); }
}

/// Start the background score refresh, or queue another pass if one is running.
pub(crate) fn spawn_score_refresh(app: AppHandle) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        PENDING.store(true, Ordering::SeqCst);
        return;
    }
    tauri::async_runtime::spawn(async move {
        let _guard = RunningGuard;
        loop {
            PENDING.store(false, Ordering::SeqCst);
            refresh_all(&app).await;
            if !PENDING.load(Ordering::SeqCst) { break; }
        }
    });
}

/// Frontend entry point: kicks off the refresh and returns immediately.
#[tauri::command]
pub fn start_score_refresh(app: AppHandle) {
    spawn_score_refresh(app);
}

/// One pass of all three phases. A failing phase is logged and the next one
/// still runs; whatever didn't finish is picked up by the next pass.
async fn refresh_all(app: &AppHandle) {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(20)).build() {
        Ok(c) => c,
        Err(e) => return eprintln!("[Scores] client: {e}"),
    };
    let now = chrono::Utc::now().timestamp();

    match match_titles(app, &client, now).await {
        Ok(n) if n > 0 => eprintln!("[Scores] matched {n} titles"),
        Ok(_) => {}
        Err(e) => eprintln!("[Scores] {e:#}"),
    }
    match refresh_steam_reviews(app.state::<Mutex<AppStore>>().inner(), &client).await {
        Ok(games) if !games.is_empty() => { app.emit(EVENT, games).ok(); }
        Ok(_) => {}
        Err(e) => eprintln!("[Scores] user reviews: {e:#}"),
    }
    match refresh_metacritic(app, &client, now).await {
        Ok(n) if n > 0 => eprintln!("[Scores] Metacritic: {n} games checked"),
        Ok(_) => {}
        Err(e) => eprintln!("[Scores] {e:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GameCopy, GameStatus};

    fn game(copies: &[(Platform, &str)]) -> Game {
        Game {
            id: 1, title: "X".into(), cover: None, rating: 0, status: GameStatus::Backlog,
            tags: vec![], note: String::new(), genre: vec![], dev: String::new(), year: 0,
            metacritic: None, metacritic_checked_at: None,
            opencritic: None, user_score: None, user_score_reviews: None,
            steam_match: None, steam_match_checked_at: None, steam_match_manual: false,
            links: Default::default(), custom: vec![],
            copies: copies.iter().map(|(p, id)| GameCopy {
                platform: p.clone(), app_id: (*id).into(), account_id: None,
                installed: false, playtime: None, last: None,
            }).collect(),
        }
    }

    fn reviews(list: &[(u64, i32, u32)]) -> HashMap<u64, SteamReviews> {
        list.iter().map(|&(id, percent, count)| (id, SteamReviews { percent, count })).collect()
    }

    fn cands(list: &[(u64, &str)]) -> Vec<Candidate> {
        list.iter().map(|&(appid, n)| Candidate { appid, name: n.into(), reviews: 0 }).collect()
    }

    // ── matching ──

    #[test]
    fn strips_edition_labels() {
        assert_eq!(strip_edition("The Witcher 3: Wild Hunt - Game of the Year Edition"), "The Witcher 3: Wild Hunt");
        assert_eq!(strip_edition("Heroes of Might and Magic 3: Complete"), "Heroes of Might and Magic 3");
        assert_eq!(strip_edition("Heroes® of Might & Magic® III - HD Edition"), "Heroes of Might & Magic III");
        assert_eq!(strip_edition("DEATH STRANDING DIRECTOR'S CUT"), "DEATH STRANDING");
        assert_eq!(strip_edition("Tomb Raider (GOTY)"), "Tomb Raider");
        // Never strips to nothing, never cuts inside a word.
        assert_eq!(strip_edition("Complete"), "Complete");
        assert_eq!(strip_edition("Incomplete"), "Incomplete");
        assert_eq!(strip_edition("Hades"), "Hades");
    }

    #[test]
    fn exact_title_wins_over_edition_match() {
        let c = cands(&[(632470, "Disco Elysium - The Final Cut"), (1233220, "Disco Elysium - The Final Cut Soundtrack")]);
        assert_eq!(pick_match("Disco Elysium - The Final Cut", &c).unwrap().appid, 632470);
        let c = cands(&[(1850570, "DEATH STRANDING DIRECTOR'S CUT"), (1190460, "DEATH STRANDING")]);
        assert_eq!(pick_match("Death Stranding", &c).unwrap().appid, 1190460);
    }

    #[test]
    fn matches_across_edition_labels_and_numerals() {
        let c = cands(&[(391220, "Rise of the Tomb Raider™"), (203160, "Tomb Raider Game of the Year")]);
        assert_eq!(pick_match("Tomb Raider", &c).unwrap().appid, 203160);
        let c = cands(&[(3809850, "Heroes of Might and Magic III Remake"), (297000, "Heroes® of Might & Magic® III - HD Edition")]);
        assert_eq!(pick_match("Heroes of Might and Magic 3: Complete", &c).unwrap().appid, 297000);
        // Same match level → most reviews wins.
        let mut c = cands(&[(4921760, "Heroes Of Might And Magic III"), (297000, "Heroes® of Might & Magic® III - HD Edition")]);
        c[0].reviews = 268;
        c[1].reviews = 20_305;
        assert_eq!(pick_match("Heroes of Might and Magic 3: Complete", &c).unwrap().appid, 297000);
        let c = cands(&[(870780, "CONTROL Ultimate Edition")]);
        assert_eq!(pick_match("Control", &c).unwrap().name, "CONTROL Ultimate Edition");
    }

    #[test]
    fn rejects_sequels_and_extras() {
        let c = cands(&[(1145350, "Hades II"), (755800, "Hades' Star"), (1206340, "Hades Original Soundtrack")]);
        assert_eq!(pick_match("Hades", &c), None);
        let c = cands(&[(228280, "Baldur's Gate: Enhanced Edition")]);
        assert_eq!(pick_match("Baldur's Gate II: Enhanced Edition", &c), None);
        assert_eq!(pick_match("Fall Guys", &[]), None);
    }

    #[test]
    fn matched_app_feeds_scores_only_without_steam_copy() {
        let mut g = game(&[(Platform::Gog, "1207658930")]);
        g.steam_match = Some(SteamMatch { appid: 292030, name: "The Witcher 3".into() });
        assert_eq!(steam_appids(&g), vec![292030]);
        let mut games = vec![g];
        assert_eq!(apply_steam_reviews(&mut games, &reviews(&[(292030, 97, 800_000)])).len(), 1);
        assert_eq!(games[0].user_score, Some(97));
    }

    #[test]
    fn parses_steam_links_and_ids() {
        for (input, id) in [
            ("https://store.steampowered.com/app/292030/The_Witcher_3_Wild_Hunt/", 292030),
            ("store.steampowered.com/app/620", 620),
            ("  620  ", 620),
            ("https://steamdb.info/app/1145360/charts/", 1145360),
            ("https://s.team/a/413150", 413150),
            ("steam://store/632470", 632470),
            ("https://cdn.akamai.steamstatic.com/steam/apps/297000/header.jpg", 297000),
        ] {
            assert_eq!(parse_steam_appid(input), Some(id), "{input}");
        }
        for bad in ["", "0", "hades", "https://www.gog.com/en/game/hades", "https://store.steampowered.com/app/"] {
            assert_eq!(parse_steam_appid(bad), None, "{bad}");
        }
    }

    #[test]
    fn clearing_keeps_user_typed_metacritic_link() {
        let mut g = game(&[(Platform::Gog, "1")]);
        g.user_score = Some(90);
        g.metacritic = Some(88);
        g.links.insert("metacritic".into(), "https://www.metacritic.com/game/pc/hades".into());
        clear_steam_scores(&mut g);
        assert_eq!((g.user_score, g.metacritic), (None, None));
        assert!(!g.links.contains_key("metacritic"));

        g.links.insert("metacritic".into(), "https://www.metacritic.com/game/hades/".into());
        clear_steam_scores(&mut g);
        assert!(g.links.contains_key("metacritic"));
    }

    // ── reviews ──

    #[test]
    fn sets_percent_and_count() {
        let mut games = vec![game(&[(Platform::Steam, "620")])];
        assert_eq!(apply_steam_reviews(&mut games, &reviews(&[(620, 98, 393_276)])), vec![1]);
        assert_eq!(games[0].user_score, Some(98));
        assert_eq!(games[0].user_score_reviews, Some(393_276));
        // Same values again → nothing changed, no save needed.
        assert!(apply_steam_reviews(&mut games, &reviews(&[(620, 98, 393_276)])).is_empty());
    }

    #[test]
    fn too_few_reviews_has_no_percent() {
        let mut games = vec![game(&[(Platform::Steam, "10")])];
        apply_steam_reviews(&mut games, &reviews(&[(10, 100, 3)]));
        assert_eq!(games[0].user_score, None);
        assert_eq!(games[0].user_score_reviews, Some(3));
    }

    #[test]
    fn picks_steam_copy_with_most_reviews() {
        let mut games = vec![game(&[(Platform::Steam, "2130"), (Platform::Steam, "2100")])];
        apply_steam_reviews(&mut games, &reviews(&[(2130, 76, 51), (2100, 89, 9_431)]));
        assert_eq!(games[0].user_score, Some(89));
    }

    #[test]
    fn leaves_unmatched_and_unknown_games_alone() {
        let mut gog = game(&[(Platform::Gog, "620")]);
        gog.user_score = Some(50);
        let mut games = vec![gog, game(&[(Platform::Steam, "999")])];
        assert!(apply_steam_reviews(&mut games, &reviews(&[(620, 98, 1_000)])).is_empty());
        assert_eq!(games[0].user_score, Some(50));
        assert_eq!(games[1].user_score_reviews, None);
    }

    // ── Metacritic ──

    #[test]
    fn parses_appdetails_shapes() {
        let with = serde_json::json!({"620": {"success": true, "data": {"metacritic": {
            "score": 95, "url": "https://www.metacritic.com/game/pc/portal-2?ftag=MCD-06-10aaa1f"}}}});
        assert_eq!(parse_metacritic(&with, 620), Some(Metascore {
            score: 95, url: Some("https://www.metacritic.com/game/pc/portal-2".into()),
        }));
        // No Metascore: `data` is an empty array.
        let none = serde_json::json!({"3527290": {"success": true, "data": []}});
        assert_eq!(parse_metacritic(&none, 3527290), None);
        let unknown = serde_json::json!({"999999999": {"success": false}});
        assert_eq!(parse_metacritic(&unknown, 999999999), None);
    }

    #[test]
    fn metacritic_never_clears_and_keeps_existing_link() {
        let mut g = game(&[(Platform::Steam, "620")]);
        g.metacritic = Some(90);
        g.links.insert("metacritic".into(), "https://custom".into());
        assert!(!apply_metacritic(&mut g, None, 100));
        assert_eq!((g.metacritic, g.metacritic_checked_at), (Some(90), Some(100)));

        let mc = Metascore { score: 95, url: Some("https://mc/portal-2".into()) };
        assert!(apply_metacritic(&mut g, Some(&mc), 200));
        assert_eq!(g.metacritic, Some(95));
        assert_eq!(g.links["metacritic"], "https://custom");
    }

    #[test]
    fn recheck_window() {
        assert!(due(None, 1_000));
        assert!(!due(Some(1_000), 1_000 + RECHECK_SECS - 1));
        assert!(due(Some(1_000), 1_000 + RECHECK_SECS));
    }

    // ── live (real Steam API): `cargo test -- --ignored live_` ──

    #[tokio::test]
    #[ignore]
    async fn live_fetch_parses_real_response() {
        let client = reqwest::Client::new();
        let r = fetch_steam_reviews(&client, &[620, 999_999_999]).await.unwrap();
        let portal = r.get(&620).expect("Portal 2 present");
        assert!(portal.count > 100_000 && (90..=100).contains(&portal.percent));
        assert!(!r.contains_key(&999_999_999));
    }

    #[tokio::test]
    #[ignore]
    async fn live_metacritic() {
        let client = reqwest::Client::new();
        let Ok(Some(mc)) = fetch_metacritic(&client, 620).await else { panic!("no Metascore for Portal 2") };
        assert_eq!(mc.score, 95);
        assert!(matches!(fetch_metacritic(&client, 3527290).await, Ok(None)));
    }

    #[tokio::test]
    #[ignore]
    async fn live_matching() {
        let client = reqwest::Client::new();
        for (title, expected) in [
            ("The Witcher 3: Wild Hunt - Game of the Year Edition", Some(292030)),
            ("Disco Elysium - The Final Cut", Some(632470)),
            ("Heroes of Might and Magic 3: Complete", Some(297000)),
            ("Hades", Some(1145360)),
            ("Fall Guys", None),
        ] {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let Ok(c) = search_steam(&client, &strip_edition(title)).await else { panic!("search failed") };
            let got = pick_match(title, &c);
            println!("{title:?} → {got:?}");
            assert_eq!(got.map(|m| m.appid), expected, "{title}");
        }
    }
}
