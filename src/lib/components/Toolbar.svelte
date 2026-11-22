<script lang="ts">
  import Icon from './Icon.svelte';

  type SortKey = 'title' | 'playtime' | 'recent' | 'rating' | 'metacritic' | 'userScore' | 'year';
  type ViewMode = 'grid' | 'list';
  type Theme = 'dark' | 'light';

  let {
    title,
    count,
    query,
    sort,
    view,
    filterCount,
    filtersOpen,
    theme,
    syncing,
    onQuery,
    onSort,
    onView,
    onToggleFilters,
    onToggleTheme,
    onSync,
  }: {
    title: string;
    count: number;
    query: string;
    sort: SortKey;
    view: ViewMode;
    filterCount: number;
    filtersOpen: boolean;
    theme: Theme;
    syncing: boolean;
    onQuery: (q: string) => void;
    onSort: (s: SortKey) => void;
    onView: (v: ViewMode) => void;
    onToggleFilters: () => void;
    onToggleTheme: () => void;
    onSync: () => void;
  } = $props();

  const SORT_OPTS: [SortKey, string][] = [
    ['title',       'Title'],
    ['playtime',    'Playtime'],
    ['recent',      'Recently played'],
    ['rating',      'Your rating'],
    ['metacritic',  'Metacritic'],
    ['userScore',   'Steam reviews'],
    ['year',        'Release date'],
  ];

  let sortOpen = $state(false);
  let searchFocused = $state(false);

  const sortLabel = $derived(SORT_OPTS.find(([k]) => k === sort)?.[1] ?? sort);
  const filtersActive = $derived(filterCount > 0);
</script>

<div class="toolbar">
  <!-- Title + count -->
  <div class="title-group">
    <span class="title">{title}</span>
    <span class="count">{count}</span>
  </div>

  <div class="spacer"></div>

  <!-- Search -->
  <div class="search" class:focused={searchFocused}>
    <Icon name="search" size={15} color="var(--fg-3)" />
    <input
      type="text"
      placeholder="Search your library…"
      value={query}
      oninput={(e) => onQuery((e.target as HTMLInputElement).value)}
      onfocus={() => searchFocused = true}
      onblur={() => searchFocused = false}
    />
  </div>

  <!-- Filters button -->
  <button
    class="filters-btn"
    class:active={filtersOpen}
    class:on={filtersActive}
    onclick={onToggleFilters}
  >
    <Icon name="sliders-horizontal" size={14} color={filtersActive ? 'var(--accent)' : 'var(--fg-2)'} />
    Filters
    {#if filtersActive}
      <span class="filter-badge">{filterCount}</span>
    {/if}
  </button>

  <!-- Sort dropdown -->
  <div class="sort-wrap">
    <button class="sort-btn" onclick={() => sortOpen = !sortOpen}>
      <Icon name="arrow-up-down" size={14} color="var(--fg-2)" />
      {sortLabel}
    </button>
    {#if sortOpen}
      <div class="backdrop" onclick={() => sortOpen = false} role="presentation"></div>
      <div class="dropdown">
        {#each SORT_OPTS as [key, label]}
          <div
            class="dropdown-item"
            class:selected={key === sort}
            role="menuitem"
            tabindex="0"
            onclick={() => { onSort(key); sortOpen = false; }}
            onkeydown={(e) => e.key === 'Enter' && (onSort(key), sortOpen = false)}
          >
            {label}
            {#if key === sort}
              <Icon name="check" size={14} color="var(--accent)" />
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- View toggle -->
  <div class="seg">
    <button
      class="tool-btn"
      class:active={view === 'grid'}
      title="Grid"
      onclick={() => onView('grid')}
    >
      <Icon name="layout-grid" size={16} color={view === 'grid' ? 'var(--fg-1)' : 'var(--fg-2)'} />
    </button>
    <button
      class="tool-btn"
      class:active={view === 'list'}
      title="List"
      onclick={() => onView('list')}
    >
      <Icon name="list" size={16} color={view === 'list' ? 'var(--fg-1)' : 'var(--fg-2)'} />
    </button>
  </div>

  <!-- Theme toggle -->
  <button class="tool-btn" title={theme === 'dark' ? 'Switch to light' : 'Switch to dark'} onclick={onToggleTheme}>
    <Icon name={theme === 'dark' ? 'sun' : 'moon'} size={16} color="var(--fg-2)" />
  </button>

  <!-- Sync button -->
  <button class="sync-btn" onclick={onSync} disabled={syncing}>
    <span class:spin={syncing}>
      <Icon name="refresh-cw" size={14} color="var(--fg-on-accent)" />
    </span>
    {syncing ? 'Syncing…' : 'Sync'}
  </button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    height: 52px;
    border-bottom: 1px solid var(--border-1);
    flex: none;
    background: var(--bg-1);
  }

  .title-group {
    display: flex;
    align-items: baseline;
    gap: 9px;
    min-width: 0;
    flex: none;
  }

  .title {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }

  .count {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-3);
  }

  .spacer {
    flex: 1;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 1 240px;
    min-width: 140px;
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    transition: border-color 0.1s, box-shadow 0.1s;
  }

  .search.focused {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--fg-1);
    font-family: var(--font-ui);
    font-size: 13px;
  }

  .search input::placeholder {
    color: var(--fg-3);
  }

  .filters-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 11px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border-1);
    background: var(--bg-3);
    color: var(--fg-2);
    transition: border-color 0.1s, color 0.1s;
  }

  .filters-btn.on,
  .filters-btn.active {
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }

  .filters-btn.active {
    background: var(--accent-soft);
  }

  .filter-badge {
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--fg-on-accent);
    font-family: var(--font-mono);
    font-size: 9.5px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .sort-wrap {
    position: relative;
  }

  .sort-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    border: 1px solid var(--border-1);
    background: var(--bg-3);
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
  }

  .dropdown {
    position: absolute;
    top: 34px;
    right: 0;
    z-index: 21;
    min-width: 180px;
    background: var(--bg-2);
    border: 1px solid var(--border-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    padding: 5px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 13px;
    color: var(--fg-2);
    background: transparent;
    user-select: none;
  }

  .dropdown-item:hover {
    background: var(--bg-3);
  }

  .dropdown-item.selected {
    color: var(--fg-1);
    background: var(--bg-3);
  }

  .seg {
    display: flex;
    gap: 2px;
    background: var(--bg-3);
    padding: 2px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-1);
  }

  .tool-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 28px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg-2);
  }

  .tool-btn:hover {
    background: var(--bg-3);
  }

  .tool-btn.active {
    border-color: var(--border-2);
    background: var(--bg-3);
    color: var(--fg-1);
  }

  .sync-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 13px;
    border-radius: var(--radius-sm);
    border: none;
    cursor: pointer;
    background: var(--accent);
    color: var(--fg-on-accent);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
  }

  .sync-btn:disabled {
    opacity: 0.85;
    cursor: default;
  }

  .spin {
    display: inline-flex;
    animation: crt-spin 1s linear infinite;
  }
</style>
