<script lang="ts">
  import { PLAT } from '$lib/constants.js';
  import Icon from './Icon.svelte';
  import type { Game } from '$lib/types.js';

  let {
    game,
    size = 'md',
    onLaunch,
  }: {
    game: Game;
    size?: 'md' | 'lg';
    onLaunch?: (g: Game) => void;
  } = $props();

  let hover = $state(false);
  const big = $derived(size === 'lg');
</script>

{#if !game.installed}
  <button
    disabled
    title="Not installed — install from {PLAT[game.platform].name}"
    style="
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 7px;
      height: {big ? 44 : 30}px;
      padding: 0 {big ? 22 : 13}px;
      border-radius: var(--radius-sm);
      border: 1px solid var(--border-1);
      background: var(--bg-3);
      color: var(--fg-3);
      font-family: var(--font-ui);
      font-size: {big ? 14 : 12}px;
      font-weight: 600;
      cursor: default;
    "
  >
    <Icon name="download" size={big ? 17 : 14} />
    Not installed
  </button>
{:else}
  <button
    onclick={(e) => { e.stopPropagation(); onLaunch?.(game); }}
    onmouseenter={() => (hover = true)}
    onmouseleave={() => (hover = false)}
    title="Launch on {PLAT[game.platform].name}"
    class="launch-btn"
    style="
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      height: {big ? 44 : 30}px;
      padding: 0 {big ? 26 : 15}px;
      border-radius: var(--radius-sm);
      border: none;
      cursor: pointer;
      background: {hover ? 'var(--accent-hover)' : 'var(--accent)'};
      color: var(--fg-on-accent);
      font-family: var(--font-ui);
      font-size: {big ? 15 : 12.5}px;
      font-weight: 700;
      letter-spacing: -0.01em;
      box-shadow: var(--shadow-sm);
      transition: background 0.12s;
    "
  >
    <Icon name="play" size={big ? 18 : 15} />
    Launch{big ? ` on ${PLAT[game.platform].name}` : ''}
  </button>
{/if}

<style>
  /* Fill the play triangle — override lucide's default fill:none */
  .launch-btn :global(svg) {
    fill: currentColor;
  }
</style>
