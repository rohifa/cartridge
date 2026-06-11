<script lang="ts">
  import ScoreBadge from './ScoreBadge.svelte';
  import Icon from './Icon.svelte';
  import { LISTDOT } from '$lib/constants.js';
  import { totalPlaytime } from '$lib/utils.js';
  import type { Game, GameList } from '$lib/types.js';

  let {
    games,
    listMeta = null,
  }: {
    games: Game[];
    listMeta?: GameList | null;
  } = $props();

  const totalGames   = $derived(games.length);
  const tracked      = $derived(games.filter(g => totalPlaytime(g) != null));
  const totalH       = $derived(tracked.reduce((s, g) => s + (totalPlaytime(g) ?? 0), 0));
  const rated        = $derived(games.filter(g => g.rating > 0));
  const avgRating    = $derived(rated.length ? rated.reduce((s, g) => s + g.rating, 0) / rated.length : null);
  const withMc       = $derived(games.filter(g => g.metacritic != null));
  const avgMc        = $derived(withMc.length
    ? Math.round(withMc.reduce((s, g) => s + (g.metacritic ?? 0), 0) / withMc.length)
    : null);
</script>

<div class="agg-bar">
  {#if listMeta}
    <div class="list-meta">
      <span class="list-dot" style="background:{LISTDOT[listMeta.color]}"></span>
      <span class="list-name">{listMeta.name}</span>
      {#if listMeta.kind === 'smart'}
        <span class="kind-badge smart">
          <Icon name="zap" size={11} color="var(--accent)" />
          SMART
        </span>
      {:else}
        <span class="kind-badge manual">MANUAL</span>
      {/if}
    </div>
  {/if}

  <div class="stat">
    <span class="stat-label">Games</span>
    <span class="stat-val">{totalGames}</span>
  </div>

  <div class="stat">
    <span class="stat-label">Total playtime</span>
    <span class="stat-val">{Math.round(totalH)} h</span>
  </div>

  <div class="stat">
    <span class="stat-label">Avg rating</span>
    <span class="stat-val">
      {#if avgRating != null}
        <span class="star">★</span>{avgRating.toFixed(1)}
      {:else}
        —
      {/if}
    </span>
  </div>

  <div class="stat">
    <span class="stat-label">Avg Metacritic</span>
    <span class="stat-val"><ScoreBadge value={avgMc} size="sm" /></span>
  </div>
</div>

<style>
  .agg-bar {
    display: flex;
    align-items: center;
    gap: 30px;
    padding: 13px 22px;
    border-bottom: 1px solid var(--border-1);
    background: var(--bg-1);
    flex: none;
  }

  .list-meta {
    display: flex;
    align-items: center;
    gap: 9px;
    padding-right: 30px;
    border-right: 1px solid var(--border-1);
  }

  .list-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
  }

  .list-name {
    font-size: 14px;
    font-weight: 600;
  }

  .kind-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 600;
  }

  .kind-badge.smart {
    color: var(--accent);
  }

  .kind-badge.manual {
    color: var(--fg-3);
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stat-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .stat-val {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 15px;
    font-weight: 600;
    color: var(--fg-1);
  }

  .star {
    color: var(--gold-500);
  }
</style>
