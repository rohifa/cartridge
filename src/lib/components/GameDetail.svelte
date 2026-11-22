<script lang="ts">
  import Cover from './Cover.svelte';
  import LaunchButton from './LaunchButton.svelte';
  import StatusPill from './StatusPill.svelte';
  import ScoreBadge from './ScoreBadge.svelte';
  import Stars from './Stars.svelte';
  import Tag from './Tag.svelte';
  import Icon from './Icon.svelte';
  import PlatIcon from './PlatIcon.svelte';
  import { PLAT, STATUS_META, SCORE_SOURCES, LISTDOT } from '$lib/constants.js';
  import { fmtHours, fmtReviews, matchFilters, primaryCopy, totalPlaytime } from '$lib/utils.js';
  import type { Game, GameList, SteamMatchAction } from '$lib/types.js';

  let {
    game,
    lists,
    onBack,
    onPatch,
    onSteamMatch,
    onAddTag,
    onLaunch,
    onToggleList,
    onCreateList,
  }: {
    game: Game;
    lists: GameList[];
    onBack: () => void;
    onPatch: (id: number, delta: Partial<Game>) => void;
    onSteamMatch: (id: number, action: SteamMatchAction) => Promise<void>;
    onAddTag: (g: Game) => void;
    onLaunch: (g: Game) => void;
    onToggleList: (listId: string, gameId: number) => void;
    onCreateList: () => void;
  } = $props();

  const primary = $derived(primaryCopy(game));

  // ── Lists: manual ones are editable, smart ones only report a match ──────
  const manualLists  = $derived(lists.filter(l => l.kind === 'manual'));
  const smartMatches = $derived(lists.filter(l => l.kind === 'smart' && matchFilters(game, l.rules ?? {})));

  // ── Steam match (games with no Steam copy take Steam scores from a match) ──
  const hasSteamCopy = $derived(game.copies.some(c => c.platform === 'steam'));
  let matchEditing = $state(false);
  let matchInput   = $state('');
  let matchError   = $state('');
  let matchBusy    = $state(false);

  // Reset the editor when another game is shown — not when the same game is
  // replaced by an update (background score events would close it mid-typing).
  let matchForId: number | null = null;
  $effect(() => {
    const id = game.id;
    if (matchForId !== null && id !== matchForId) {
      matchEditing = false;
      matchInput = '';
      matchError = '';
    }
    matchForId = id;
  });

  async function runMatch(action: SteamMatchAction) {
    matchBusy = true;
    matchError = '';
    try {
      await onSteamMatch(game.id, action);
      matchEditing = false;
      matchInput = '';
    } catch (e) {
      matchError = String(e);
    } finally {
      matchBusy = false;
    }
  }

  function focusOnMount(node: HTMLElement) { node.focus(); }

  const LINK_DEFS = [
    { key: 'metacritic' as const, label: 'Metacritic',    icon: 'square-m', ph: 'https://metacritic.com/game/…'    },
    { key: 'hltb'       as const, label: 'HowLongToBeat', icon: 'timer',    ph: 'https://howlongtobeat.com/game/…' },
    { key: 'store'      as const, label: 'Store page',    icon: 'store',    ph: 'https://store…'                   },
  ];

  function patchLink(k: string, v: string) {
    onPatch(game.id, { links: { ...game.links, [k]: v } });
  }

  function patchCustom(next: Array<{ k: string; v: string }>) {
    onPatch(game.id, { custom: next });
  }

  function setField(i: number, key: 'k' | 'v', val: string) {
    patchCustom(game.custom.map((f, idx) => idx === i ? { ...f, [key]: val } : f));
  }

  function addField() {
    patchCustom([...(game.custom ?? []), { k: '', v: '' }]);
  }

  function removeField(i: number) {
    patchCustom(game.custom.filter((_, idx) => idx !== i));
  }
</script>

