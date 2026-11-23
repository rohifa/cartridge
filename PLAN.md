# Cartridge — Implementation Plan

Translating the React prototype in `project/` into Svelte 5 (Runes API) within the Tauri + SvelteKit structure.

---

## Section 1 — Foundation ✅
- [x] Update `src/app.html` — fonts (Geist + Geist Mono via Google Fonts; later bundled in `static/fonts/`), `data-theme="dark"` on `<html>`, links to `design-tokens.css` + `global.css`
- [x] `static/design-tokens.css` — all CSS custom properties, dark + light themes
- [x] `static/global.css` — body reset, scrollbars, range slider, spin animation
- [x] `src/lib/types.ts` — TypeScript interfaces: `Game`, `Account`, `GameList`, `Filters`, `Prefs`, `AppView`, `SidebarFilter`, `CartridgeData`
- [x] `src/lib/data.ts` — 16 games, 3 accounts, 6 lists, genres array, statuses array (removed later; the app starts with an empty library)
- [x] Cover art `static/covers/*.png` (12 covers)
- [x] Logo/icon `static/cartridge-icon.png`, `static/cartridge-logo-full.png`

---

## Section 2 — Primitives ✅
- [x] `src/lib/constants.ts` — PLAT, TAGCOLOR, TAGBG, LISTDOT, STATUS_META, SCORE_SOURCES, ALLTAGS
- [x] `src/lib/utils.ts` — `fmtHours()`, `matchFilters()`, `scoreTone()`
- [x] `src/lib/components/Icon.svelte` — dynamic wrapper around `@lucide/svelte`
- [x] `src/lib/components/PlatformDot.svelte` — colored dot with optional ring
- [x] `src/lib/components/Stars.svelte` — 5-star display, optional click-to-set
- [x] `src/lib/components/Tag.svelte` — colored chip with optional remove button
- [x] `src/lib/components/ScoreBadge.svelte` — green/yellow/red numeric badge (sm/md/lg)
- [x] `src/lib/components/ScoreCluster.svelte` — Metacritic + Steam reviews badges side by side (OpenCritic removed, see Section 14)
- [x] `src/lib/components/StatusPill.svelte` — colored status pill (backlog/playing/beaten/dropped/wishlist)
- [x] `src/lib/components/LaunchButton.svelte` — gold "Launch" button or "Not installed" disabled state

---

## Section 3 — Sidebar ✅
- [x] `src/lib/components/Sidebar.svelte`
  - macOS traffic-light dots
  - Cartridge logo + wordmark
  - Library section: All games, Recently played, Favourites, Never played
  - Platforms section: Steam, GOG, Epic (with counts from accounts)
  - Lists section: dynamic list with dot colors, smart/manual badge, counts; "New list…" row
  - Bottom: Settings & sync nav item

---

## Section 4 — Toolbar + FilterPanel ✅
- [x] `src/lib/components/Toolbar.svelte`
  - Title + count (mono)
  - Search field with focus ring
  - Filters button (with active count badge)
  - Sort dropdown menu
  - Grid/List view toggle (segmented)
  - Theme toggle (sun/moon)
  - Sync button (spinner animation when syncing)
- [x] `src/lib/components/FilterPanel.svelte`
  - Category chips (all genres)
  - Platform chips (Steam, GOG, Epic)
  - Status chips (backlog/playing/beaten/dropped/wishlist)
  - Year dual-range slider
  - Metacritic / Steam reviews min-sliders
  - "Clear all" header action
  - "Save as smart list" footer button

---

## Section 5 — Library Views ✅
- [x] `src/lib/components/Cover.svelte` — 3:4 aspect ratio cover art; fallback with gamepad icon + "no cover art"
- [x] `src/lib/components/AggBar.svelte` — game count, total playtime, avg rating, avg Metacritic; list name + type badge when in a list
- [x] `src/lib/components/GridView.svelte` — auto-fill grid of GridCards; hover overlay with LaunchButton
- [x] `src/lib/components/ListView.svelte` — sticky header row; ListRows with cover thumbnail, platform dot, status pill, score badges, playtime, installed/cloud indicator

---

