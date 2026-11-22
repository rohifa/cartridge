<script lang="ts">
  let {
    title,
    body = '',
    confirmLabel,
    danger = false,
    onConfirm,
    onCancel,
  }: {
    title: string;
    body?: string;
    confirmLabel: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  // Enter and a click can both land on the confirm button; act once.
  let done = false;
  function confirm() {
    if (done) return;
    done = true;
    onConfirm();
  }
  function cancel() {
    if (done) return;
    done = true;
    onCancel();
  }

  function focusOnMount(node: HTMLElement) { node.focus(); }

  // Window-level so the keys work wherever focus is. A focused button inside
  // the modal keeps its own Enter (so Enter on Cancel cancels).
  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancel();
    } else if (e.key === 'Enter') {
      const t = e.target as HTMLElement | null;
      if (t?.tagName === 'BUTTON' && t.closest('.modal')) return;
      e.preventDefault();
      confirm();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Backdrop -->
<div class="backdrop" onclick={cancel} role="presentation">
  <!-- Modal panel -->
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    aria-describedby={body ? 'confirm-body' : undefined}
    tabindex="-1"
  >
    <div class="modal-head">
      <div class="modal-title" id="confirm-title">{title}</div>
      {#if body}
        <div class="modal-desc" id="confirm-body">{body}</div>
      {/if}
    </div>

    <div class="modal-foot">
      <button class="cancel-btn" onclick={cancel}>Cancel</button>
      <button
        class="confirm-btn"
        class:danger
        onclick={confirm}
        use:focusOnMount
      >{confirmLabel}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: oklch(0 0 0 / 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .modal {
    width: 380px;
    max-width: 100%;
    background: var(--bg-2);
    border: 1px solid var(--border-2);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    outline: none;
  }

  .modal-head {
    padding: 20px 22px 18px;
  }

  .modal-title {
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .modal-desc {
    font-size: 12.5px;
    color: var(--fg-2);
    margin-top: 6px;
    line-height: 1.5;
  }

  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 9px;
    padding: 14px 22px;
    border-top: 1px solid var(--border-1);
    background: var(--bg-1);
  }

  .cancel-btn {
    height: 32px;
    padding: 0 15px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-2);
    background: transparent;
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .confirm-btn {
    height: 32px;
    padding: 0 17px;
    border-radius: var(--radius-sm);
    border: none;
    background: var(--accent);
    color: var(--fg-on-accent);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .confirm-btn.danger {
    background: var(--danger);
    color: oklch(0.98 0.01 25);
  }

  .cancel-btn:focus-visible,
  .confirm-btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>
