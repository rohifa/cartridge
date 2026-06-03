<script lang="ts">
  let {
    value,
    size = 13,
    onSet,
  }: {
    value: number;
    size?: number;
    onSet?: (n: number) => void;
  } = $props();
</script>

<span style="display:inline-flex; gap:2px; font-size:{size}px; line-height:1; cursor:{onSet ? 'pointer' : 'default'}">
  {#each [1, 2, 3, 4, 5] as n}
    <span
      role={onSet ? 'button' : undefined}
      tabindex={onSet ? 0 : undefined}
      onclick={onSet ? () => onSet(n === value ? 0 : n) : undefined}
      onkeydown={onSet ? (e) => e.key === 'Enter' && onSet(n === value ? 0 : n) : undefined}
      style="color:{n <= value ? 'var(--gold-500)' : 'var(--fg-3)'}"
    >★</span>
  {/each}
</span>
