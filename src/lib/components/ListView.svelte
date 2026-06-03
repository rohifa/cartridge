<script lang="ts">
  import PlatformDot from './PlatformDot.svelte';
  import StatusPill from './StatusPill.svelte';
  import ScoreBadge from './ScoreBadge.svelte';
  import LaunchButton from './LaunchButton.svelte';
  import Icon from './Icon.svelte';
  import { fmtHours } from '$lib/utils.js';
  import type { Game } from '$lib/types.js';

  let {
    games,
    selectedId,
    showScores,
    onSelect,
    onLaunch,
  }: {
    games: Game[];
    selectedId: number | null;
    showScores: boolean;
    onSelect: (id: number) => void;
    onLaunch: (g: Game) => void;
  } = $props();

  const cols = $derived(showScores
    ? '30px 1fr 116px 92px 40px 40px 40px 56px 112px'
    : '30px 1fr 1fr 110px 56px 112px');
</script>

<div class="list">
  <!-- Sticky header -->
  <div class="head" style="grid-template-columns:{cols}">
    <span></span>
    <span>Title</span>
    <span>Genre</span>
    <span>Status</span>
    {#if showScores}
      <span class="center">MC</span>
      <span class="center">OC</span>
      <span class="center">USR</span>
    {/if}
    <span class="right">Hours</span>
    <span></span>
  </div>

  {#each games as g (g.id)}
    <div
      class="row"
      class:selected={g.id === selectedId}
      style="grid-template-columns:{cols}"
      onclick={() => onSelect(g.id)}
      role="button"
      tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onSelect(g.id)}
    >
      <!-- Thumbnail -->
      {#if g.cover}
        <img
          src="/covers/{g.cover}.png"
          alt=""
          class="thumb"
        />
      {:else}
        <div class="thumb thumb-empty">
          <PlatformDot platform={g.platform} size={7} ring={false} />
        </div>
      {/if}

      <!-- Title + platform dot -->
      <span class="title-cell">
        <PlatformDot platform={g.platform} size={7} ring={false} />
        <span class="title-text">{g.title}</span>
      </span>

      <!-- Genre -->
      <span class="genre">{g.genre.join(', ')}</span>

      <!-- Status -->
      <span>
        {#if g.status}
          <StatusPill status={g.status} size="sm" />
        {/if}
      </span>

      <!-- Score badges (conditional) -->
      {#if showScores}
        <span class="center"><ScoreBadge value={g.metacritic} size="sm" /></span>
        <span class="center"><ScoreBadge value={g.opencritic} size="sm" /></span>
        <span class="center"><ScoreBadge value={g.userScore}  size="sm" /></span>
      {/if}

      <!-- Playtime -->
      <span class="hours" class:dim={g.playtime == null}>{fmtHours(g.playtime)}</span>

      <!-- Launch on hover / cloud indicator at rest -->
      <span class="actions">
        <span class="action-launch">
          <LaunchButton game={g} {onLaunch} />
        </span>
        <span class="action-status">
          <Icon name={g.installed ? 'hard-drive' : 'cloud'} size={12} color="var(--fg-3)" />
          {g.installed ? 'installed' : 'cloud'}
        </span>
      </span>
    </div>
  {/each}
</div>

<style>
  .list {
    min-width: 0;
  }

  .head {
    display: grid;
    align-items: center;
    gap: 13px;
    padding: 7px 22px;
    position: sticky;
    top: 0;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border-1);
    z-index: 1;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .row {
    display: grid;
    align-items: center;
    gap: 13px;
    padding: 7px 22px;
    cursor: pointer;
    font-size: 13px;
    border-bottom: 1px solid var(--border-1);
    background: transparent;
  }

  .row:hover {
    background: var(--bg-3);
  }

  .row.selected {
    background: var(--bg-sel);
  }

  .row.selected:hover {
    background: var(--bg-sel);
  }

  .thumb {
    width: 30px;
    height: 40px;
    object-fit: cover;
    border-radius: 3px;
    box-shadow: var(--cover-ring);
  }

  .thumb-empty {
    background: var(--bg-3);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .title-cell {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .title-text {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .genre {
    color: var(--fg-2);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .center {
    display: flex;
    justify-content: center;
  }

  .hours {
    font-family: var(--font-mono);
    font-size: 12px;
    text-align: right;
    color: var(--fg-1);
  }

  .hours.dim {
    color: var(--fg-3);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .action-status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-3);
  }

  .action-launch {
    display: none;
  }

  .row:hover .action-launch {
    display: flex;
  }

  .row:hover .action-status {
    display: none;
  }
</style>
