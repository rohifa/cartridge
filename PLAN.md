# Cartridge — Implementation Plan

Translating the React prototype in `project/` into Svelte 5 (Runes API) within the Tauri + SvelteKit structure.

---

## Section 1 — Foundation ✅
- [x] Update `src/app.html` — fonts (Geist + Geist Mono via Google Fonts), `data-theme="dark"` on `<html>`, links to `design-tokens.css` + `global.css`
- [x] `static/design-tokens.css` — all CSS custom properties, dark + light themes
- [x] `static/global.css` — body reset, scrollbars, range slider, spin animation
- [x] `src/lib/types.ts` — TypeScript interfaces: `Game`, `Account`, `GameList`, `Filters`, `Prefs`, `AppView`, `SidebarFilter`, `CartridgeData`
- [x] `src/lib/data.ts` — 16 games, 3 accounts, 6 lists, genres array, statuses array
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
- [x] `src/lib/components/ScoreCluster.svelte` — MC + OC + USR badges side by side
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

## Section 4 — Toolbar + FilterPanel
- [ ] `src/lib/components/Toolbar.svelte`
  - Title + count (mono)
  - Search field with focus ring
  - Filters button (with active count badge)
  - Sort dropdown menu
  - Grid/List view toggle (segmented)
  - Theme toggle (sun/moon)
  - Sync button (spinner animation when syncing)
- [ ] `src/lib/components/FilterPanel.svelte`
  - Category chips (all genres)
  - Platform chips (Steam, GOG, Epic)
  - Status chips (backlog/playing/beaten/dropped/wishlist)
  - Year dual-range slider
  - Metacritic / OpenCritic / User score min-sliders
  - "Clear all" header action
  - "Save as smart list" footer button

---

## Section 5 — Library Views
- [ ] `src/lib/components/Cover.svelte` — 3:4 aspect ratio cover art; fallback with gamepad icon + "no cover art"
- [ ] `src/lib/components/AggBar.svelte` — game count, total playtime, avg rating, avg Metacritic; list name + type badge when in a list
- [ ] `src/lib/components/GridView.svelte` — auto-fill grid of GridCards; hover overlay with LaunchButton
- [ ] `src/lib/components/ListView.svelte` — sticky header row; ListRows with cover thumbnail, platform dot, status pill, score badges, playtime, installed/cloud indicator

---

## Section 6 — Game Detail
- [ ] `src/lib/components/GameDetail.svelte`
  - Page header: back button, game title breadcrumb, LaunchButton
  - Hero: sticky cover (232px), large LaunchButton, installed status
  - Right column: platform + dev + status pill, h1 title, genre chips
  - Score tiles: MC, OC, USR, Hours
  - Store metadata section (OriginLabel "from store · read-only", MetaRow list)
  - Status & rating card (StatusSelect buttons, Stars)
  - Tags card (Tag chips + "+ tag" add button, Notes textarea)
  - Links card (Metacritic, OpenCritic, HowLongToBeat, Store — each with open button)
  - Custom fields card (key/value inputs, add/remove)

---

## Section 7 — Settings
- [ ] `src/lib/components/Settings.svelte`
  - Connections: AccountCard per account (platform dot, handle, game count, sync status badge, sync button, remove)
  - Steam: API key + Steam ID fields; GOG: OAuth note; Epic: playtime-not-tracked notice
  - "Add an account" dashed button
  - Appearance PrefCard: Theme segmented (Dark/Light), Default view segmented (Grid/List)
  - Library PrefCard: Show critic scores toggle, Confirm before launching toggle, Auto-sync toggle
  - Local data PrefCard: Library file path + "Reveal in Finder", Export JSON

---

## Section 8 — Overlays
- [ ] `src/lib/components/CreateListModal.svelte`
  - Name input (autofocus, Enter to submit)
  - Type selector: Manual vs Smart (Smart disabled if no active filters)
  - Color picker (6 dot swatches)
  - Cancel / Create list buttons
- [ ] `src/lib/components/LaunchToast.svelte`
  - Fixed bottom-center toast: play icon, game title, "Handing off to {platform}…"
  - Auto-dismisses after 2.6 s

---

## Section 9 — Main App
- [ ] Rewrite `src/routes/+page.svelte` — all app-level state and logic:
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
