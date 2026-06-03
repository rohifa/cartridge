<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import FilterPanel from '$lib/components/FilterPanel.svelte';
  import AggBar from '$lib/components/AggBar.svelte';
  import GridView from '$lib/components/GridView.svelte';
  import ListView from '$lib/components/ListView.svelte';
  import GameDetail from '$lib/components/GameDetail.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import CreateListModal from '$lib/components/CreateListModal.svelte';
  import LaunchToast from '$lib/components/LaunchToast.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import {
    loadData,
    patchGame as apiPatchGame,
    launchGame as apiLaunchGame,
    syncAccount as apiSyncAccount,
    createList as apiCreateList,
    savePrefs as apiSavePrefs,
  } from '$lib/api.js';
  import { matchFilters } from '$lib/utils.js';
  import { PLAT, ALLTAGS } from '$lib/constants.js';
  import type {
    Game, GameList, GamePatch, SidebarFilter, AppView,
    Filters, Prefs, ListColor, ListKind, Account, CartridgeData,
  } from '$lib/types.js';

  type SortKey = 'title' | 'playtime' | 'recent' | 'rating' | 'metacritic' | 'opencritic' | 'userScore' | 'year';

  // ── localStorage — theme only (prefs now persisted in Rust) ──────────────
  function lsGet<T>(key: string, def: T): T {
    try {
      const v = localStorage.getItem('cartridge.' + key);
      return v == null ? def : JSON.parse(v) as T;
    } catch { return def; }
  }
  function lsSave(key: string, val: unknown) {
    try { localStorage.setItem('cartridge.' + key, JSON.stringify(val)); } catch {}
  }

  // ── State — starts empty, filled by onMount ───────────────────────────────
  let games    = $state<Game[]>([]);
  let lists    = $state<GameList[]>([]);
  let accounts = $state<Account[]>([]);
  let genres   = $state<string[]>([]);
  let statuses = $state<CartridgeData['statuses']>([]);
  let prefs    = $state<Prefs>({ defaultView: 'grid', showScores: true, confirmLaunch: false, autoSync: false });

  let view         = $state<AppView>('library');
  let filter       = $state<SidebarFilter>({ type: 'all' });
  let filters      = $state<Filters>({ genres: [], platforms: [], statuses: [], yearLo: 1990, yearHi: new Date().getFullYear(), mcMin: 0, ocMin: 0, usrMin: 0 });
  let filtersOpen  = $state(false);

  let theme        = $state<'dark' | 'light'>(lsGet('theme', 'dark'));
  let viewMode     = $state<'grid' | 'list'>('grid');
  let sort         = $state<SortKey>('title');
  let query        = $state('');

  let selectedId        = $state<number | null>(null);
  let syncing           = $state(false);
  let syncingId         = $state<string | null>(null);
  let createOpen        = $state(false);
  let createFromFilters = $state(false);
  let toast             = $state<{ title: string; platform: string } | null>(null);

  // ── Boot — load persisted state from Rust ─────────────────────────────────
  onMount(async () => {
    try {
      const d = await loadData();
      games    = d.games;
      lists    = d.lists;
      accounts = d.accounts;
      genres   = d.genres;
      statuses = d.statuses;
      prefs    = d.prefs;
      viewMode = d.prefs.defaultView;
      filters  = emptyFilters(); // now yearBounds is correct
    } catch (e) {
      console.error('loadData failed:', e);
    }
  });

  // ── Theme persistence (localStorage) ─────────────────────────────────────
  $effect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    lsSave('theme', theme);
  });

  // ── Derived ───────────────────────────────────────────────────────────────
  const yearBounds = $derived.by<[number, number]>(() => {
    const ys = games.filter(g => g.year > 0).map(g => g.year);
    return ys.length > 0 ? [Math.min(...ys), Math.max(...ys)] : [1990, new Date().getFullYear()];
  });

  function emptyFilters(): Filters {
    return {
      genres: [], platforms: [], statuses: [],
      yearLo: yearBounds[0], yearHi: yearBounds[1],
      mcMin: 0, ocMin: 0, usrMin: 0,
    };
  }

  const data = $derived<CartridgeData>({ accounts, games, lists, genres, statuses, prefs });

  const activeFilterCount = $derived.by(() => {
    let n = filters.genres.length + filters.platforms.length + filters.statuses.length;
    if (filters.yearLo > yearBounds[0] || filters.yearHi < yearBounds[1]) n++;
    if (filters.mcMin)  n++;
    if (filters.ocMin)  n++;
    if (filters.usrMin) n++;
    return n;
  });

  const filtered = $derived.by(() => {
    const f = filter;
    let g = [...games];

    if (f.type === 'platform') {
      g = g.filter(x => x.platform === f.value);
    } else if (f.type === 'fav') {
      g = g.filter(x => x.tags.includes('favourite'));
    } else if (f.type === 'never') {
      g = g.filter(x => x.playtime == null || x.playtime === 0);
    } else if (f.type === 'recent') {
      g = g.filter(x => x.last != null);
    } else if (f.type === 'list') {
      const L = lists.find(l => l.id === f.value);
      if (L) {
        g = L.kind === 'manual'
          ? g.filter(x => (L.gameIds ?? []).includes(x.id))
          : g.filter(x => matchFilters(x, L.rules ?? {}));
      }
    }

    g = g.filter(x => matchFilters(x, filters));

    if (query.trim()) {
      const q = query.toLowerCase();
      g = g.filter(x =>
        x.title.toLowerCase().includes(q) ||
        x.genre.join(' ').toLowerCase().includes(q) ||
        x.dev.toLowerCase().includes(q)
      );
    }

    const cmp: Record<SortKey, (a: Game, b: Game) => number> = {
      title:      (a, b) => a.title.localeCompare(b.title),
      playtime:   (a, b) => (b.playtime ?? 0) - (a.playtime ?? 0),
      rating:     (a, b) => b.rating - a.rating,
      recent:     (a, b) => (a.last ? 0 : 1) - (b.last ? 0 : 1),
      metacritic: (a, b) => (b.metacritic ?? 0) - (a.metacritic ?? 0),
      opencritic: (a, b) => (b.opencritic ?? 0) - (a.opencritic ?? 0),
      userScore:  (a, b) => (b.userScore ?? 0) - (a.userScore ?? 0),
      year:       (a, b) => b.year - a.year,
    };
    return g.sort(cmp[sort]);
  });

  const activeList = $derived.by(() => {
    const f = filter;
    if (f.type !== 'list') return null;
    return lists.find(l => l.id === f.value) ?? null;
  });

  const pageTitle = $derived.by(() => {
    const f = filter;
    if (f.type === 'all')      return 'All games';
    if (f.type === 'fav')      return 'Favourites';
    if (f.type === 'never')    return 'Never played';
    if (f.type === 'recent')   return 'Recently played';
    if (f.type === 'platform') return PLAT[f.value].name;
    if (f.type === 'list')     return activeList?.name ?? 'List';
    return 'Library';
  });

  const selected = $derived(games.find(g => g.id === selectedId) ?? null);

  // ── Handlers — all mutations go through Rust ──────────────────────────────
  async function patch(id: number, fields: GamePatch) {
    const updated = await apiPatchGame(id, fields);
    games = games.map(g => g.id === id ? updated : g);
  }

  async function addTag(g: Game) {
    const next = ALLTAGS.find(t => !g.tags.includes(t));
    if (next) await patch(g.id, { tags: [...g.tags, next] });
  }

  function launch(g: Game) {
    toast = { title: g.title, platform: PLAT[g.platform].name };
    apiLaunchGame(g.id).catch(() => { toast = null; });
  }

  async function syncAll() {
    if (syncing) return;
    syncing = true;
    try {
      for (const acc of accounts) await apiSyncAccount(acc.id);
      const d = await loadData();
      games    = d.games;
      accounts = d.accounts;
    } catch (e) {
      console.error('syncAll failed:', e);
    } finally {
      syncing = false;
    }
  }

  async function syncOne(id: string) {
    if (syncingId) return;
    syncingId = id;
    try {
      await apiSyncAccount(id);
      const d = await loadData();
      games    = d.games;
      accounts = d.accounts;
    } catch (e) {
      console.error('syncOne failed:', e);
    } finally {
      syncingId = null;
    }
  }

  function clearFilters() {
    filters = emptyFilters();
  }

  async function createList(def: { name: string; color: ListColor; kind: ListKind; icon: string }) {
    const list: GameList = def.kind === 'smart'
      ? { id: '', ...def, rules: { ...filters } }
      : { id: '', ...def, gameIds: [] };
    const created = await apiCreateList(list);
    lists = [...lists, created];
    createOpen = false;
    filter = { type: 'list', value: created.id };
    view = 'library';
  }

  function listCount(list: GameList): number {
    if (list.kind === 'manual') return (list.gameIds ?? []).length;
    return games.filter(g => matchFilters(g, list.rules ?? {})).length;
  }

  async function setPref<K extends keyof Prefs>(key: K, val: Prefs[K]) {
    const updated = { ...prefs, [key]: val };
    prefs = updated;
    await apiSavePrefs(updated);
  }
