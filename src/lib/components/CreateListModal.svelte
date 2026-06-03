<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from './Icon.svelte';
  import { LISTDOT } from '$lib/constants.js';
  import type { ListColor, ListKind } from '$lib/types.js';

  const LIST_COLORS: ListColor[] = ['green', 'blue', 'purple', 'amber', 'teal', 'red'];

  let {
    fromFilters = false,
    activeFilterCount = 0,
    onClose,
    onCreate,
  }: {
    fromFilters?: boolean;
    activeFilterCount?: number;
    onClose: () => void;
    onCreate: (list: { name: string; color: ListColor; kind: ListKind; icon: string }) => void;
  } = $props();

  const canSmart = $derived(activeFilterCount > 0);

  // Capture initial kind once when the modal opens — intentionally not reactive.
  const initialKind: ListKind = untrack(() => fromFilters && activeFilterCount > 0 ? 'smart' : 'manual');

  let name  = $state('');
  let color = $state<ListColor>('amber');
  let kind  = $state<ListKind>(initialKind);

  function create() {
    if (!name.trim()) return;
    onCreate({ name: name.trim(), color, kind, icon: kind === 'smart' ? 'zap' : 'list' });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') create();
    if (e.key === 'Escape') onClose();
  }
</script>

<!-- Backdrop -->
<div
  class="backdrop"
  onclick={onClose}
  role="presentation"
  onkeydown={onKeydown}
>
  <!-- Modal panel -->
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="New list"
    tabindex="-1"
  >
    <!-- Header -->
    <div class="modal-head">
      <div class="modal-title">New list</div>
      <div class="modal-desc">
        Group games your way. A manual list holds whatever you add; a smart list fills itself from rules.
      </div>
    </div>

    <!-- Body -->
    <div class="modal-body">
      <!-- Name -->
      <label class="field">
        <span class="field-label">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          autofocus
          class="name-input"
          type="text"
          placeholder="e.g. Cozy winter games"
          bind:value={name}
          onkeydown={(e) => { if (e.key === 'Enter') create(); }}
        />
      </label>

      <!-- Type -->
      <div class="type-block">
        <span class="field-label">Type</span>
        <div class="type-options">
          {#each [
            { k: 'manual' as const, t: 'List',       d: 'Pick games by hand',     ic: 'list' },
            { k: 'smart'  as const, t: 'Smart list',  d: 'Auto-fill from filters', ic: 'zap'  },
          ] as opt}
            {@const on       = kind === opt.k}
            {@const disabled = opt.k === 'smart' && !canSmart}
            <button
              class="type-btn"
              class:active={on}
              disabled={disabled}
              onclick={() => { if (!disabled) kind = opt.k; }}
            >
              <div class="type-btn-head" style="color:{on ? 'var(--accent)' : 'var(--fg-1)'}">
                <Icon name={opt.ic} size={14} color={on ? 'var(--accent)' : 'var(--fg-1)'} />
                {opt.t}
              </div>
              <div class="type-btn-desc">{opt.d}</div>
            </button>
          {/each}
        </div>
        {#if kind === 'smart'}
          <div class="smart-hint">
            <Icon name="sliders-horizontal" size={13} color="var(--accent)" />
            Uses your {activeFilterCount} active filter{activeFilterCount === 1 ? '' : 's'} and keeps updating.
          </div>
        {/if}
      </div>

      <!-- Colour -->
      <div class="color-block">
        <span class="field-label">Colour</span>
        <div class="color-swatches">
          {#each LIST_COLORS as c}
            <button
              class="swatch"
              class:selected={color === c}
              style="background:{LISTDOT[c]}"
              onclick={() => color = c}
              aria-label={c}
            ></button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="modal-foot">
      <button class="cancel-btn" onclick={onClose}>Cancel</button>
      <button
        class="create-btn"
        disabled={!name.trim()}
        onclick={create}
      >Create list</button>
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
    width: 420px;
    max-width: 100%;
    background: var(--bg-2);
    border: 1px solid var(--border-2);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }

  .modal-head {
    padding: 20px 22px 0;
  }

  .modal-title {
    font-size: 17px;
    font-weight: 600;
  }

  .modal-desc {
    font-size: 12.5px;
    color: var(--fg-2);
    margin-top: 4px;
    line-height: 1.5;
  }

  .modal-body {
    padding: 18px 22px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .field-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .name-input {
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 9px 11px;
    color: var(--fg-1);
    font-family: var(--font-ui);
    font-size: 13.5px;
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }

  .name-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  /* Type selector */
  .type-block {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .type-options {
    display: flex;
    gap: 9px;
    margin-top: 8px;
  }

  .type-btn {
    flex: 1;
    text-align: left;
    padding: 11px 13px;
    border-radius: var(--radius-md);
    cursor: pointer;
    border: 1px solid var(--border-2);
    background: transparent;
    transition: border-color 0.1s, background 0.1s;
  }

  .type-btn.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .type-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .type-btn-head {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    font-weight: 600;
  }

  .type-btn-desc {
    font-size: 11.5px;
    color: var(--fg-3);
    margin-top: 3px;
  }

  .smart-hint {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 9px;
    font-size: 11.5px;
    color: var(--fg-2);
  }

  /* Colour swatches */
  .color-block {
    display: flex;
    flex-direction: column;
    gap: 9px;
  }

  .color-swatches {
    display: flex;
    gap: 10px;
  }

  .swatch {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    cursor: pointer;
    border: 2px solid transparent;
    box-shadow: none;
    transition: border-color 0.1s, box-shadow 0.1s;
  }

  .swatch.selected {
    border-color: var(--fg-1);
    box-shadow: 0 0 0 2px var(--bg-2);
  }

  /* Footer */
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

  .create-btn {
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

  .create-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
