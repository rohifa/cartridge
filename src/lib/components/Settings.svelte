<script lang="ts">
  import Icon from './Icon.svelte';
  import PlatIcon from './PlatIcon.svelte';
  import type { Account, CartridgeData, Prefs } from '$lib/types.js';

  let {
    data,
    syncingIds,
    prefs,
    theme,
    onSync,
    onSetPref,
    onTheme,
    onAddAccount,
    onEditAccount,
    onRemoveAccount,
    onReauth,
  }: {
    data: CartridgeData;
    syncingIds: Set<string>;
    prefs: Prefs;
    theme: 'dark' | 'light';
    onSync: (id: string) => void;
    onSetPref: <K extends keyof Prefs>(key: K, val: Prefs[K]) => void;
    onTheme: (t: 'dark' | 'light') => void;
    onAddAccount: () => void;
    onEditAccount: (account: Account) => void;
    onRemoveAccount: (id: string) => void;
    onReauth: (platform: string) => void;
  } = $props();
</script>

<div class="settings">
  <div class="inner">
    <div class="page-title">Settings &amp; sync</div>
    <p class="page-desc">
      Connect a storefront to pull its library in. Your ratings, tags, notes, links and custom
      fields are kept locally and are never overwritten by a sync.
    </p>

    <!-- Connections -->
    <div class="section-title">Connections</div>
    <div class="account-list">
      {#each data.accounts as account}
        {@const syncing = syncingIds.has(account.id)}
        {@const stale   = account.status === 'stale'}
        {@const errored = account.status === 'error'}
        <div class="account-card">
          <div class="account-head">
            <div class="plat-icon">
              <PlatIcon platform={account.platform} size={20} />
            </div>
            <div class="account-info">
              <div class="account-name">{account.name}</div>
              <div class="account-sub">{account.handle} · {account.count} games</div>
            </div>
            <span
              class="sync-badge"
              class:badge-syncing={syncing}
              class:badge-error={!syncing && errored}
              class:badge-stale={!syncing && !errored && stale}
              class:badge-ok={!syncing && !errored && !stale}
            >
              {#if syncing}
                <span class="spin"><Icon name="loader" size={12} /></span>
                syncing…
              {:else if errored}
                <span class="badge-dot" style="background:var(--danger)"></span>
                session expired
              {:else}
                <span class="badge-dot" style="background:{stale ? 'var(--warn)' : 'var(--ok)'}"></span>
                {stale ? `stale · ${account.synced}` : `synced ${account.synced}`}
              {/if}
            </span>
          </div>

          <!-- Platform-specific info -->
          {#if account.platform === 'steam'}
            <div class="fields-grid">
              <div class="field">
                <span class="field-label">Steam API key</span>
                <div class="field-val">••••••••••••3F9A</div>
              </div>
              <div class="field">
                <span class="field-label">Steam ID</span>
                <div class="field-val mono">76561198042…</div>
              </div>
            </div>
          {:else if account.platform === 'gog'}
            <div class="field">
              <span class="field-label">GOG account</span>
              <div class="field-val">Connected via OAuth</div>
            </div>
          {:else if account.platform === 'epic'}
            <div class="epic-notice">
              <Icon name="info" size={14} color="var(--fg-3)" />
              Epic doesn't expose playtime — games and metadata only.
            </div>
          {/if}

          <!-- Actions -->
          <div class="account-actions">
            {#if errored}
              <button class="reauth-btn" onclick={() => onReauth(account.platform)}>
                <Icon name="log-in" size={14} />
                Re-authenticate
              </button>
            {:else}
              <button
                class="sync-btn"
                disabled={syncing}
                onclick={() => onSync(account.id)}
              >
                <span class:spin={syncing}>
                  <Icon name="refresh-cw" size={14} />
                </span>
                {syncing ? 'Syncing…' : 'Sync now'}
              </button>
            {/if}
            {#if account.platform === 'steam'}
              <button class="edit-btn" onclick={() => onEditAccount(account)}>Edit</button>
            {/if}
            <button class="remove-btn" onclick={() => onRemoveAccount(account.id)}>Remove</button>
          </div>
        </div>
      {/each}

      <button class="add-account-btn" onclick={onAddAccount}>
        <Icon name="plus" size={16} color="var(--fg-2)" />
        Add an account
      </button>
    </div>

    <!-- Appearance -->
    <div class="pref-section">
      <div class="pref-section-title">Appearance</div>
      <div class="pref-card">
        <div class="pref-row">
          <div class="pref-text">
            <div class="pref-label">Theme</div>
            <div class="pref-desc">Dark is the primary theme; light is calm warm paper.</div>
          </div>
          <div class="seg">
            {#each [['dark','Dark','moon'],['light','Light','sun']] as [k, l, ico]}
              <button
                class="seg-btn"
                class:active={k === theme}
                onclick={() => onTheme(k as 'dark' | 'light')}
              >
                <Icon name={ico} size={14} color={k === theme ? 'var(--fg-1)' : 'var(--fg-3)'} />
                {l}
              </button>
            {/each}
          </div>
        </div>
        <div class="pref-row last">
          <div class="pref-text">
            <div class="pref-label">Default library view</div>
            <div class="pref-desc">How the library opens.</div>
          </div>
          <div class="seg">
            {#each [['grid','Grid','layout-grid'],['list','List','list']] as [k, l, ico]}
              <button
                class="seg-btn"
                class:active={k === prefs.defaultView}
                onclick={() => onSetPref('defaultView', k as 'grid' | 'list')}
              >
                <Icon name={ico} size={14} color={k === prefs.defaultView ? 'var(--fg-1)' : 'var(--fg-3)'} />
                {l}
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <!-- Library -->
    <div class="pref-section">
      <div class="pref-section-title">Library</div>
      <div class="pref-card">
        <div class="pref-row">
          <div class="pref-text">
            <div class="pref-label">Show critic scores</div>
            <div class="pref-desc">Metacritic, OpenCritic and user-score badges in lists.</div>
          </div>
          <button
            class="toggle"
            class:on={prefs.showScores}
            onclick={() => onSetPref('showScores', !prefs.showScores)}
            aria-pressed={prefs.showScores}
            aria-label="Show critic scores"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>
        <div class="pref-row">
          <div class="pref-text">
            <div class="pref-label">Confirm before launching</div>
            <div class="pref-desc">Ask before handing off to a storefront launcher.</div>
          </div>
          <button
            class="toggle"
            class:on={prefs.confirmLaunch}
            onclick={() => onSetPref('confirmLaunch', !prefs.confirmLaunch)}
            aria-pressed={prefs.confirmLaunch}
            aria-label="Confirm before launching"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>
        <div class="pref-row last">
          <div class="pref-text">
            <div class="pref-label">Auto-sync on open</div>
            <div class="pref-desc">Refresh all connected libraries when Cartridge starts.</div>
          </div>
          <button
            class="toggle"
            class:on={prefs.autoSync}
            onclick={() => onSetPref('autoSync', !prefs.autoSync)}
            aria-pressed={prefs.autoSync}
            aria-label="Auto-sync on open"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>
      </div>
    </div>

    <!-- Local data -->
    <div class="pref-section">
      <div class="pref-section-title">Local data</div>
      <div class="pref-card">
        <div class="pref-row">
          <div class="pref-text">
            <div class="pref-label">Library file</div>
            <div class="pref-desc file-path">
              ~/Library/Application Support/Cartridge/library.db
            </div>
          </div>
          <button class="action-btn">Reveal in Finder</button>
        </div>
        <div class="pref-row last">
          <div class="pref-text">
            <div class="pref-label">Export library</div>
            <div class="pref-desc">Save a JSON snapshot of your games and custom data.</div>
          </div>
          <button class="action-btn">Export JSON</button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .settings {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-1);
  }

  .inner {
    max-width: 660px;
    margin: 0 auto;
    padding: 34px 28px 64px;
  }

  .page-title {
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .page-desc {
    font-size: 13px;
    color: var(--fg-2);
    margin: 6px 0 0;
    line-height: 1.5;
  }

  /* ── Connections ── */
  .section-title {
    font-size: 15px;
    font-weight: 600;
    margin-top: 30px;
  }

  .account-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin-top: 12px;
  }

  .account-card {
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-lg);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .account-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .plat-icon {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    flex: none;
    background: var(--bg-3);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-2);
  }

  .account-info {
    flex: 1;
    min-width: 0;
  }

  .account-name {
    font-size: 15px;
    font-weight: 600;
  }

  .account-sub {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-3);
  }

  .sync-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 4px 9px;
    border-radius: var(--radius-full);
  }

  .badge-syncing {
    color: var(--gold-400);
    background: var(--accent-soft);
  }

  .badge-error {
    color: var(--danger);
    background: oklch(0.45 0.22 25 / 0.15);
  }

  .badge-stale {
    color: var(--warn);
    background: oklch(0.74 0.14 65 / 0.15);
  }

  .badge-ok {
    color: var(--ok);
    background: oklch(0.72 0.14 150 / 0.15);
  }

  .badge-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .field-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .field-val {
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    font-size: 13px;
    font-family: var(--font-ui);
    color: var(--fg-1);
  }

  .field-val.mono {
    font-family: var(--font-mono);
  }

  .epic-notice {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--fg-3);
  }

  .account-actions {
    display: flex;
    gap: 9px;
    align-items: center;
  }

  .sync-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 13px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-2);
    background: var(--bg-3);
    color: var(--fg-1);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .sync-btn:disabled {
    cursor: default;
    opacity: 0.7;
  }

  .reauth-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 13px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--danger);
    background: oklch(0.45 0.22 25 / 0.1);
    color: var(--danger);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .edit-btn {
    height: 30px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-2);
    background: transparent;
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 12px;
    cursor: pointer;
  }

  .remove-btn {
    height: 30px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--danger);
    font-family: var(--font-ui);
    font-size: 12px;
    cursor: pointer;
  }

  .add-account-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 46px;
    border-radius: var(--radius-lg);
    border: 1px dashed var(--border-2);
    background: transparent;
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  /* ── Pref sections ── */
  .pref-section {
    margin-top: 30px;
  }

  .pref-section-title {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 12px;
  }

  .pref-card {
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-lg);
    padding: 4px 18px;
  }

  .pref-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 13px 0;
    border-bottom: 1px solid var(--border-1);
  }

  .pref-row.last {
    border-bottom: none;
  }

  .pref-text {
    flex: 1;
    min-width: 0;
  }

  .pref-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-1);
  }

  .pref-desc {
    font-size: 12px;
    color: var(--fg-3);
    margin-top: 2px;
    line-height: 1.45;
  }

  .file-path {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  /* ── Segmented control ── */
  .seg {
    display: inline-flex;
    gap: 2px;
    background: var(--bg-3);
    padding: 2px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-1);
    flex: none;
  }

  .seg-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 13px;
    border-radius: var(--radius-xs);
    cursor: pointer;
    border: none;
    background: transparent;
    color: var(--fg-3);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
  }

  .seg-btn.active {
    background: var(--bg-1);
    color: var(--fg-1);
    font-weight: 600;
    box-shadow: var(--shadow-sm);
  }

  /* ── Toggle ── */
  .toggle {
    width: 40px;
    height: 23px;
    flex: none;
    border-radius: var(--radius-full);
    border: none;
    cursor: pointer;
    padding: 2px;
    background: var(--bg-3);
    display: flex;
    align-items: center;
    transition: background 0.15s;
    position: relative;
  }

  .toggle.on {
    background: var(--accent);
  }

  .toggle-thumb {
    width: 19px;
    height: 19px;
    border-radius: 50%;
    background: #fff;
    box-shadow: var(--shadow-sm);
    display: block;
    transform: translateX(0);
    transition: transform 0.15s;
    flex: none;
  }

  .toggle.on .toggle-thumb {
    transform: translateX(17px);
  }

  /* ── Action buttons ── */
  .action-btn {
    height: 30px;
    padding: 0 13px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-2);
    background: var(--bg-3);
    color: var(--fg-1);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    flex: none;
  }

  .spin {
    display: inline-flex;
    animation: crt-spin 1s linear infinite;
  }
</style>
