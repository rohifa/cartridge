<script lang="ts">
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { cartridgeData } from '$lib/data.js';
  import { matchFilters } from '$lib/utils.js';
  import type { SidebarFilter, AppView, GameList } from '$lib/types.js';

  let view   = $state<AppView>('library');
  let filter = $state<SidebarFilter>({ type: 'all' });

  function listCount(list: GameList): number {
    if (list.kind === 'manual') return (list.gameIds ?? []).length;
    return cartridgeData.games.filter(g => matchFilters(g, list.rules ?? null)).length;
  }
</script>

<div style="display:flex; height:100vh; overflow:hidden">
  <Sidebar
    data={cartridgeData}
    {view}
    {filter}
    {listCount}
    onNavigate={(v) => { view = v; }}
    onFilter={(f) => { filter = f; view = 'library'; }}
    onCreateList={() => {}}
  />

  <main style="flex:1; display:flex; align-items:center; justify-content:center; color:var(--fg-3); font-size:13px">
    {#if view === 'settings'}
      Settings — coming in Section 7
    {:else}
      Library view — coming in Section 5
    {/if}
  </main>
</div>
