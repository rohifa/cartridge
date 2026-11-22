<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { save } from '@tauri-apps/plugin-dialog';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import FilterPanel from '$lib/components/FilterPanel.svelte';
  import AggBar from '$lib/components/AggBar.svelte';
  import GridView from '$lib/components/GridView.svelte';
  import ListView from '$lib/components/ListView.svelte';
  import GameDetail from '$lib/components/GameDetail.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import AddAccountModal from '$lib/components/AddAccountModal.svelte';
  import CreateListModal from '$lib/components/CreateListModal.svelte';
  import LaunchToast from '$lib/components/LaunchToast.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import ContextMenu, { menuPoint, type MenuItem } from '$lib/components/ContextMenu.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import {
    loadData,
    patchGame as apiPatchGame,
    launchGame as apiLaunchGame,
    syncAccount as apiSyncAccount,
    removeAccount as apiRemoveAccount,
    createList as apiCreateList,
    deleteList as apiDeleteList,
    addToList as apiAddToList,
    removeFromList as apiRemoveFromList,
    savePrefs as apiSavePrefs,
    startOAuthFlow,
    startScoreRefresh,
    setSteamMatch,
    rejectSteamMatch,
    autoSteamMatch,
    exportJsonTo,
    getLibraryPath,
    revealLibrary as apiRevealLibrary,
  } from '$lib/api.js';
  import { allTags, matchFilters, primaryCopy, totalPlaytime } from '$lib/utils.js';
  import { PLAT, LISTDOT } from '$lib/constants.js';
  import type {
    Game, GameList, GamePatch, SidebarFilter, AppView,
    Filters, Prefs, ListColor, ListKind, Account, CartridgeData, Platform, SteamMatchAction,
  } from '$lib/types.js';

  type SortKey = 'title' | 'playtime' | 'recent' | 'rating' | 'metacritic' | 'userScore' | 'year';

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
  let syncingIds        = $state(new Set<string>());
  let createOpen        = $state(false);
  let createFromFilters = $state(false);
  let addAccountOpen    = $state(false);
  let editingAccount    = $state<{ platform: Platform; steamId: string } | null>(null);
  let toast             = $state<{ title: string; platform: string } | null>(null);
  let syncNotif         = $state<{ ok: boolean; message: string } | null>(null);
  let syncNotifTimer    = 0;
  let menu              = $state<{ x: number; y: number; items: MenuItem[]; label: string } | null>(null);
  let confirmReq        = $state<ConfirmRequest | null>(null);
  let libraryPath       = $state('');
  let exporting         = $state(false);

  type ConfirmRequest = {
    title: string;
    body: string;
    confirmLabel: string;
    danger?: boolean;
    onConfirm: () => void;
  };

  // Background score refreshes in Rust push single-game updates.
  onMount(() => {
    const unlisten = listen<Game[]>('games-updated', (e) => {
      const byId = new Map(e.payload.map(g => [g.id, g]));
      games = games.map(g => byId.get(g.id) ?? g);
    });
    return () => { unlisten.then(f => f()); };
  });

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
      // Only does work for games never checked or due a recheck.
      startScoreRefresh().catch(e => console.error('score refresh failed:', e));
    } catch (e) {
      console.error('loadData failed:', e);
    }
  });

  onMount(() => {
    getLibraryPath()
      .then(p => { libraryPath = p; })
      .catch(e => console.error('getLibraryPath failed:', e));
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
    if (filters.usrMin) n++;
    return n;
  });

  const filtered = $derived.by(() => {
    const f = filter;
    let g = [...games];

    if (f.type === 'platform') {
      g = g.filter(x => x.copies.some(c => c.platform === f.value));
    } else if (f.type === 'fav') {
      g = g.filter(x => x.tags.includes('favourite'));
    } else if (f.type === 'never') {
      const playtime = (x: Game) => totalPlaytime(x);
      g = g.filter(x => playtime(x) == null || playtime(x) === 0);
    } else if (f.type === 'recent') {
      g = g.filter(x => x.copies.some(c => c.last != null));
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
      playtime:   (a, b) => (totalPlaytime(b) ?? 0) - (totalPlaytime(a) ?? 0),
      rating:     (a, b) => b.rating - a.rating,
      recent:     (a, b) => {
        const aLast = primaryCopy(a)?.last;
        const bLast = primaryCopy(b)?.last;
        return (aLast ? 0 : 1) - (bLast ? 0 : 1);
      },
      metacritic: (a, b) => (b.metacritic ?? 0) - (a.metacritic ?? 0),
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
  const knownTags = $derived(allTags(games));

  // An empty manual list gets its own empty state instead of "Nothing matches":
  // no filter or query can be the reason nothing shows.
  const emptyManualList = $derived.by(() => {
    if (activeList?.kind !== 'manual') return false;
    const ids = activeList.gameIds ?? [];
    return !games.some(g => ids.includes(g.id));
  });

  // ── Handlers — all mutations go through Rust ──────────────────────────────
  async function patch(id: number, fields: GamePatch) {
    const updated = await apiPatchGame(id, fields);
    games = games.map(g => g.id === id ? updated : g);
  }

  /// Throws on failure (e.g. a bad link) so GameDetail can show the message.
  async function steamMatch(id: number, action: SteamMatchAction) {
    const updated =
      action.kind === 'set'    ? await setSteamMatch(id, action.input) :
      action.kind === 'reject' ? await rejectSteamMatch(id) :
                                 await autoSteamMatch(id);
    games = games.map(g => g.id === id ? updated : g);
  }

  function launch(g: Game) {
    const copy = primaryCopy(g);
    toast = { title: g.title, platform: copy ? PLAT[copy.platform].name : 'Unknown' };
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
      lists    = d.lists; // sync may merge duplicates and remap list memberships
    } catch (e) {
      console.error('syncAll failed:', e);
    } finally {
      syncing = false;
    }
  }

  function showSyncNotif(ok: boolean, message: string) {
    clearTimeout(syncNotifTimer);
    syncNotif = { ok, message };
    syncNotifTimer = window.setTimeout(() => { syncNotif = null; }, 5000);
  }

  async function reauth(platform: string) {
    try {
      const account = await startOAuthFlow(platform);
      const d = await loadData();
      accounts = d.accounts;
      showSyncNotif(true, `Re-authenticated ${account.name} as ${account.handle}.`);
    } catch (e) {
      showSyncNotif(false, String(e));
    }
  }

  async function revealLibrary() {
    try {
      await apiRevealLibrary();
    } catch (e) {
      showSyncNotif(false, String(e));
    }
  }

  async function exportLibrary() {
    if (exporting) return;
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    const defaultPath = `cartridge-export-${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}.json`;
    exporting = true;
    try {
      const path = await save({ defaultPath, filters: [{ name: 'JSON', extensions: ['json'] }] });
      if (!path) return; // cancelled
      const count = await exportJsonTo(path);
      const file = path.split(/[\\/]/).pop();
      showSyncNotif(true, `Exported ${count.toLocaleString('en-US')} game${count === 1 ? '' : 's'} to ${file}.`);
    } catch (e) {
      showSyncNotif(false, String(e));
    } finally {
      exporting = false;
    }
  }

  async function syncOne(id: string) {
    if (syncingIds.has(id)) return;
    syncingIds = new Set([...syncingIds, id]);
    try {
      const result = await apiSyncAccount(id);
      const d = await loadData();
      games    = d.games;
      accounts = d.accounts;
      lists    = d.lists; // sync may merge duplicates and remap list memberships
      if (result.errors.length > 0) {
        showSyncNotif(false, result.errors[0]);
      } else if (result.added === 0 && result.updated === 0) {
        showSyncNotif(false, 'No games found. In Steam, go to Edit Profile → Privacy Settings and set "Game details" to Public.');
      } else {
        const parts = [];
        if (result.added > 0)   parts.push(`${result.added} new`);
        if (result.updated > 0) parts.push(`${result.updated} updated`);
        showSyncNotif(true, `Synced ${parts.join(', ')} game${result.added + result.updated === 1 ? '' : 's'}.`);
      }
    } catch (e) {
      // Reload accounts so any status change written by Rust (e.g. Error after
      // a token-refresh failure) is immediately reflected — the Re-authenticate
      // button in Settings must appear without requiring an app restart.
      try { const d = await loadData(); accounts = d.accounts; } catch {}
      showSyncNotif(false, String(e));
    } finally {
      const next = new Set(syncingIds);
      next.delete(id);
      syncingIds = next;
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

  function inList(list: GameList, gameId: number): boolean {
    return list.kind === 'manual' && (list.gameIds ?? []).includes(gameId);
  }

  /// Adds the game to a manual list, or removes it if it's already there.
  async function toggleInList(listId: string, gameId: number) {
    const list = lists.find(l => l.id === listId);
    if (!list || list.kind !== 'manual') return;
    try {
      const updated = inList(list, gameId)
        ? await apiRemoveFromList(listId, gameId)
        : await apiAddToList(listId, gameId);
      lists = lists.map(l => l.id === listId ? updated : l);
    } catch (e) {
      showSyncNotif(false, String(e));
    }
  }

  async function deleteList(id: string) {
    try {
      await apiDeleteList(id);
      lists = lists.filter(l => l.id !== id);
      if (filter.type === 'list' && filter.value === id) filter = { type: 'all' };
    } catch (e) {
      showSyncNotif(false, String(e));
    }
  }

  function askDeleteList(list: GameList) {
    confirmReq = {
      title: `Delete "${list.name}"?`,
      body: 'Games stay in your library.',
      confirmLabel: 'Delete list',
      danger: true,
      onConfirm: () => deleteList(list.id),
    };
  }

  function openCreateList() {
    createFromFilters = false;
    createOpen = true;
  }

  // ── Context menus ─────────────────────────────────────────────────────────
  function openMenu(e: MouseEvent, label: string, items: MenuItem[]) {
    menu = { ...menuPoint(e), label, items };
  }

  function gameMenu(e: MouseEvent, g: Game) {
    const manual = lists.filter(l => l.kind === 'manual');
    const listItems: MenuItem[] = manual.length > 0
      ? manual.map(l => ({
          label: l.name,
          dot: LISTDOT[l.color],
          checked: inList(l, g.id),
          onSelect: () => toggleInList(l.id, g.id),
        }))
      : [
          { label: 'No manual lists yet', disabled: true, onSelect: () => {} },
          { kind: 'separator' },
          { label: 'New list…', onSelect: openCreateList },
        ];
    const items: MenuItem[] = [
      { label: 'Open', onSelect: () => { selectedId = g.id; } },
      { label: 'Launch', disabled: !primaryCopy(g)?.installed, onSelect: () => launch(g) },
      { kind: 'separator' },
      { kind: 'submenu', label: 'Add to list', items: listItems },
    ];
    const current = activeList;
    if (current?.kind === 'manual' && inList(current, g.id)) {
      items.push({ label: `Remove from ${current.name}`, onSelect: () => toggleInList(current.id, g.id) });
    }
    openMenu(e, g.title, items);
  }

  function listMenu(e: MouseEvent, list: GameList) {
    openMenu(e, list.name, [
      { label: 'Delete list', danger: true, onSelect: () => askDeleteList(list) },
    ]);
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
    onCreateList={openCreateList}
    onListMenu={listMenu}
  />

  <main>
    {#if view === 'settings'}
      <Settings
        {data}
        {syncingIds}
        {prefs}
        {theme}
        {libraryPath}
        {exporting}
        onSync={syncOne}
        onSetPref={setPref}
        onTheme={(t) => { theme = t; }}
        onAddAccount={() => { addAccountOpen = true; }}
        onEditAccount={(acc) => { editingAccount = { platform: acc.platform, steamId: acc.id }; }}
        onRemoveAccount={async (id) => {
          await apiRemoveAccount(id);
          accounts = accounts.filter(a => a.id !== id);
        }}
        onReauth={reauth}
        onReveal={revealLibrary}
        onExport={exportLibrary}
      />

    {:else if selected}
      <GameDetail
        game={selected}
        {lists}
        onBack={() => { selectedId = null; }}
        onPatch={patch}
        onSteamMatch={steamMatch}
        {knownTags}
        onLaunch={launch}
        onToggleList={toggleInList}
        onCreateList={openCreateList}
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
        <div class="library-col">
          <AggBar games={filtered} listMeta={activeList} onDeleteList={askDeleteList} />
          <div class="scroll-area">
            {#if emptyManualList}
              <div class="empty">
                <Icon name="list" size={34} color="var(--fg-3)" />
                <div class="empty-title">This list is empty</div>
                <p class="empty-desc">Right-click a game, or use Lists on its page, to add it.</p>
              </div>
            {:else if filtered.length === 0}
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
                onContextMenu={gameMenu}
              />
            {:else}
              <ListView
                games={filtered}
                {selectedId}
                showScores={prefs.showScores}
                onSelect={(id) => { selectedId = id; }}
                onLaunch={launch}
                onContextMenu={gameMenu}
              />
            {/if}
          </div>
        </div>

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
      </div>
    {/if}
  </main>
</div>

{#if addAccountOpen || editingAccount}
  <AddAccountModal
    editAccount={editingAccount}
    onClose={() => { addAccountOpen = false; editingAccount = null; }}
    onAdd={(account) => {
      accounts = [...accounts.filter(a => a.id !== account.id), account];
      addAccountOpen = false;
      editingAccount = null;
      syncOne(account.id);
    }}
  />
{/if}

{#if createOpen}
  <CreateListModal
    fromFilters={createFromFilters}
    {activeFilterCount}
    onClose={() => { createOpen = false; }}
    onCreate={createList}
  />
{/if}

{#if confirmReq}
  {@const req = confirmReq}
  <ConfirmModal
    title={req.title}
    body={req.body}
    confirmLabel={req.confirmLabel}
    danger={req.danger}
    onConfirm={() => { confirmReq = null; req.onConfirm(); }}
    onCancel={() => { confirmReq = null; }}
  />
{/if}

{#if menu}
  {#key menu}
    <ContextMenu
      x={menu.x}
      y={menu.y}
      items={menu.items}
      label={menu.label}
      onClose={() => { menu = null; }}
    />
  {/key}
{/if}

<LaunchToast {toast} onDismiss={() => { toast = null; }} />

{#if syncNotif}
  <div class="sync-notif" class:notif-ok={syncNotif.ok} class:notif-err={!syncNotif.ok}>
    <Icon name={syncNotif.ok ? 'check-circle' : 'alert-circle'} size={14} color={syncNotif.ok ? 'var(--ok)' : 'var(--danger)'} />
    {syncNotif.message}
  </div>
{/if}

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
    min-height: 0;
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

  .sync-notif {
    position: fixed;
    bottom: 22px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 80;
    display: flex;
    align-items: flex-start;
    gap: 9px;
    max-width: 420px;
    padding: 11px 16px;
    border-radius: var(--radius-md);
    background: var(--bg-2);
    border: 1px solid var(--border-2);
    box-shadow: var(--shadow-lg);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .notif-ok  { color: var(--fg-1); }
  .notif-err { color: var(--danger); }
</style>
