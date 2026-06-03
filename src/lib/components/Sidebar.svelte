<script lang="ts">
  import { PLAT, LISTDOT } from '$lib/constants.js';
  import Icon from './Icon.svelte';
  import type { CartridgeData, SidebarFilter, AppView, GameList, Platform } from '$lib/types.js';

  let {
    data,
    view,
    filter,
    listCount,
    onNavigate,
    onFilter,
    onCreateList,
  }: {
    data: CartridgeData;
    view: AppView;
    filter: SidebarFilter;
    listCount: (list: GameList) => number;
    onNavigate: (v: AppView) => void;
    onFilter: (f: SidebarFilter) => void;
    onCreateList: () => void;
  } = $props();

  const total        = $derived(data.games.length);
  const fav          = $derived(data.games.filter(g => g.tags.includes('favourite')).length);
  const never        = $derived(data.games.filter(g => g.playtime == null || g.playtime === 0).length);
  const isLib        = $derived(view === 'library');
  const filterValue  = $derived((filter as any).value as string | undefined);
</script>

{#snippet navItem(opts: {
  icon?: string;
  label: string;
  count?: number | null;
  active: boolean;
  dot?: string;
  kind?: string;
  onClick: () => void;
})}
  <div
    class="nav-item"
    class:active={opts.active}
    onclick={opts.onClick}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && opts.onClick()}
  >
    {#if opts.dot}
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
  <!-- macOS traffic lights -->
  <div class="traffic-lights">
    {#each ['#ff5f57', '#febc2e', '#28c840'] as c}
      <span class="traffic-dot" style="background:{c}"></span>
    {/each}
  </div>

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
      dot:    PLAT[a.platform].color,
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

  .traffic-lights {
    display: flex;
    gap: 8px;
    padding: 14px 16px 0;
  }

  .traffic-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex: none;
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
</style>
