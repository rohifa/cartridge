<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from './Icon.svelte';
  import { PLAT } from '$lib/constants.js';
  import { addSteamAccount } from '$lib/api.js';
  import type { Account, Platform } from '$lib/types.js';

  let {
    editAccount = null,
    onClose,
    onAdd,
  }: {
    editAccount?: { platform: Platform; steamId: string } | null;
    onClose: () => void;
    onAdd: (account: Account) => void;
  } = $props();

  const isEdit = $derived(editAccount != null);

  let platform = $state<Platform>(untrack(() => editAccount?.platform ?? 'steam'));
  let apiKey   = $state('');
  let steamId  = $state(untrack(() => editAccount?.steamId ?? ''));
  let error    = $state('');
  let saving   = $state(false);

  async function connect() {
    if (!apiKey.trim() || !steamId.trim()) return;
    error = '';
    saving = true;
    try {
      const account = await addSteamAccount(apiKey.trim(), steamId.trim(), editAccount?.steamId);
      onAdd(account);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  const canSubmit = $derived(apiKey.trim().length > 0 && steamId.trim().length > 0 && !saving);
</script>

<div
  class="backdrop"
  onclick={onClose}
  role="presentation"
  onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
>
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label={isEdit ? 'Edit account' : 'Add account'}
    tabindex="-1"
  >
    <div class="modal-head">
      <div class="modal-title">{isEdit ? 'Edit account' : 'Add an account'}</div>
      <div class="modal-desc">
        {#if isEdit}
          Update your credentials. Your ratings, tags, and notes are never overwritten by a sync.
        {:else}
          Connect a storefront to pull your library. Your ratings, tags, and notes are never overwritten by a sync.
        {/if}
      </div>
    </div>

    <div class="modal-body">
      <!-- Platform picker — locked when editing -->
      <div class="field-group">
        <span class="field-label">Platform</span>
        <div class="plat-options">
          {#each (['steam', 'gog', 'epic'] as Platform[]) as p}
            {@const supported = p === 'steam'}
            {@const locked   = isEdit && p !== platform}
            <button
              class="plat-btn"
              class:active={platform === p}
              disabled={!supported || locked}
              onclick={() => { if (supported && !isEdit) platform = p; }}
            >
              <span class="plat-dot" style="background:{PLAT[p].color}"></span>
              {PLAT[p].name}
              {#if !supported && !isEdit}<span class="soon">soon</span>{/if}
            </button>
          {/each}
        </div>
      </div>

      <!-- Steam fields -->
      {#if platform === 'steam'}
        <label class="field">
          <span class="field-label">Steam API key</span>
          <input
            class="text-input"
            type="text"
            placeholder={isEdit ? 'Re-enter your API key to save' : '32-character hex key'}
            bind:value={apiKey}
            onkeydown={(e) => { if (e.key === 'Enter') connect(); }}
            autocomplete="off"
            spellcheck="false"
          />
          <span class="field-hint">
            {#if isEdit}
              For security, the key isn't stored in the UI — re-enter it to update.
            {:else}
              Get yours at <span class="mono">steamcommunity.com/dev/apikey</span>
            {/if}
          </span>
        </label>

        <label class="field">
          <span class="field-label">Steam ID</span>
          <input
            class="text-input mono"
            type="text"
            placeholder="17-digit SteamID64"
            bind:value={steamId}
            onkeydown={(e) => { if (e.key === 'Enter') connect(); }}
            autocomplete="off"
            spellcheck="false"
          />
          <span class="field-hint">
            Find it at <span class="mono">steamid.io</span> or in your profile URL
          </span>
        </label>
      {/if}

      {#if error}
        <div class="error-banner">
          <Icon name="alert-circle" size={13} color="var(--danger)" />
          {error}
        </div>
      {/if}
    </div>

    <div class="modal-foot">
      <button class="cancel-btn" onclick={onClose}>Cancel</button>
      <button
        class="connect-btn"
        disabled={!canSubmit}
        onclick={connect}
      >
        {#if saving}
          <span class="spin"><Icon name="loader" size={13} color="var(--fg-on-accent)" /></span>
          {isEdit ? 'Saving…' : 'Connecting…'}
        {:else}
          {isEdit ? 'Save' : 'Connect'}
        {/if}
      </button>
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
    gap: 16px;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .plat-options {
    display: flex;
    gap: 8px;
  }

  .plat-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 38px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-2);
    background: transparent;
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 0.1s, background 0.1s;
  }

  .plat-btn.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--fg-1);
  }

  .plat-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .plat-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
  }

  .soon {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--fg-3);
    background: var(--bg-3);
    padding: 1px 5px;
    border-radius: var(--radius-full);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .text-input {
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 9px 11px;
    color: var(--fg-1);
    font-family: var(--font-ui);
    font-size: 13px;
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }

  .text-input.mono {
    font-family: var(--font-mono);
  }

  .text-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .field-hint {
    font-size: 11px;
    color: var(--fg-3);
    line-height: 1.4;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: oklch(0.45 0.22 25 / 0.15);
    border: 1px solid oklch(0.45 0.22 25 / 0.30);
    font-size: 12px;
    color: var(--danger);
    line-height: 1.45;
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

  .connect-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
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

  .connect-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .spin {
    display: inline-flex;
    animation: crt-spin 1s linear infinite;
  }
</style>