## Section 6 — Game Detail ✅
- [x] `src/lib/components/GameDetail.svelte`
  - Page header: back button, game title breadcrumb, LaunchButton
  - Hero: sticky cover (232px), large LaunchButton, installed status
  - Right column: platform + dev + status pill, h1 title, genre chips
  - Score tiles: Metacritic, Steam reviews (with review count), Hours; Steam match line for GOG/Epic games (Section 14)
  - Store metadata section (OriginLabel "from store · read-only", MetaRow list)
  - Status & rating card (StatusSelect buttons, Stars)
  - Tags card (Tag chips + "+ tag" add button, Notes textarea)
  - Links card (Metacritic, HowLongToBeat, Store — each with open button)
  - Custom fields card (key/value inputs, add/remove)

---

## Section 7 — Settings ✅
- [x] `src/lib/components/Settings.svelte`
  - Connections: AccountCard per account (platform dot, handle, game count, sync status badge, sync button, remove)
  - Steam: API key + Steam ID fields; GOG: OAuth note; Epic: playtime-not-tracked notice
  - "Add an account" dashed button
  - Appearance PrefCard: Theme segmented (Dark/Light), Default view segmented (Grid/List)
  - Library PrefCard: Show critic scores toggle, Confirm before launching toggle, Auto-sync toggle
  - Local data PrefCard: Library file path + "Reveal in Finder", Export JSON

---

## Section 8 — Overlays ✅
- [x] `src/lib/components/CreateListModal.svelte`
  - Name input (autofocus, Enter to submit)
  - Type selector: Manual vs Smart (Smart disabled if no active filters)
  - Color picker (6 dot swatches)
  - Cancel / Create list buttons
- [x] `src/lib/components/LaunchToast.svelte`
  - Fixed bottom-center toast: play icon, game title, "Handing off to {platform}…"
  - Auto-dismisses after 2.6 s

---

## Section 9 — Main App ✅
- [x] Rewrite `src/routes/+page.svelte` — all app-level state and logic:
  - State: `games`, `lists`, `view`, `filter` (sidebar scope), `filters` (deep panel), `filtersOpen`, `theme`, `prefs`, `viewMode`, `sort`, `query`, `selectedId`, `syncing`, `syncingId`, `createOpen`, `toast`
  - Derived: `filtered` (applies sidebar scope + deep filters + search + sort), `activeFilterCount`, `yearBounds`
  - Handlers: `patch()`, `addTag()`, `launch()`, `syncAll()`, `syncOne()`, `createList()`, `clearFilters()`
  - Layout: `<Sidebar>` | `<main>` (Toolbar → FilterPanel + library area | GameDetail | Settings)
  - Theme persistence via `localStorage`

---

## Notes
- All components use Svelte 5 Runes API (`$state`, `$derived`, `$effect`, `$props`)
- Icons via `lucide-svelte` package (to install)
- Cover images served from `static/covers/`; referenced as `/covers/{name}.png`
- No SSR — `prerender = true`, `ssr = false` already set in `+layout.ts`
- Inline styles mirror the prototype exactly; CSS classes used only for design-token utilities

---

## Section 10 — Rust Data Layer

> Establishes the in-memory store, file persistence, and Rust mirrors of the TypeScript types. All subsequent command modules depend on this.

- [x] **Cargo dependencies** — add to `src-tauri/Cargo.toml`:
  - `tokio = { version = "1", features = ["full"] }` — async runtime for HTTP calls
  - `reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }` — HTTP client for platform APIs and cover art downloads
  - `dirs = "5"` — resolves `~/Library/Application Support/…`
  - `anyhow = "1"` — ergonomic error propagation; errors mapped to `String` at the JS boundary
- [x] **`src-tauri/src/models.rs`** — Rust structs mirroring `types.ts`, all `serde::{Serialize, Deserialize}`:
  - `Platform`, `GameStatus`, `ListColor`, `ListKind` enums
  - `Game` struct — all existing fields; add `app_id: Option<String>` (platform-native ID needed for launch deep links)
  - `GamePatch` struct — every `Game` field wrapped in `Option<T>` for partial updates without clobbering user data
  - `Account` struct — add `api_key: Option<String>` (stored in the JSON file, never returned to the frontend)
  - `GameList`, `Filters`, `Prefs`, `CartridgeData` structs
  - `SyncResult` struct: `{ added: u32, updated: u32, errors: Vec<String> }`