<div class="detail">
  <!-- Page header -->
  <div class="page-head">
    <button class="back-btn" onclick={onBack}>
      <Icon name="chevron-left" size={16} color="var(--fg-2)" />
      Library
    </button>
    <span class="breadcrumb">{game.title}</span>
    <div class="spacer"></div>
    <LaunchButton {game} {onLaunch} />
  </div>

  <!-- Scrollable body -->
  <div class="body">
    <div class="inner">

      <!-- Hero: sticky cover + right column -->
      <div class="hero">
        <div class="cover-col">
          <Cover {game} radius="var(--radius-lg)" />
          <div class="launch-lg">
            <LaunchButton {game} {onLaunch} size="lg" />
          </div>
          <div class="install-status">
            <Icon name={primary?.installed ? 'hard-drive' : 'cloud'} size={13} color="var(--fg-3)" />
            {#if primary?.installed}
              Installed on {PLAT[primary.platform].name}
            {:else if primary}
              Not installed · {PLAT[primary.platform].name}
            {:else}
              No platform copies
            {/if}
          </div>
        </div>

        <div class="right-col">
          <!-- Platforms + dev + status -->
          <div class="meta-row">
            <div class="platform-dots">
              {#each game.copies as copy}
                <span class="plat-dot" style="background:{PLAT[copy.platform].color}"></span>
              {/each}
            </div>
            {#if game.dev}
              <span class="meta-text">{game.dev}</span>
            {/if}
            {#if game.status}
              <div class="spacer"></div>
              <StatusPill status={game.status} />
            {/if}
          </div>

          <h1 class="game-title">{game.title}</h1>

          <div class="genre-row">
            {#each game.genre as g}
              <span class="genre-chip">{g}</span>
            {/each}
          </div>

          <!-- Score tiles -->
          <div class="score-tiles">
            {#each SCORE_SOURCES as src}
              <div class="score-tile">
                <ScoreBadge value={game[src.key]} size="lg" unit={src.unit} />
                <span class="score-label">{src.label}</span>
                {#if src.key === 'userScore'}
                  {@const caption = fmtReviews(game.userScoreReviews, game.userScore)}
                  {#if caption}<span class="score-sub">{caption}</span>{/if}
                {/if}
              </div>
            {/each}
            <div class="score-tile">
              <span class="hours-val">{fmtHours(totalPlaytime(game)).replace(' h', '')}</span>
              <span class="score-label">Hours</span>
            </div>
          </div>
          {#if !hasSteamCopy}
            <div class="steam-match">
              {#if matchEditing}
                <form class="match-form" onsubmit={(e) => { e.preventDefault(); runMatch({ kind: 'set', input: matchInput }); }}>
                  <input
                    class="match-input"
                    type="text"
                    placeholder="Steam store link or app ID"
                    bind:value={matchInput}
                    use:focusOnMount
                    onkeydown={(e) => { if (e.key === 'Escape') { matchEditing = false; matchError = ''; } }}
                  />
                  <button class="match-btn" type="submit" disabled={matchBusy || !matchInput.trim()}>
                    {matchBusy ? 'Checking…' : 'Use'}
                  </button>
                  <button class="match-link muted" type="button" onclick={() => { matchEditing = false; matchError = ''; }}>
                    Cancel
                  </button>
                </form>
              {:else}
                <span class="match-text">
                  {#if game.steamMatch}
                    Steam scores from <span class="match-name">{game.steamMatch.name}</span>
                    · {game.steamMatchManual ? 'set by you' : 'matched by title'}
                  {:else if game.steamMatchManual}
                    No Steam match
                  {:else if game.steamMatchCheckedAt}
                    Not found on Steam
                  {:else}
                    Not matched on Steam yet
                  {/if}
                </span>
                <span class="match-actions">
                  {#if game.steamMatch}
                    <button class="match-link" disabled={matchBusy} onclick={() => runMatch({ kind: 'reject' })}>Not this game</button>
                  {/if}
                  <button class="match-link" disabled={matchBusy} onclick={() => { matchEditing = true; matchError = ''; }}>
                    {game.steamMatch ? 'Change' : 'Add Steam link'}
                  </button>
                  {#if game.steamMatchManual}
                    <button class="match-link" disabled={matchBusy} onclick={() => runMatch({ kind: 'auto' })}>Match automatically</button>
                  {/if}
                </span>
              {/if}
              {#if matchError}
                <div class="match-error">{matchError}</div>
              {/if}
            </div>
          {/if}

          <!-- Owned on section -->
          {#if game.copies.length > 0}
            <div class="owned-block">
              <div class="origin-label">
                <Icon name="layers" size={11} color="var(--fg-3)" />
                Owned on
              </div>
              <div class="copies-list">
                {#each game.copies as copy}
                  <div class="copy-row">
                    <PlatIcon platform={copy.platform} size={18} />
                    <div class="copy-info">
                      <div class="copy-platform">{PLAT[copy.platform].name}</div>
                      <div class="copy-meta">
                        {#if copy.playtime != null}
                          <span>{fmtHours(copy.playtime)}</span>
                        {/if}
                        {#if copy.last}
                          <span>· {copy.last}</span>
                        {/if}
                        {#if copy.installed}
                          <span>· Installed</span>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Store metadata -->
          <div class="store-block">
            <div class="origin-label">
              <Icon name="lock" size={11} color="var(--fg-3)" />
              From store · read-only
            </div>
            <div class="meta-list">
              <div class="meta-item">
                <span class="meta-key">Genre</span>
                <span class="meta-val">{game.genre.join(', ')}</span>
              </div>
              <div class="meta-item">
                <span class="meta-key">Released</span>
                <span class="meta-val mono">{game.year}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- User data cards -->
      <div class="cards">

        <!-- Status & rating -->
        <section class="card">
          <div class="card-head">
            <span class="card-title">Status &amp; rating</span>
          </div>
          <div class="field-label">Status</div>
          <div class="status-select">
            {#each Object.entries(STATUS_META) as [id, m]}
              {@const active = game.status === id}
              <button
                class="status-btn"
                class:active
                style="
                  border-color: {active ? 'transparent' : 'var(--border-2)'};
                  background: {active ? m.bg : 'transparent'};
                  color: {active ? m.color : 'var(--fg-2)'};
                "
                onclick={() => onPatch(game.id, { status: (active ? game.status : id) as Game['status'] })}
              >
                <Icon name={m.icon} size={13} color={active ? m.color : 'var(--fg-2)'} />
                {m.label}
              </button>
            {/each}
          </div>
          <div class="field-label" style="margin-top:16px">Your rating</div>
          <Stars value={game.rating} size={24} onSet={(r) => onPatch(game.id, { rating: r })} />
        </section>

        <!-- Tags + notes -->
        <section class="card">
          <div class="card-head">
            <span class="card-title">Tags</span>
          </div>
          <div class="tag-row">
            {#each game.tags as t}
              <Tag label={t} onRemove={() => onPatch(game.id, { tags: game.tags.filter(x => x !== t) })} />
            {/each}
            <span
              class="add-tag"
              role="button"
              tabindex="0"
              onclick={() => onAddTag(game)}
              onkeydown={(e) => e.key === 'Enter' && onAddTag(game)}
            >+ tag</span>
          </div>
          <div class="field-label" style="margin-top:16px; margin-bottom:7px">Notes</div>
          <textarea
            class="notes"
            placeholder="Add a private note…"
            value={game.note}
            oninput={(e) => onPatch(game.id, { note: (e.target as HTMLTextAreaElement).value })}
          ></textarea>
        </section>

        <!-- Lists -->
        <section class="card">
          <div class="card-head">
            <span class="card-title">Lists</span>
          </div>
          {#if manualLists.length === 0}
            <p class="empty-hint">
              No manual lists yet.
              <button class="match-link" onclick={onCreateList}>New list…</button>
            </p>
          {/if}
          <div class="list-rows">
            {#each manualLists as l (l.id)}
              <label class="list-row">
                <!-- The box follows the saved state: the click is cancelled and
                     the row re-renders once Rust returns the updated list. -->
                <input
                  type="checkbox"
                  checked={(l.gameIds ?? []).includes(game.id)}
                  onclick={(e) => { e.preventDefault(); onToggleList(l.id, game.id); }}
                />
                <span class="list-dot" style="background:{LISTDOT[l.color]}"></span>
                <span class="list-name">{l.name}</span>
              </label>
            {/each}
            {#each smartMatches as l (l.id)}
              <div class="list-row readonly" title="Smart lists fill themselves from their filters">
                <Icon name="zap" size={13} color="var(--fg-3)" />
                <span class="list-dot" style="background:{LISTDOT[l.color]}"></span>
                <span class="list-name">{l.name}</span>
                <span class="list-note">matches filters</span>
              </div>
            {/each}
          </div>
        </section>

        <!-- Links -->
        <section class="card">
          <div class="card-head">
            <span class="card-title">Links</span>
          </div>
          <div class="origin-label user">
            <Icon name="pencil" size={11} color="var(--gold-400)" />
            Your data · saved locally
          </div>
          <div class="link-list">
            {#each LINK_DEFS as def}
              {@const val = game.links?.[def.key] ?? ''}
              {@const has = val.trim().length > 0}
              <div class="link-row">
                <div class="link-icon">
                  <Icon name={def.icon} size={15} color="var(--fg-2)" />
                </div>
                <div class="link-body">
                  <div class="link-name">{def.label}</div>
                  <input
                    class="link-input"
                    type="text"
                    placeholder={def.ph}
                    value={val}
                    oninput={(e) => patchLink(def.key, (e.target as HTMLInputElement).value)}
                  />
                </div>
                <a
                  class="link-open"
                  class:enabled={has}
                  href={has ? val : undefined}
                  target="_blank"
                  rel="noreferrer"
                  onclick={(e) => { if (!has) e.preventDefault(); }}
                  title={has ? 'Open link' : 'No link set'}
                >
                  <Icon name="external-link" size={14} color={has ? 'var(--accent)' : 'var(--fg-3)'} />
                </a>
              </div>
            {/each}
          </div>
        </section>

        <!-- Custom fields -->
        <section class="card">
          <div class="card-head">
            <span class="card-title">Custom fields</span>
            <button class="add-field-btn" onclick={addField}>
              <Icon name="plus" size={14} color="var(--accent)" />
              Add field
            </button>
          </div>
          <div class="origin-label user">
            <Icon name="pencil" size={11} color="var(--gold-400)" />
            Your data · saved locally
          </div>
          <div class="custom-list">
            {#if game.custom.length === 0}
              <p class="empty-hint">
                No custom fields yet. Add your own — "100% time", "save location", anything you track.
              </p>
            {/if}
            {#each game.custom as f, i}
              <div class="custom-row">
                <input
                  class="custom-key"
                  type="text"
                  placeholder="Field name"
                  value={f.k}
                  oninput={(e) => setField(i, 'k', (e.target as HTMLInputElement).value)}
                />
                <input
                  class="custom-val"
                  type="text"
                  placeholder="Value"
                  value={f.v}
                  oninput={(e) => setField(i, 'v', (e.target as HTMLInputElement).value)}
                />
                <button class="custom-remove" title="Remove field" onclick={() => removeField(i)}>
                  <Icon name="trash-2" size={14} />
                </button>
              </div>
            {/each}
          </div>
        </section>

      </div>
    </div>
  </div>
</div>

<style>
  .detail {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-1);
  }

  /* ── Page header ── */
  .page-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 18px;
    height: 52px;
    border-bottom: 1px solid var(--border-1);
    flex: none;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 12px 0 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-1);
    background: var(--bg-3);
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
  }

  .breadcrumb {
    font-size: 13px;
    color: var(--fg-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spacer { flex: 1; }

  /* ── Scrollable body ── */
  .body {
    flex: 1;
    overflow-y: auto;
  }

  .inner {
    max-width: 920px;
    margin: 0 auto;
    padding: 30px 28px 64px;
  }

  /* ── Hero ── */
  .hero {
    display: grid;
    grid-template-columns: 232px 1fr;
    gap: 30px;
    align-items: start;
  }

  .cover-col {
    position: sticky;
    top: 0;
  }

  .launch-lg {
    margin-top: 14px;
    display: flex;
  }

  .launch-lg :global(button) {
    width: 100%;
    justify-content: center;
  }

  .install-status {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-3);
  }

  .right-col { min-width: 0; }

  .meta-row {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-bottom: 8px;
  }

  .plat-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
  }

  .meta-text {
    font-size: 12.5px;
    color: var(--fg-2);
    font-weight: 500;
  }

  .game-title {
    margin: 0;
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.08;
    color: var(--fg-1);
  }

  .genre-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 10px;
  }

  .genre-chip {
    font-size: 12px;
    color: var(--fg-2);
    background: var(--bg-3);
    padding: 3px 10px;
    border-radius: var(--radius-full);
  }

  /* ── Score tiles ── */
  .score-tiles {
    display: flex;
    gap: 10px;
    margin-top: 18px;
  }

  .score-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    flex: 1;
    padding: 12px 8px;
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-md);
  }

  .hours-val {
    font-family: var(--font-mono);
    font-size: 18px;
    font-weight: 700;
    color: var(--fg-1);
  }

  .score-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-2);
  }

  /* ── Steam match ── */
  .steam-match {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 12px;
    margin-top: 8px;
    font-size: 11px;
    color: var(--fg-3);
  }

  .match-name { color: var(--fg-2); }

  .match-actions {
    display: inline-flex;
    gap: 12px;
  }

  .match-link {
    border: none;
    background: transparent;
    padding: 0;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    cursor: pointer;
  }
  .match-link:hover:not(:disabled) { text-decoration: underline; }
  .match-link:disabled { color: var(--fg-3); cursor: default; }
  .match-link.muted { color: var(--fg-2); }

  .match-form {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .match-input {
    flex: 1;
    min-width: 0;
    padding: 5px 8px;
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    background: var(--bg-2);
    color: var(--fg-1);
    font-family: var(--font-mono);
    font-size: 11.5px;
    outline: none;
  }
  .match-input:focus { border-color: var(--accent); }
  .match-input::placeholder { color: var(--fg-3); }

  .match-btn {
    padding: 5px 12px;
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    background: var(--bg-3);
    color: var(--fg-1);
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .match-btn:disabled { color: var(--fg-3); cursor: default; }

  .match-error {
    width: 100%;
    color: var(--danger);
  }

  .score-sub {
    margin-top: -3px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-3);
    font-feature-settings: 'tnum' 1;
  }

  /* ── Owned on section ── */
  .owned-block {
    margin-top: 24px;
  }

  .platform-dots {
    display: flex;
    gap: 6px;
    flex: none;
  }

  .copies-list {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .copy-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border-1);
    border-radius: var(--radius-md);
    background: var(--bg-2);
  }

  .copy-info {
    flex: 1;
    min-width: 0;
  }

  .copy-platform {
    font-size: 13px;
    font-weight: 600;
    color: var(--fg-1);
  }

  .copy-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-3);
    margin-top: 2px;
  }

  /* ── Store metadata ── */
  .store-block { margin-top: 24px; }

  .origin-label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .origin-label.user { color: var(--gold-400); }

  .meta-list { margin-top: 6px; }

  .meta-item {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-1);
  }

  .meta-key {
    font-size: 12px;
    color: var(--fg-3);
  }

  .meta-val {
    font-size: 13px;
    color: var(--fg-1);
    text-align: right;
  }

  .meta-val.mono { font-family: var(--font-mono); }

  /* ── Cards ── */
  .cards {
    margin-top: 28px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px;
  }

  .card {
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-lg);
    padding: 18px;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .card-title {
    font-size: 13px;
    font-weight: 600;
  }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-3);
    margin-bottom: 8px;
  }

  /* ── Status select ── */
  .status-select {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }

  .status-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border-radius: var(--radius-full);
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    border: 1px solid;
    transition: all 0.1s;
  }

  .status-btn.active { font-weight: 600; }

  /* ── Tags ── */
  .tag-row {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }

  .add-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 3px 9px;
    border-radius: var(--radius-full);
    border: 1px dashed var(--border-2);
    color: var(--fg-3);
    cursor: pointer;
  }

  .notes {
    width: 100%;
    min-height: 64px;
    box-sizing: border-box;
    resize: vertical;
    font-family: var(--font-ui);
    font-size: 13px;
    line-height: 1.5;
    color: var(--fg-1);
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 9px 11px;
    outline: none;
  }

  /* ── Lists ── */
  .list-rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .list-row {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 5px 6px;
    margin: 0 -6px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--fg-1);
    cursor: pointer;
  }

  .list-row:hover { background: var(--bg-3); }

  .list-row.readonly {
    color: var(--fg-2);
    cursor: default;
  }

  .list-row.readonly:hover { background: transparent; }

  .list-row input {
    width: 13px;
    height: 13px;
    margin: 0;
    flex: none;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .list-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }

  .list-name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .list-note {
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--fg-3);
    flex: none;
  }

  /* ── Links ── */
  .link-list {
    display: flex;
    flex-direction: column;
    gap: 13px;
    margin-top: 12px;
  }

  .link-row {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .link-icon {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: var(--radius-sm);
    background: var(--bg-3);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-2);
  }

  .link-body {
    flex: 1;
    min-width: 0;
  }

  .link-name {
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-3);
    margin-bottom: 2px;
  }

  .link-input {
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    color: var(--fg-1);
    font-family: var(--font-mono);
    font-size: 11.5px;
    padding: 0;
    text-overflow: ellipsis;
  }

  .link-input::placeholder { color: var(--fg-3); }

  .link-open {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    text-decoration: none;
    border: 1px solid var(--border-1);
    background: transparent;
    cursor: default;
    opacity: 0.5;
  }

  .link-open.enabled {
    border-color: var(--border-2);
    background: var(--bg-3);
    cursor: pointer;
    opacity: 1;
  }

  /* ── Custom fields ── */
  .add-field-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: none;
    background: transparent;
    color: var(--accent);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }

  .custom-list {
    display: flex;
    flex-direction: column;
    gap: 9px;
    margin-top: 12px;
  }

  .empty-hint {
    font-size: 12.5px;
    color: var(--fg-3);
    padding: 6px 0;
    line-height: 1.5;
    margin: 0;
  }

  .custom-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .custom-key {
    flex: 0 0 38%;
    min-width: 0;
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    color: var(--fg-2);
    font-family: var(--font-ui);
    font-size: 12.5px;
    outline: none;
  }

  .custom-val {
    flex: 1;
    min-width: 0;
    background: var(--bg-3);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    color: var(--fg-1);
    font-family: var(--font-mono);
    font-size: 12.5px;
    outline: none;
  }

  .custom-remove {
    width: 30px;
    height: 30px;
    flex: none;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--fg-3);
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .custom-remove:hover {
    background: var(--bg-3);
    color: var(--danger);
  }
</style>
