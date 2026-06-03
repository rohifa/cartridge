<script lang="ts">
  import * as icons from '@lucide/svelte';

  let {
    name,
    size = 16,
    strokeWidth = 1.75,
    color,
    fill,
  }: {
    name: string;
    size?: number;
    strokeWidth?: number;
    color?: string;
    fill?: string;
  } = $props();

  function toPascal(s: string): string {
    return s.split('-').map(p => p.charAt(0).toUpperCase() + p.slice(1)).join('');
  }

  const Comp = $derived((icons as Record<string, any>)[toPascal(name)]);
</script>

{#if Comp}
  <Comp {size} {strokeWidth} {color} {fill} />
{/if}
