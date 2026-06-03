<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    toast,
    onDismiss,
  }: {
    toast: { title: string; platform: string } | null;
    onDismiss: () => void;
  } = $props();

  $effect(() => {
    if (!toast) return;
    const t = setTimeout(onDismiss, 2600);
    return () => clearTimeout(t);
  });
</script>

{#if toast}
  <div class="toast">
    <span class="play-icon">
      <Icon name="play" size={15} color="var(--accent)" />
    </span>
    <div>
      <div class="toast-title">Launching {toast.title}</div>
      <div class="toast-sub">Handing off to {toast.platform}…</div>
    </div>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 22px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 80;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 11px 16px;
    border-radius: var(--radius-md);
    background: var(--bg-2);
    border: 1px solid var(--border-2);
    box-shadow: var(--shadow-lg);
    white-space: nowrap;
  }

  .play-icon {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }

  /* fill the play triangle */
  .play-icon :global(svg) {
    fill: currentColor;
  }

  .toast-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--fg-1);
  }

  .toast-sub {
    font-size: 11.5px;
    color: var(--fg-3);
  }
</style>