- [x] **`src-tauri/src/store.rs`** — `AppStore` managed as `Mutex<AppStore>` via `app.manage()`:
  - `data: CartridgeData` — live in-memory state
  - `path: PathBuf` — `dirs::data_dir()/com.cartridge.app/library.json`
  - `covers_dir: PathBuf` — `dirs::data_dir()/com.cartridge.app/covers/`
  - `AppStore::load(app_handle) -> Self` — reads JSON from disk; on first run, seeds from a compiled-in default (mirroring the current `data.ts` demo games)
  - `AppStore::save(&self) -> anyhow::Result<()>` — atomic write (write to `.tmp` then rename) so a crash never corrupts the library
  - `AppStore::patch_game(&mut self, id: u64, patch: GamePatch) -> Option<Game>` — applies only `Some` fields, auto-saves, returns the updated game
- [x] **Wire into `src-tauri/src/lib.rs`** — call `AppStore::load()` in the `setup` hook, register with `app.manage()`; remove placeholder `greet` command

---

## Section 11 — Tauri Commands

> One file per domain under `src-tauri/src/commands/`. All commands return `Result<T, String>` (anyhow errors stringified) so JS `invoke()` rejects on failure.

- [x] **`src-tauri/src/commands/data.rs`**
  - `load_data(state) -> CartridgeData` — clones in-memory state; called once on app boot
  - `patch_game(state, id: u64, patch: GamePatch) -> Game` — delegates to `AppStore::patch_game`, triggers save
  - `save_prefs(state, prefs: Prefs)` — replaces `data.prefs`, saves
  - `export_json(state) -> String` — pretty-prints `CartridgeData` (strips `api_key` from accounts before returning)

- [x] **`src-tauri/src/commands/lists.rs`**
  - `create_list(state, list: GameList) -> GameList` — assigns a UUID, pushes to `data.lists`, saves
  - `delete_list(state, id: String)` — removes by id, saves
  - `add_to_list(state, list_id: String, game_id: u64)` — appends to `game_ids` of a manual list, saves
  - `remove_from_list(state, list_id: String, game_id: u64)` — removes from `game_ids`, saves

- [x] **`src-tauri/src/commands/launch.rs`**
  - `launch_game(state, app_handle, game_id: u64)` — builds platform deep link from `game.app_id`, opens via `tauri-plugin-opener`:
    - Steam → `steam://rungameid/{app_id}`
    - GOG → `goggalaxy://openGame/{app_id}`
    - Epic → `com.epicgames.launcher://apps/{app_id}?action=launch&silent=true`
    - Fallback (no `app_id`) → opens `game.links.store` in the default browser
  - `open_url(app_handle, url: String)` — generic wrapper used for Metacritic / HLTB / store links

- [x] **`src-tauri/src/commands/sync.rs`**
  - `add_steam_account(state, api_key: String, steam_id: String) -> Account` — validates via `GetPlayerSummaries`; stores `api_key` in the account struct (persisted to disk, never sent to UI); saves
  - `sync_account(state, account_id: String) -> SyncResult`:
    - **Steam**: `GET api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?include_appinfo=1&include_played_free_games=1`
      - Merge strategy: add new games, update `playtime`/`last`/`installed` on existing ones; never overwrite `rating`, `status`, `tags`, `note`, `custom`
      - Cover art: download to `covers_dir/{app_id}.jpg` from `cdn.akamai.steamstatic.com/steam/apps/{id}/library_600x900_2x.jpg` (skip if file already exists)
      - Sets `game.cover` to the absolute path; `coverSrc()` on the frontend will call `convertFileSrc()` for these
    - **GOG / Epic**: implemented later — OAuth login (`start_oauth_flow`), library + catalog import; same-titled games across platforms are merged into one entry with several copies
    - Updates `account.synced` timestamp and `account.status`; saves; returns `SyncResult`
  - `remove_account(state, account_id: String)` — removes account from `data.accounts`, leaves games in library

- [x] **`src-tauri/src/commands/fs.rs`**
  - `get_library_path(state) -> String` — returns absolute path to `library.json`
  - `reveal_in_finder(app_handle, path: String)` — opens the enclosing folder via `tauri-plugin-opener`

- [x] **Register all commands** in `lib.rs` inside `generate_handler![...]`

---

## Section 12 — Frontend API Bridge

> Thin TypeScript wrappers — no business logic, just typed `invoke()` calls. All state logic stays in `+page.svelte`.

