<script lang="ts">
  import Icon from './Icon.svelte';
  import PlatIcon from './PlatIcon.svelte';
  import ScoreBadge from './ScoreBadge.svelte';
  import type { CartridgeData, Filters } from '$lib/types.js';

  let {
    data,
    filters,
    yearBounds,
    activeCount,
    onSetFilters,
    onClear,
    onSaveSmart,
  }: {
    data: CartridgeData;
    filters: Filters;
    yearBounds: [number, number];
    activeCount: number;
    onSetFilters: (updater: (f: Filters) => Filters) => void;
    onClear: () => void;
    onSaveSmart: () => void;
  } = $props();

  function toggleArr<T>(key: keyof Filters, val: T) {
    onSetFilters((f) => {
      const cur = (f[key] as T[]) ?? [];
      return {
        ...f,
        [key]: cur.includes(val) ? cur.filter((x) => x !== val) : [...cur, val],
      };
    });
  }

  const [minYear, maxYear] = $derived(yearBounds);

  function pct(v: number) {
    return ((v - minYear) / (maxYear - minYear)) * 100;
  }
</script>

<div class="panel">
  <!-- Header -->
  <div class="panel-head">
    <span class="panel-title">Filters</span>
    <button
      class="clear-btn"
      disabled={activeCount === 0}
      onclick={onClear}
    >Clear all</button>
  </div>

  <!-- Scrollable body -->
  <div class="panel-body">

    <!-- Category -->
    <div class="field">
      <div class="field-head">Category</div>
      <div class="chip-row">
        {#each data.genres as genre}
          <button
            class="chip"
            class:active={filters.genres.includes(genre)}
            onclick={() => toggleArr('genres', genre)}
          >{genre}</button>
        {/each}
      </div>
    </div>

    <!-- Platform -->
    <div class="field">
      <div class="field-head">Platform</div>
      <div class="chip-row">
        {#each data.accounts as a}
          <button
            class="chip"
            class:active={filters.platforms.includes(a.platform)}
            onclick={() => toggleArr('platforms', a.platform)}
          >
            <span class="chip-icon">
              <PlatIcon platform={a.platform} size={13} />
            </span>
            {a.name}
          </button>
        {/each}
      </div>
    </div>

    <!-- Status -->
    <div class="field">
      <div class="field-head">Status</div>
      <div class="chip-row">
        {#each data.statuses as s}
          <button
            class="chip"
            class:active={filters.statuses.includes(s.id)}
            onclick={() => toggleArr('statuses', s.id)}
          >{s.label}</button>
        {/each}
      </div>
    </div>

    <!-- Release year -->
    <div class="field">
      <div class="field-head">Release year</div>
      <div class="year-range">
        <div class="track-wrap">
          <div class="track-bg"></div>
          <div
            class="track-fill"
            style="left:{pct(filters.yearLo)}%; right:{100 - pct(filters.yearHi)}%"
          ></div>
          <input
            type="range"
            class="range-dual"
            min={minYear}
            max={maxYear}
            value={filters.yearLo}
            oninput={(e) => {
              const lo = Math.min(+(e.target as HTMLInputElement).value, filters.yearHi);
              onSetFilters((f) => ({ ...f, yearLo: lo }));
            }}
          />
          <input
            type="range"
            class="range-dual"
            min={minYear}
            max={maxYear}
            value={filters.yearHi}
            oninput={(e) => {
              const hi = Math.max(+(e.target as HTMLInputElement).value, filters.yearLo);
              onSetFilters((f) => ({ ...f, yearHi: hi }));
            }}
          />
        </div>
        <div class="year-labels">
          <span class="mono">{filters.yearLo}</span>
          <span class="mono">{filters.yearHi}</span>
        </div>
      </div>
    </div>

    <!-- Score sliders -->
    <div class="field">
      <div class="score-sliders">
        {#each [
          { key: 'mcMin' as const,  label: 'Metacritic', val: filters.mcMin, unit: ''  },
          { key: 'usrMin' as const, label: 'Steam reviews', val: filters.usrMin, unit: '%' },
        ] as row}
          <div class="score-row">
            <div class="score-row-head">
              <ScoreBadge value={row.val === 0 ? null : row.val} size="sm" unit={row.unit} />
              <span class="score-label">{row.label}</span>
            </div>
            <div class="min-slider">
              <input
                type="range"
                min="0"
                max="100"
                step="5"
                value={row.val}
                oninput={(e) => {
                  const v = +(e.target as HTMLInputElement).value;
                  onSetFilters((f) => ({ ...f, [row.key]: v }));
                }}
              />
              <span class="mono slider-val">{row.val === 0 ? 'any' : row.val + row.unit + '+'}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>

  </div>

  <!-- Footer -->
  <div class="panel-foot">
    <button
      class="smart-btn"
      class:enabled={activeCount > 0}
      disabled={activeCount === 0}
      onclick={onSaveSmart}
    >
      <Icon name="zap" size={14} color={activeCount > 0 ? 'var(--accent)' : 'var(--fg-3)'} />
      Save as smart list
    </button>
  </div>
</div>

<style>
  .panel {
    width: 266px;
    flex: none;
    background: var(--bg-1);
    border-right: 1px solid var(--border-1);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 12px;
    border-bottom: 1px solid var(--border-1);
    flex: none;
  }

  .panel-title {
    font-size: 13px;
    font-weight: 600;
  }

  .clear-btn {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--accent);
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    padding: 0;
  }

  .clear-btn:disabled {
    color: var(--fg-3);
    cursor: default;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 16px 8px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .field-head {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--fg-3);
    margin-bottom: 9px;
  }

  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: var(--radius-full);
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 400;
    border: 1px solid var(--border-2);
    background: transparent;
    color: var(--fg-2);
    transition: background 0.1s, border-color 0.1s, color 0.1s;
  }

  .chip.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .chip-icon {
    width: 14px;
    height: 14px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  /* Year range dual slider */
  .year-range {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .track-wrap {
    position: relative;
    height: 24px;
    display: flex;
    align-items: center;
  }

  .track-bg {
    position: absolute;
    left: 0;
    right: 0;
    height: 4px;
    border-radius: 2px;
    background: var(--bg-3);
  }

  .track-fill {
    position: absolute;
    height: 4px;
    border-radius: 2px;
    background: var(--accent);
  }

  .year-labels {
    display: flex;
    justify-content: space-between;
    margin-top: 2px;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-1);
  }

  /* Score sliders */
  .score-sliders {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .score-row-head {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 9px;
  }

  .score-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-2);
  }

  .min-slider {
    display: flex;
    align-items: center;
    gap: 11px;
  }

  .min-slider input[type="range"] {
    flex: 1;
    height: 4px;
    accent-color: var(--accent);
  }

  .slider-val {
    min-width: 46px;
    text-align: right;
  }

  .panel-foot {
    flex: none;
    padding: 12px 16px;
    border-top: 1px solid var(--border-1);
  }

  .smart-btn {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 34px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-1);
    background: transparent;
    color: var(--fg-3);
    font-family: var(--font-ui);
    font-size: 12.5px;
    font-weight: 600;
    cursor: default;
  }

  .smart-btn.enabled {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    cursor: pointer;
  }
</style>