</script>

<div class="app">
  <Sidebar
    {data}
    {view}
    {filter}
    {listCount}
    onNavigate={(v) => { view = v; selectedId = null; }}
    onFilter={(f) => { filter = f; view = 'library'; selectedId = null; }}
    onCreateList={() => { createFromFilters = false; createOpen = true; }}
  />

  <main>
    {#if view === 'settings'}
      <Settings
        {data}
        {syncingId}
        {prefs}
        {theme}
        onSync={syncOne}
        onSetPref={setPref}
        onTheme={(t) => { theme = t; }}
      />

    {:else if selected}
      <GameDetail
        game={selected}
        onBack={() => { selectedId = null; }}
        onPatch={patch}
        onAddTag={addTag}
        onLaunch={launch}
      />

    {:else}
      <Toolbar
        title={pageTitle}
        count={filtered.length}
        {query}
        {sort}
        view={viewMode}
        filterCount={activeFilterCount}
        {filtersOpen}
        {theme}
        {syncing}
        onQuery={(q) => { query = q; }}
        onSort={(s) => { sort = s; }}
        onView={(v) => { viewMode = v; }}
        onToggleFilters={() => { filtersOpen = !filtersOpen; }}
        onToggleTheme={() => { theme = theme === 'dark' ? 'light' : 'dark'; }}
        onSync={syncAll}
      />

      <div class="content-area">
        {#if filtersOpen}
          <FilterPanel
            {data}
            {filters}
            {yearBounds}
            activeCount={activeFilterCount}
            onSetFilters={(updater) => { filters = updater(filters); }}
            onClear={clearFilters}
            onSaveSmart={() => { createFromFilters = true; createOpen = true; }}
          />
        {/if}

        <div class="library-col">
          <AggBar games={filtered} listMeta={activeList} />
          <div class="scroll-area">
            {#if filtered.length === 0}
              <div class="empty">
                <Icon name="search-x" size={34} color="var(--fg-3)" />
                <div class="empty-title">Nothing matches</div>
                <p class="empty-desc">
                  No games fit the current filters. Try loosening a score threshold or clearing a category.
                </p>
              </div>
            {:else if viewMode === 'grid'}
              <GridView
                games={filtered}
                {selectedId}
                showScores={prefs.showScores}
                onSelect={(id) => { selectedId = id; }}
                onLaunch={launch}
              />
            {:else}
              <ListView
                games={filtered}
                {selectedId}
                showScores={prefs.showScores}
                onSelect={(id) => { selectedId = id; }}
                onLaunch={launch}
              />
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>

{#if createOpen}
  <CreateListModal
    fromFilters={createFromFilters}
    {activeFilterCount}
    onClose={() => { createOpen = false; }}
    onCreate={createList}
  />
{/if}

<LaunchToast {toast} onDismiss={() => { toast = null; }} />

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .content-area {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .library-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
    color: var(--fg-3);
    padding: 40px;
    text-align: center;
  }

  .empty :global(svg) {
    opacity: 0.6;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--fg-2);
  }

  .empty-desc {
    font-size: 12.5px;
    max-width: 280px;
    line-height: 1.5;
    margin: 0;
    color: var(--fg-3);
  }
</style>
