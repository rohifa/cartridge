<script lang="ts">
  import Icon from './Icon.svelte';
  import PlatformDot from './PlatformDot.svelte';
  import type { Game } from '$lib/types.js';

  let {
    game,
    selected = false,
    radius = 'var(--radius-md)',
  }: {
    game: Game;
    selected?: boolean;
    radius?: string;
  } = $props();
</script>

<div
  class="cover"
  style="
    border-radius: {radius};
    box-shadow: {selected
      ? '0 0 0 2px var(--accent), 0 0 0 5px var(--accent-soft)'
      : 'var(--cover-ring), var(--shadow-sm)'};
  "
>
  {#if game.cover}
    <img src="/covers/{game.cover}.png" alt="" />
  {:else}
    <div class="fallback">
      <Icon name="gamepad-2" size={22} color="var(--fg-3)" />
      <span class="fallback-title">{game.title}</span>
      <span class="fallback-sub">no cover art</span>
    </div>
  {/if}
  <span class="plat-dot">
    <PlatformDot platform={game.platform} />
  </span>
</div>

<style>
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: var(--bg-3);
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .fallback {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 10px;
    text-align: center;
  }

  .fallback :global(svg) {
    opacity: 0.6;
  }

  .fallback-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--fg-2);
    line-height: 1.25;
  }

  .fallback-sub {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--fg-3);
  }

  .plat-dot {
    position: absolute;
    top: 7px;
    left: 7px;
  }
</style>
