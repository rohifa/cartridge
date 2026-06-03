<script lang="ts">
  import { STATUS_META } from '$lib/constants.js';
  import Icon from './Icon.svelte';
  import type { GameStatus } from '$lib/types.js';

  let {
    status,
    size = 'md',
  }: {
    status: GameStatus;
    size?: 'sm' | 'md';
  } = $props();

  const m   = $derived(STATUS_META[status]);
  const pad = $derived(size === 'sm' ? '2px 8px' : '4px 11px');
  const fs  = $derived(size === 'sm' ? 10 : 11);
</script>

{#if m}
  <span style="
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: {pad};
    border-radius: var(--radius-full);
    font-size: {fs}px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: {m.color};
    background: {m.bg};
  ">
    <Icon name={m.icon} size={fs + 1} />
    {m.label}
  </span>
{/if}
