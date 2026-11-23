use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use tauri::{AppHandle, Manager};

use crate::models::{CartridgeData, Game, GamePatch, GameStatus, Prefs, StatusMeta};

/// Library format written by this build. See `parse_library` for migrations.
pub const CURRENT_VERSION: u32 = 2;

/// Longest tag, in characters (`TAG_MAX` in `utils.ts`).
const TAG_MAX: usize = 32;
const DAY: i64 = 86_400;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Same rules as `normTag()` in `utils.ts`: trimmed, lowercase, single spaces,
/// at most `TAG_MAX` characters.
pub fn norm_tag(raw: &str) -> String {
    let joined = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower: String = joined.to_lowercase().chars().take(TAG_MAX).collect();
    lower.trim().to_string()
}

/// Normalised tags, empty ones dropped, duplicates removed (first one kept).
pub fn clean_tags(tags: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = norm_tag(&t);
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// Unix secs estimated from a legacy relative label ("today", "yesterday",
/// "3 days ago", "2 weeks ago", "5 months ago", "1 years ago"), snapped to the
/// start of a day. None for anything else.
fn estimate_last_played(label: &str, now: i64) -> Option<i64> {
    let days = match label.trim() {
        "today" => 0,
        "yesterday" => 1,
        other => {
            let mut parts = other.split_whitespace();
            let n: i64 = parts.next()?.parse().ok()?;
            let unit = parts.next()?;
            if parts.next()? != "ago" || parts.next().is_some() {
                return None;
            }
            let per = match unit.strip_suffix('s').unwrap_or(unit) {
                "day" => 1,
                "week" => 7,
                "month" => 30,
                "year" => 365,
                _ => return None,
            };
            n * per
        }
    };
    Some((now / DAY - days) * DAY)
}

pub struct AppStore {
    pub data: CartridgeData,
    pub path: PathBuf,
    pub covers_dir: PathBuf,
}

impl AppStore {
    pub fn load(app: &AppHandle) -> Self {
        let base = dirs::data_dir().unwrap_or_else(|| {
            app.path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
        });
        let app_dir = base.join("com.cartridge.app");
        std::fs::create_dir_all(&app_dir).ok();

        let path = app_dir.join("library.json");
        let covers_dir = app_dir.join("covers");
        std::fs::create_dir_all(&covers_dir).ok();

        let (data, needs_save) = Self::read_library(&path);

        let store = Self {
            data,
            path,
            covers_dir,
        };

        // First run: create the file. Migrated: write the new format once
        // (the original was copied aside by `read_library`).
        if needs_save {
            store.save().ok();
        }

        store
    }

    /// Reads the library at `path`. Returns `(data, needs_save)`: true on first
    /// run (no file yet) and after a migration.
    ///
    /// Before a migrated library is saved in the new format, the original is
    /// copied to `library.v<old version>.json` (once; an existing copy is kept).
    ///
    /// If the file exists but can't be read, parsed or migrated, the error is
    /// logged and the file is renamed to `library.corrupt-<unix secs>.json` next
    /// to it before falling back to empty data, so the next save can't
    /// overwrite the user's real library.
    fn read_library(path: &Path) -> (CartridgeData, bool) {
        if !path.exists() {
            return (Self::empty_data(), true);
        }

        let raw = std::fs::read_to_string(path).context("could not read file");
        let parsed = raw
            .and_then(|s| Self::parse_library(&s, now_secs()).map(|(data, from)| (data, from, s)));

        match parsed {
            Ok((data, from, _)) if from == CURRENT_VERSION => (data, false),
            Ok((data, from, original)) => {
                let copy = path.with_file_name(format!("library.v{from}.json"));
                if copy.exists() {
                    eprintln!("[Store] Keeping existing {}", copy.display());
                } else if let Err(e) = std::fs::write(&copy, original) {
                    // The migration loses nothing the app uses, so carry on.
                    eprintln!("[Store] Could not keep a copy at {}: {e}", copy.display());
                }
                eprintln!("[Store] Migrated library v{from} → v{CURRENT_VERSION}");
                (data, true)
            }
            Err(e) => {
                eprintln!("[Store] Could not load {}: {e:#}", path.display());
                Self::backup_unreadable(path);
                (Self::empty_data(), false)
            }
        }
    }

    /// Parses and migrates a library. Returns the data and the version the
    /// file was written in. Add new migrations here, oldest first.
    fn parse_library(s: &str, now: i64) -> anyhow::Result<(CartridgeData, u32)> {
        // Parse as a JSON Value first to check the version
        let mut value: serde_json::Value = serde_json::from_str(s).context("not valid JSON")?;
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        if version > CURRENT_VERSION {
            anyhow::bail!(
                "library v{version} was written by a newer Cartridge \
                 (this build reads up to v{CURRENT_VERSION})"
            );
        }
        if version < 1 {
            value = Self::migrate_v0_to_v1(value)?;
        }
        if version < 2 {
            Self::migrate_v1_to_v2(&mut value, now);
        }
        let data = serde_json::from_value(value).context("does not match the current format")?;
        Ok((data, version))
    }

    /// Moves an unreadable library aside as `library.corrupt-<unix secs>.json`.
    /// A failed rename is logged; loading continues with empty data either way.
    fn backup_unreadable(path: &Path) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = path.with_file_name(format!("library.corrupt-{secs}.json"));
        match std::fs::rename(path, &backup) {
            Ok(()) => eprintln!("[Store] Moved unreadable library to {}", backup.display()),
            Err(e) => eprintln!(
                "[Store] Could not back up {} to {}: {e}",
                path.display(),
                backup.display()
            ),
        }
    }

    /// Migrate v0 (old single-platform format) to v1 (multi-platform copies format)
    fn migrate_v0_to_v1(mut value: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        // Extract games array
        let games_array = value
            .get_mut("games")
            .and_then(|v| v.as_array_mut())
            .context("games field missing or not an array")?;

        // Transform each game
        for game_value in games_array.iter_mut() {
            if let Some(game_obj) = game_value.as_object_mut() {
                // Extract old platform-specific fields
                let platform = game_obj.remove("platform");
                let app_id = game_obj.remove("appId");
                let installed = game_obj
                    .remove("installed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let playtime = game_obj.remove("playtime");
                let last = game_obj.remove("last");

                // Create a single copy if we have platform info
                if let (Some(platform), Some(app_id)) = (platform, app_id) {
                    let copy = serde_json::json!({
                        "platform": platform,
                        "appId": app_id.as_str().unwrap_or(""),
                        "accountId": null,
                        "installed": installed,
                        "playtime": playtime,
                        "last": last,
                    });
                    game_obj.insert("copies".to_string(), serde_json::json!([copy]));
                } else {
                    // No platform info - create empty copies array
                    game_obj.insert("copies".to_string(), serde_json::json!([]));
                }
            }
        }

        // Set version to 1
        if let Some(obj) = value.as_object_mut() {
            obj.insert("version".to_string(), serde_json::json!(1));
        }

        Ok(value)
    }

    /// v1 → v2: tags normalised like `normTag()` (and deduplicated); the legacy
    /// relative `last` label on copies is dropped, after turning it into an
    /// estimated `lastPlayedAt` where the copy has no timestamp.
    fn migrate_v1_to_v2(value: &mut serde_json::Value, now: i64) {
        use serde_json::Value;

        let games = value.get_mut("games").and_then(|v| v.as_array_mut());
        for game in games.into_iter().flatten() {
            if let Some(tags) = game.get_mut("tags").and_then(|t| t.as_array_mut()) {
                let raw = tags.iter().filter_map(|t| t.as_str()).map(String::from);
                *tags = clean_tags(raw).into_iter().map(Value::String).collect();
            }
            let copies = game.get_mut("copies").and_then(|c| c.as_array_mut());
            for copy in copies
                .into_iter()
                .flatten()
                .filter_map(|c| c.as_object_mut())
            {
                let last = copy.remove("last");
                let has_ts = copy.get("lastPlayedAt").is_some_and(|v| !v.is_null());
                let estimate = last
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .and_then(|label| estimate_last_played(label, now));
                if let (false, Some(ts)) = (has_ts, estimate) {
                    copy.insert("lastPlayedAt".into(), serde_json::json!(ts));
                }
            }
        }
        if let Some(obj) = value.as_object_mut() {
            obj.insert("version".into(), serde_json::json!(2));
        }
    }

    fn empty_data() -> CartridgeData {
        CartridgeData {
            version: CURRENT_VERSION,
            accounts: vec![],
            games: vec![],
            lists: vec![],
            genres: vec![
                "Action".into(),
                "Adventure".into(),
                "Art".into(),
                "Card".into(),
                "Co-op".into(),
                "Cozy".into(),
                "Horror".into(),
                "Metroidvania".into(),
                "Mystery".into(),
                "Narrative".into(),
                "Open world".into(),
                "Platformer".into(),
                "Puzzle".into(),
                "Roguelike".into(),
                "RPG".into(),
                "Simulation".into(),
            ],
            statuses: vec![
                StatusMeta {
                    id: GameStatus::Backlog,
                    label: "Backlog".into(),
                    icon: "layers".into(),
                },
                StatusMeta {
                    id: GameStatus::Playing,
                    label: "Playing".into(),
                    icon: "play".into(),
                },
                StatusMeta {
                    id: GameStatus::Beaten,
                    label: "Beaten".into(),
                    icon: "check-check".into(),
                },
                StatusMeta {
                    id: GameStatus::Dropped,
                    label: "Dropped".into(),
                    icon: "x".into(),
                },
                StatusMeta {
                    id: GameStatus::Wishlist,
                    label: "Wishlist".into(),
                    icon: "bookmark".into(),
                },
            ],
            prefs: Prefs::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&self.data).context("serialise CartridgeData")?;
        std::fs::write(&tmp, json).context("write library.json.tmp")?;
        std::fs::rename(&tmp, &self.path).context("rename tmp → library.json")?;
        Ok(())
    }

    pub fn patch_game(&mut self, id: u64, patch: GamePatch) -> Option<Game> {
        let game = self.data.games.iter_mut().find(|g| g.id == id)?;

        if let Some(v) = patch.title {
            game.title = v;
        }
        if let Some(v) = patch.cover {
            game.cover = v;
        }
        if let Some(v) = patch.genre {
            game.genre = v;
        }
        if let Some(v) = patch.dev {
            game.dev = v;
        }
        if let Some(v) = patch.year {
            game.year = v;
        }
        if let Some(v) = patch.rating {
            game.rating = v;
        }
        if let Some(v) = patch.status {
            game.status = v;
        }
        if let Some(v) = patch.metacritic {
            game.metacritic = v;
        }
        if let Some(v) = patch.opencritic {
            game.opencritic = v;
        }
        if let Some(v) = patch.user_score {
            game.user_score = v;
        }
        if let Some(v) = patch.tags {
            game.tags = clean_tags(v);
        }
        if let Some(v) = patch.note {
            game.note = v;
        }
        if let Some(v) = patch.links {
            game.links = v;
        }
        if let Some(v) = patch.custom {
            game.custom = v;
        }

        let updated = game.clone();
        self.save().ok();
        Some(updated)
    }

    /// Returns a clone of CartridgeData with `api_key` stripped from all accounts.
    /// Always use this when returning data to the frontend.
    pub fn sanitized_data(&self) -> CartridgeData {
        let mut data = self.data.clone();
        for account in &mut data.accounts {
            account.api_key = None;
            account.access_token = None;
            account.refresh_token = None;
            account.egs_session = None;
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory under the system temp dir.
    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cartridge-store-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn backups(dir: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("library.corrupt-") && n.ends_with(".json"))
            })
            .collect()
    }

    fn assert_backed_up(name: &str, contents: &str) {
        let dir = temp_dir(name);
        let path = dir.join("library.json");
        std::fs::write(&path, contents).unwrap();

        let (data, first_run) = AppStore::read_library(&path);

        assert!(!first_run);
        assert!(data.games.is_empty());
        assert_eq!(data.version, CURRENT_VERSION);
        assert!(
            !path.exists(),
            "unreadable library.json should be moved aside"
        );
        let found = backups(&dir);
        assert_eq!(found.len(), 1, "expected one backup, found {found:?}");
        assert_eq!(std::fs::read_to_string(&found[0]).unwrap(), contents);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_json_is_backed_up() {
        assert_backed_up("invalid", "{ not json");
    }

    #[test]
    fn wrong_shape_is_backed_up() {
        assert_backed_up("shape", r#"{"version":1,"games":"oops"}"#);
    }

    #[test]
    fn valid_json_loads_without_backup() {
        let dir = temp_dir("valid");
        let path = dir.join("library.json");
        let mut original = AppStore::empty_data();
        original.genres = vec!["Puzzle".into()];
        std::fs::write(&path, serde_json::to_string(&original).unwrap()).unwrap();

        let (data, first_run) = AppStore::read_library(&path);

        assert!(!first_run);
        assert_eq!(data.genres, vec!["Puzzle".to_string()]);
        assert_eq!(data.statuses.len(), original.statuses.len());
        assert!(path.exists());
        assert!(backups(&dir).is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn newer_version_is_backed_up() {
        assert_backed_up("newer", r#"{"version":3,"games":[]}"#);
    }

    /// A v1 library as older builds wrote it: one game, the given tags and copies.
    fn v1_library(tags: serde_json::Value, copies: serde_json::Value) -> String {
        let mut v = serde_json::to_value(AppStore::empty_data()).unwrap();
        v["version"] = serde_json::json!(1);
        v["games"] = serde_json::json!([{
            "id": 1, "title": "Hades", "cover": null, "rating": 0, "status": "backlog",
            "tags": tags, "note": "", "genre": [], "dev": "", "year": 2020,
            "metacritic": null, "opencritic": null, "userScore": null,
            "links": {}, "custom": [], "copies": copies,
        }]);
        v.to_string()
    }

    const NOW: i64 = 1_763_900_000; // 2025-11-23, mid-day

    #[test]
    fn v1_tags_are_normalised_and_deduplicated() {
        let s = v1_library(
            serde_json::json!([" Co-Op ", "co-op", "Short   Sessions", "  ", "RPG"]),
            serde_json::json!([]),
        );
        let (data, from) = AppStore::parse_library(&s, NOW).unwrap();
        assert_eq!(from, 1);
        assert_eq!(data.version, CURRENT_VERSION);
        assert_eq!(data.games[0].tags, vec!["co-op", "short sessions", "rpg"]);
    }

    #[test]
    fn v1_last_label_becomes_a_timestamp_and_is_dropped() {
        let s = v1_library(
            serde_json::json!([]),
            serde_json::json!([
                { "platform": "steam", "appId": "1", "installed": true, "playtime": 1.0,
                  "last": "2 weeks ago" },
                { "platform": "steam", "appId": "2", "installed": true, "playtime": 1.0,
                  "last": "today", "lastPlayedAt": 1_700_000_000 },
                { "platform": "gog", "appId": "3", "installed": false, "playtime": null,
                  "last": "a while back" },
            ]),
        );
        let (data, _) = AppStore::parse_library(&s, NOW).unwrap();
        let copies = &data.games[0].copies;
        assert_eq!(copies[0].last_played_at, Some((NOW / DAY - 14) * DAY));
        assert_eq!(copies[1].last_played_at, Some(1_700_000_000));
        assert_eq!(copies[2].last_played_at, None);
        let saved = serde_json::to_value(&data).unwrap();
        assert!(saved["games"][0]["copies"][0].get("last").is_none());
    }

    #[test]
    fn v0_library_migrates_all_the_way() {
        let mut v = serde_json::to_value(AppStore::empty_data()).unwrap();
        v.as_object_mut().unwrap().remove("version");
        v["games"] = serde_json::json!([{
            "id": 1, "title": "Celeste", "cover": null, "rating": 4, "status": "beaten",
            "tags": ["Hard"], "note": "", "genre": [], "dev": "", "year": 2018,
            "metacritic": null, "opencritic": null, "userScore": null,
            "links": {}, "custom": [],
            "platform": "steam", "appId": "504230", "installed": true,
            "playtime": 30.0, "last": "yesterday",
        }]);
        let (data, from) = AppStore::parse_library(&v.to_string(), NOW).unwrap();
        assert_eq!(from, 0);
        let g = &data.games[0];
        assert_eq!(g.tags, vec!["hard"]);
        assert_eq!(g.copies.len(), 1);
        assert_eq!(g.copies[0].app_id, "504230");
        assert_eq!(g.copies[0].last_played_at, Some((NOW / DAY - 1) * DAY));
    }

    #[test]
    fn migrated_library_keeps_a_copy_of_the_original() {
        let dir = temp_dir("migrate");
        let path = dir.join("library.json");
        let original = v1_library(serde_json::json!(["RPG"]), serde_json::json!([]));
        std::fs::write(&path, &original).unwrap();

        let (data, needs_save) = AppStore::read_library(&path);

        assert!(needs_save);
        assert_eq!(data.games[0].tags, vec!["rpg"]);
        let copy = dir.join("library.v1.json");
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), original);
        assert!(backups(&dir).is_empty());

        // A second migration (e.g. the save failed) keeps the first copy.
        std::fs::write(&copy, "first").unwrap();
        AppStore::read_library(&path);
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), "first");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn estimates_legacy_labels() {
        let day0 = NOW / DAY * DAY;
        assert_eq!(estimate_last_played("today", NOW), Some(day0));
        assert_eq!(estimate_last_played("yesterday", NOW), Some(day0 - DAY));
        assert_eq!(
            estimate_last_played("3 days ago", NOW),
            Some(day0 - 3 * DAY)
        );
        assert_eq!(
            estimate_last_played("1 years ago", NOW),
            Some(day0 - 365 * DAY)
        );
        assert_eq!(
            estimate_last_played("1 month ago", NOW),
            Some(day0 - 30 * DAY)
        );
        assert_eq!(estimate_last_played("3 days", NOW), None);
        assert_eq!(estimate_last_played("3 fortnights ago", NOW), None);
        assert_eq!(estimate_last_played("", NOW), None);
    }

    #[test]
    fn norm_tag_matches_the_frontend() {
        assert_eq!(norm_tag("  Short\t  Sessions "), "short sessions");
        assert_eq!(norm_tag(&"x".repeat(40)), "x".repeat(32));
        // Cut at 32 chars, then trimmed (like `.slice(0, 32).trim()`).
        assert_eq!(norm_tag(&format!("{} y", "a".repeat(31))), "a".repeat(31));
        assert_eq!(clean_tags(["A".into(), "a".into(), " ".into()]), vec!["a"]);
    }

    #[test]
    fn patch_game_normalises_tags() {
        let dir = temp_dir("patch");
        let lib: serde_json::Value =
            serde_json::from_str(&v1_library(serde_json::json!([]), serde_json::json!([])))
                .unwrap();
        let game: Game = serde_json::from_value(lib["games"][0].clone()).unwrap();
        let mut data = AppStore::empty_data();
        data.games.push(game);
        let mut store = AppStore {
            data,
            path: dir.join("library.json"),
            covers_dir: dir.join("covers"),
        };

        let patch = GamePatch {
            tags: Some(vec!["Co-op".into(), " co-op ".into(), "Cozy".into()]),
            ..Default::default()
        };
        let g = store.patch_game(1, patch).unwrap();
        assert_eq!(g.tags, vec!["co-op", "cozy"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_is_first_run() {
        let dir = temp_dir("missing");
        let path = dir.join("library.json");

        let (data, first_run) = AppStore::read_library(&path);

        assert!(first_run);
        assert!(data.games.is_empty());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
