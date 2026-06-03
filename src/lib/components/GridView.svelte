<script lang="ts">
  import Cover from './Cover.svelte';
  import StatusPill from './StatusPill.svelte';
  import ScoreCluster from './ScoreCluster.svelte';
  import LaunchButton from './LaunchButton.svelte';
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
</script>

<div class="grid">
  {#each games as g (g.id)}
    <div
      class="card"
      onclick={() => onSelect(g.id)}
      role="button"
      tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onSelect(g.id)}
    >
      <div class="cover-wrap">
        <Cover game={g} selected={g.id === selectedId} />
        {#if g.status}
          <span class="status-badge">
            <StatusPill status={g.status} size="sm" />
          </span>
        {/if}
        <div class="hover-overlay">
          <LaunchButton game={g} {onLaunch} />
        </div>
      </div>

      <div class="card-title">{g.title}</div>

      {#if showScores}
        <div class="card-scores">
          <ScoreCluster game={g} size="sm" />
        </div>
      {/if}

      <div class="card-meta">
        {#if g.rating > 0}
          <span class="card-rating">★ {g.rating}</span>
        {/if}
        <span>{fmtHours(g.playtime)}</span>
      </div>
    </div>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 20px;
    padding: 22px;
    align-content: start;
  }

  .card {
    cursor: pointer;
  }

  .cover-wrap {
    position: relative;
  }

  .status-badge {
    position: absolute;
    top: 7px;
    right: 7px;
  }

  .hover-overlay {
    position: absolute;
    inset: 0;
    border-radius: var(--radius-md);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 10px;
    background: linear-gradient(to top, oklch(0 0 0 / 0.6), transparent 55%);
    opacity: 0;
    transition: opacity 0.14s;
  }

  .card:hover .hover-overlay {
    opacity: 1;
  }

  .card-title {
    font-size: 12.5px;
    font-weight: 600;
    margin-top: 8px;
    line-height: 1.25;
    color: var(--fg-1);
  }

  .card-scores {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 6px;
  }

  .card-meta {
    display: flex;
    gap: 8px;
    margin-top: 6px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-3);
    white-space: nowrap;
  }

  .card-rating {
    color: var(--gold-500);
    flex: none;
  }
</style>