- [x] **`src/lib/api.ts`**:
  ```ts
  export const loadData        = ()                     => invoke<CartridgeData>('load_data')
  export const patchGame       = (id, patch)            => invoke<Game>('patch_game', { id, patch })
  export const savePrefs       = (prefs)                => invoke<void>('save_prefs', { prefs })
  export const exportJson      = ()                     => invoke<string>('export_json')

  export const createList      = (list)                 => invoke<GameList>('create_list', { list })
  export const deleteList      = (id)                   => invoke<void>('delete_list', { id })
  export const addToList       = (listId, gameId)       => invoke<void>('add_to_list', { listId, gameId })
  export const removeFromList  = (listId, gameId)       => invoke<void>('remove_from_list', { listId, gameId })

  export const launchGame      = (gameId)               => invoke<void>('launch_game', { gameId })
  export const openUrl         = (url)                  => invoke<void>('open_url', { url })

  export const addSteamAccount = (apiKey, steamId)      => invoke<Account>('add_steam_account', { apiKey, steamId })
  export const syncAccount     = (accountId)            => invoke<SyncResult>('sync_account', { accountId })
  export const removeAccount   = (accountId)            => invoke<void>('remove_account', { accountId })

  export const getLibraryPath  = ()                     => invoke<string>('get_library_path')
  export const revealInFinder  = (path)                 => invoke<void>('reveal_in_finder', { path })
  ```

- [x] **`src/lib/types.ts` additions**:
  - `appId: string | null` on `Game` (required by `launch_game`)
  - `GamePatch` — `Partial<Omit<Game, 'id'>>` (for `patchGame` call sites)
  - `SyncResult` — `{ added: number; updated: number; errors: string[] }`

- [x] **`coverSrc(game: Game): string`** helper in `src/lib/utils.ts`:
  - If `game.cover` starts with `/` or `~` (absolute path from sync) → `convertFileSrc(game.cover)` from `@tauri-apps/api/core`
  - Otherwise → `/covers/${game.cover}.png` (bundled static asset)

---

## Section 13 — Wiring: Replace Mock Data

> Final step: boot the app from real Rust-persisted state instead of `data.ts`.

- [x] **`src/routes/+page.svelte`** — on mount, `await loadData()` to populate `games`, `lists`, `accounts`; replace all mutation handlers with `api.ts` calls; keep derived state and filter logic client-side
- [x] **`src-tauri/tauri.conf.json`** — add `asset` protocol scope for `covers_dir` so `convertFileSrc` URLs resolve correctly in the webview
- [x] **`src/lib/data.ts`** — kept as a seed reference at first; later removed, and `AppStore::load()` now starts with an empty library

---

## Section 14 — Scores ✅

