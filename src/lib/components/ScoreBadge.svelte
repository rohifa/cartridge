<script lang="ts">
  import { scoreTone } from '$lib/utils.js';

  let {
    value,
    size = 'md',
    unit = '',
  }: {
    value: number | null;
    size?: 'sm' | 'md' | 'lg';
    unit?: string;
  } = $props();

  const dim = $derived(size === 'lg' ? 40 : size === 'sm' ? 26 : 32);
  const fs  = $derived(size === 'lg' ? 18 : size === 'sm' ? 12 : 15);
  const t   = $derived(scoreTone(value));
</script>

<span style="
  min-width: {dim}px;
  height: {dim}px;
  padding: 0 3px;
  box-sizing: border-box;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: {t.bg};
  color: {t.fg};
  border: 1px solid {t.bd};
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: {fs}px;
  font-feature-settings: 'tnum' 1;
">
  {value == null ? '–' : value}{#if unit && value != null}<span style="font-size:0.7em; font-weight:600; opacity:0.85; margin-left:1px">{unit}</span>{/if}
</span>
