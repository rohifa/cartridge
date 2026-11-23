<script lang="ts">
  import { LISTDOT } from '$lib/constants.js';
  import Icon from './Icon.svelte';
  import PlatIcon from './PlatIcon.svelte';
  import type { CartridgeData, SidebarFilter, AppView, GameList, Platform, ScoreProgress } from '$lib/types.js';
  import { totalPlaytime } from '$lib/utils.js';

  let {
    data,
    view,
    filter,
    listCount,
    scoreProgress = null,
    onNavigate,
    onFilter,
    onCreateList,
    onListMenu,
  }: {
    data: CartridgeData;
    view: AppView;
    filter: SidebarFilter;
    listCount: (list: GameList) => number;
    scoreProgress?: ScoreProgress | null;
    onNavigate: (v: AppView) => void;
    onFilter: (f: SidebarFilter) => void;
    onCreateList: () => void;
    onListMenu?: (e: MouseEvent, list: GameList) => void;
  } = $props();

  const total        = $derived(data.games.length);
  const fav          = $derived(data.games.filter(g => g.tags.includes('favourite')).length);
  const never        = $derived(data.games.filter(g => {
    const pt = totalPlaytime(g);
    return pt == null || pt === 0;
  }).length);
  const isLib        = $derived(view === 'library');
  const filterValue  = $derived((filter as any).value as string | undefined);
</script>

{#snippet navItem(opts: {
  icon?: string;
  label: string;
  count?: number | null;
  active: boolean;
  dot?: string;
  platform?: Platform;
  kind?: string;
  onClick: () => void;
  onContextMenu?: (e: MouseEvent) => void;
})}
  <div
    class="nav-item"
    class:active={opts.active}
    onclick={opts.onClick}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && opts.onClick()}
    oncontextmenu={opts.onContextMenu && ((e) => { e.preventDefault(); opts.onContextMenu?.(e); })}
  >
    {#if opts.platform}
      <span class="nav-platform-icon">
        <PlatIcon platform={opts.platform} size={15} />
      </span>
    {:else if opts.dot}
      <span class="nav-dot" style="background:{opts.dot}"></span>
    {:else if opts.icon}
      <Icon name={opts.icon} size={16} color={opts.active ? 'var(--accent)' : 'var(--fg-3)'} />
    {/if}
    <span class="nav-label">{opts.label}</span>
    {#if opts.kind === 'smart'}
      <Icon name="zap" size={11} color={opts.active ? 'var(--accent)' : 'var(--fg-3)'} />
    {/if}
    {#if opts.count != null}
      <span class="nav-count">{opts.count}</span>
    {/if}
  </div>
{/snippet}

<aside>
  <!-- Logo row -->
  <div class="logo-row">
    <img src="/cartridge-icon.png" alt="" width="24" height="24" />
    <span class="logo-name">Cartridge</span>
  </div>

  <!-- Library section -->
  <div class="section-label">Library</div>
  {@render navItem({ icon: 'library',       label: 'All games',       count: total, active: isLib && filter.type === 'all',    onClick: () => onFilter({ type: 'all' }) })}
  {@render navItem({ icon: 'clock',         label: 'Recently played',              active: isLib && filter.type === 'recent',  onClick: () => onFilter({ type: 'recent' }) })}
  {@render navItem({ icon: 'star',          label: 'Favourites',      count: fav,   active: isLib && filter.type === 'fav',    onClick: () => onFilter({ type: 'fav' }) })}
  {@render navItem({ icon: 'circle-dashed', label: 'Never played',    count: never, active: isLib && filter.type === 'never',  onClick: () => onFilter({ type: 'never' }) })}

  <!-- Platforms section -->
  <div class="section-label">Platforms</div>
  {#each data.accounts as a}
    {@render navItem({
      platform: a.platform,
      label:  a.name,
      count:  a.count,
      active: isLib && filter.type === 'platform' && filterValue === a.platform,
      onClick: () => onFilter({ type: 'platform', value: a.platform as Platform }),
    })}
  {/each}

  <!-- Lists section -->
  <div class="section-label">
    <span>Lists</span>
    <button class="section-action" onclick={onCreateList} title="New list">
      <Icon name="plus" size={14} color="var(--fg-3)" />
    </button>
  </div>
  {#each data.lists as list}
    {@render navItem({
      dot:    LISTDOT[list.color],
      label:  list.name,
      kind:   list.kind,
      count:  listCount(list),
      active: isLib && filter.type === 'list' && filterValue === list.id,
      onClick: () => onFilter({ type: 'list', value: list.id }),
      onContextMenu: onListMenu && ((e) => onListMenu?.(e, list)),
    })}
  {/each}
  <div
    class="nav-item nav-new-list"
    onclick={onCreateList}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && onCreateList()}
  >
    <Icon name="plus" size={16} color="var(--fg-3)" />
    <span>New list…</span>
  </div>

  <div style="flex:1"></div>

  <!-- Settings footer -->
  <div class="footer">
    {@render navItem({ icon: 'settings', label: 'Settings & sync', active: view === 'settings', onClick: () => onNavigate('settings') })}
    {#if scoreProgress?.running}
      <div class="scores-status" title="Scores are being updated in the background">
        <span class="spin"><Icon name="loader" size={11} /></span>
        <span>Updating scores</span>
        {#if scoreProgress.total > 1}
          <span class="scores-count">{scoreProgress.done.toLocaleString('en-US')} / {scoreProgress.total.toLocaleString('en-US')}</span>
        {/if}
      </div>
    {/if}
  </div>
</aside>

<style>
  aside {
    width: 236px;
    flex: none;
    background: var(--bg-0);
    border-right: 1px solid var(--border-1);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .logo-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 6px;
  }

  .logo-name {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .section-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 17px 5px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .section-action {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: inline-flex;
    color: var(--fg-3);
    line-height: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 9px;
    margin: 0 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--fg-2);
    font-size: 13px;
    font-weight: 400;
    background: transparent;
    user-select: none;
  }

  .nav-item:hover {
    background: var(--bg-3);
  }

  .nav-item.active {
    background: var(--bg-sel);
    color: var(--fg-1);
    font-weight: 600;
  }

  .nav-item.active:hover {
    background: var(--bg-sel);
  }

  .nav-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
    margin-left: 1px;
  }

  .nav-platform-icon {
    width: 16px;
    height: 16px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: currentColor;
  }

  .nav-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nav-count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-3);
  }

  .nav-new-list {
    color: var(--fg-3);
    margin-top: 2px;
  }

  .footer {
    border-top: 1px solid var(--border-1);
    padding: 7px 0 9px;
  }

  .scores-status {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 2px 17px 0;
    font-size: 11px;
    color: var(--fg-3);
  }

  .scores-count {
    margin-left: auto;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .spin {
    display: inline-flex;
    animation: crt-spin 1s linear infinite;
  }
</style>