> Per-game scores from Steam, no API keys. Backend in `src-tauri/src/commands/scores.rs` (PRs #2, #3).

- [x] **Steam user reviews** → `userScore` (% positive) + `userScoreReviews` (count), via `IStoreBrowseService/GetItems`, batched 100 apps per request; no % below 10 reviews
- [x] **Metascore** → `metacritic` + `links.metacritic`, via store `appdetails` (1 app per request, paced, rechecked every 30 days)
- [x] **GOG/Epic-only games** matched to a Steam app by title (`steamMatch`): exact → edition labels dropped → Steam subtitle, with series and year checks; `steamMatchRev` re-runs matching for unmatched games when the rules improve
- [x] **Background job** on app start and after every sync; `games-updated` event updates the UI as results arrive
- [x] **Game detail**: shows where Steam scores come from; "Not this game", paste a Steam link / app ID, "Match automatically"
- [x] **UI**: badges with a `%` unit; "User score" renamed "Steam reviews"; OpenCritic removed from all views (fields kept in the model, no source)
- [ ] Progress indicator for the background job (today only `[Scores]` log lines)
- [ ] "Refresh scores" action in Settings that rechecks everything immediately

---

## Section 15 — Visible controls ✅

> Every control the UI already showed now does what it says. Branch `feat/visible-controls`.

- [x] **Unreadable library backup** — `store.rs`: `read_library` / `parse_library`; a `library.json` that can't be read, parsed or migrated is renamed to `library.corrupt-<unix secs>.json` before falling back to empty data, so the next save can't overwrite it (unit tests)
- [x] **Manual lists**
  - `add_to_list` / `remove_from_list` return the updated `GameList` and reject smart lists
  - Game detail: Lists card to add or remove the game from manual lists
  - Context menu on grid cards and list rows: Open, Launch, Add to list (submenu), Remove from the current list
  - Delete list from the list header (`AggBar`) or by right-clicking it in the sidebar; deleting the active list returns to All games
  - Empty manual lists get their own empty state instead of "Nothing matches"
- [x] **`src/lib/components/ConfirmModal.svelte`** — title, body, confirm label, optional danger style; acts once on Enter or click
- [x] **`src/lib/components/ContextMenu.svelte`** — items, separators, submenus, checked and disabled items; keyboard-triggered menus open at the element (`menuPoint`)
- [x] **Free-form tags** — typed in the detail view, normalised by `normTag()` (trimmed, lowercase, single spaces, max 32 chars); suggestions from `allTags()`: library tags plus `ALLTAGS`, most used first
- [x] **Settings → Local data**
  - Real `library.json` path, home shown as `~` (`get_library_path`)
  - Reveal in Finder selects the file (`reveal_library`, takes no path)
  - Export JSON through a save dialog (`tauri-plugin-dialog`, `dialog:allow-save` only) → `export_json_to` writes the library without credentials, via a temp file
- [x] **Last played timestamp** — `GameCopy.lastPlayedAt` (unix secs) from Steam's `rtime_last_played`, kept when duplicates merge; the relative label is computed at render time; the recent sort, the recent filter, `primaryCopy()` and the launch copy picker (`pick_copy`) use it; older libraries fall back to an estimate from the stored `last` label
- [x] **Confirm before launching** — when on, launching asks through `ConfirmModal`
- [x] **Auto-sync on open** — when on and accounts exist, boot runs `syncAll()` (which starts the score job via `sync_account`) instead of `start_score_refresh`; one failing account no longer stops the others

### Later

- [x] Notes are saved on every keystroke; debounce the `patch_game` call (Section 16)
- [x] Tags stored before normalisation keep their casing; normalise them in a migration (Section 16)
- [x] Adding tags in quick succession can race (each patch sends the whole tag array) (Section 16)
- [x] Drop the legacy `last` label in a future migration, once `lastPlayedAt` is filled everywhere (Section 16)

---

## Section 16 — Safe edits and library v2 ✅

> Branch `fix/edits-and-migration`.

- [x] **Library v2 migration** (`store.rs`): `parse_library` runs `migrate_v0_to_v1` → `migrate_v1_to_v2` on the JSON value, then deserialises once. v2 normalises and deduplicates tags, and turns each copy's legacy `last` label into an estimated `lastPlayedAt` (when it has none) before removing it. The original file is kept as `library.v<old>.json`; a library from a newer version is moved aside like an unreadable one (unit tests)
- [x] **`last` removed** from `GameCopy` (Rust and TS), from Steam sync and from `utils.ts`
- [x] **Tags normalised in Rust** too: `norm_tag()` / `clean_tags()` mirror `normTag()`; `patch_game` applies them
- [x] **Optimistic, ordered edits**: `patch()` in `+page.svelte` applies fields at once, queues `patch_game` calls per game, and lays unanswered fields over server copies (`withPending`), so quick successive tag edits no longer drop each other
- [x] **Notes debounced**: `GameDetail` keeps a draft and saves 500 ms after typing stops, on blur, on switching game and on leaving the view

---

## Section 17 — Credentials in the Keychain, CSP ✅

> Branch `feat/keychain-csp`.

- [x] **`src-tauri/src/secrets.rs`** — `Secrets` (the credential fields of `Account`), a `Vault` trait, `Keychain` (macOS Keychain via `keyring` 3, `apple-native`), `FakeVault` for tests; `cargo test -- --ignored live_keychain` round-trips a real item
- [x] **`AppStore`** — release builds: `save` writes changed credentials to the Keychain and strips them from `library.json` (kept in the file for an account whose write failed), deletes removed accounts' items; `load` moves credentials found in the file and reads the rest from the Keychain. A failed read never deletes an item. Debug builds keep credentials in the file and only read the Keychain as a fallback (unit tests)
- [x] **CSP** — `csp` / `devCsp` in `tauri.conf.json`; `'unsafe-inline'` for styles only, with `dangerousDisableAssetCspModification: ["style-src"]`
