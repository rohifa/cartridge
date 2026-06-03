<script lang="ts">
  import { TAGCOLOR, TAGBG } from '$lib/constants.js';
  import Icon from './Icon.svelte';

  let {
    label,
    onRemove,
  }: {
    label: string;
    onRemove?: () => void;
  } = $props();
</script>

<span style="
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  padding: 3px 9px;
  border-radius: var(--radius-full);
  color: {TAGCOLOR[label] ?? 'var(--fg-2)'};
  background: {TAGBG[label] ?? 'var(--bg-3)'};
">
  {label}
  {#if onRemove}
    <span
      onclick={onRemove}
      role="button"
      tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && onRemove()}
      style="display:inline-flex; opacity:0.5; cursor:pointer"
    >
      <Icon name="x" size={11} />
    </span>
  {/if}
</span>
