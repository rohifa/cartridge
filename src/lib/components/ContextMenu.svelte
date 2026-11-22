<script lang="ts" module>
  export type MenuItem =
    | {
        kind?: 'item';
        label: string;
        dot?: string;        // colour dot before the label (lists)
        checked?: boolean;   // shows a check; makes the item a checkbox for a11y
        disabled?: boolean;
        danger?: boolean;
        onSelect: () => void;
      }
    | { kind: 'submenu'; label: string; disabled?: boolean; items: MenuItem[] }
    | { kind: 'separator' };

  /// Where to open a menu for a `contextmenu` event. Keyboard-triggered events
  /// (context-menu key, Shift+F10) have no pointer position, so fall back to
  /// the element's top-left corner.
  export function menuPoint(e: MouseEvent): { x: number; y: number } {
    if (e.clientX === 0 && e.clientY === 0 && e.currentTarget instanceof Element) {
      const r = e.currentTarget.getBoundingClientRect();
      return { x: r.left + 12, y: r.top + 12 };
    }
    return { x: e.clientX, y: e.clientY };
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    x,
    y,
    items,
    label = 'Actions',
    onClose,
  }: {
    x: number;
    y: number;
    items: MenuItem[];
    label?: string;
    onClose: () => void;
  } = $props();

  const EDGE = 6; // keep this far from the window edges

  let rootEl = $state<HTMLDivElement | null>(null);
  let left = $state(0);
  let top  = $state(0);
  let openSub = $state<number | null>(null);

  // Place at the pointer, then pull back inside the window.
  $effect(() => {
    if (!rootEl) return;
    const r = rootEl.getBoundingClientRect();
    left = Math.max(EDGE, Math.min(x, window.innerWidth  - r.width  - EDGE));
    top  = Math.max(EDGE, Math.min(y, window.innerHeight - r.height - EDGE));
  });

  // Focus the first item; on close, give focus back if nothing else took it.
  $effect(() => {
    if (!rootEl) return;
    const el = rootEl;
    const prev = document.activeElement as HTMLElement | null;
    focusIn(el, 0);
    return () => {
      const active = document.activeElement;
      const free = !active || active === document.body || el.contains(active);
      if (free && prev?.isConnected) prev.focus({ preventScroll: true });
    };
  });

  // Close on scroll anywhere (capture: scroll doesn't bubble), resize or blur.
  $effect(() => {
    const close = () => onClose();
    window.addEventListener('scroll', close, true);
    window.addEventListener('resize', close);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('scroll', close, true);
      window.removeEventListener('resize', close);
      window.removeEventListener('blur', close);
    };
  });

  function onWindowPointerDown(e: PointerEvent) {
    if (rootEl && !rootEl.contains(e.target as Node)) onClose();
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); onClose(); }
  }

  function select(it: MenuItem) {
    if (it.kind === 'separator' || it.kind === 'submenu' || it.disabled) return;
    onClose();
    it.onSelect();
  }

  // ── Keyboard navigation ────────────────────────────────────────────────────
  function menuItems(menu: Element): HTMLElement[] {
    return [...menu.querySelectorAll<HTMLElement>(':scope > [role^="menuitem"]:not([aria-disabled="true"])')];
  }

  function focusIn(menu: Element, index: number) {
    const els = menuItems(menu);
    if (els.length === 0) return;
    els[(index + els.length) % els.length].focus({ preventScroll: true });
  }

  async function onMenuKeydown(e: KeyboardEvent) {
    const current = document.activeElement as HTMLElement | null;
    const menu = current?.closest('[role="menu"]');
    if (!current || !menu) return;
    const els = menuItems(menu);
    const i = els.indexOf(current);

    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      focusIn(menu, i < 0 ? 0 : i + (e.key === 'ArrowDown' ? 1 : -1));
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      focusIn(menu, e.key === 'Home' ? 0 : -1);
    } else if (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const sub = current.dataset.sub;
      if (sub != null) {
        openSub = Number(sub);
        await tick();
        const subMenu = current.querySelector('[role="menu"]');
        if (subMenu) focusIn(subMenu, 0);
      } else if (e.key !== 'ArrowRight') {
        current.click();
      }
    } else if (e.key === 'ArrowLeft') {
      const parent = menu.parentElement?.closest<HTMLElement>('[role="menuitem"]');
      if (parent) {
        e.preventDefault();
        openSub = null;
        parent.focus({ preventScroll: true });
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      onClose();
    }
  }

  /// Opens submenus beside their item, flipping left / shifting up at the edges.
  function placeSub(node: HTMLElement) {
    const r = node.getBoundingClientRect();
    if (r.right > window.innerWidth - EDGE) {
      node.style.left = 'auto';
      node.style.right = 'calc(100% + 2px)';
    }
    const overflow = r.bottom - (window.innerHeight - EDGE);
    if (overflow > 0) node.style.top = `${-5 - overflow}px`;
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} onkeydown={onWindowKeydown} />

{#snippet entries(list: MenuItem[], nested: boolean)}
  {#each list as it, i}
    {#if it.kind === 'separator'}
      <div class="sep" role="separator"></div>
    {:else if it.kind === 'submenu'}
      <!-- svelte-ignore a11y_click_events_have_key_events (keys are handled by onMenuKeydown on the root) -->
      <div
        class="item"
        class:open={openSub === i}
        role="menuitem"
        tabindex="-1"
        aria-haspopup="menu"
        aria-expanded={openSub === i}
        aria-disabled={it.disabled || undefined}
        data-sub={it.disabled ? undefined : i}
        onpointerenter={() => { if (!it.disabled) openSub = i; }}
        onclick={() => { if (!it.disabled) openSub = i; }}
      >
        <span class="label">{it.label}</span>
        <Icon name="chevron-right" size={13} color="var(--fg-3)" />
        {#if openSub === i}
          <div class="menu sub" role="menu" aria-label={it.label} use:placeSub>
            {@render entries(it.items, true)}
          </div>
        {/if}
      </div>
    {:else}
      <div
        class="item"
        class:danger={it.danger}
        role={it.checked != null ? 'menuitemcheckbox' : 'menuitem'}
        aria-checked={it.checked != null ? it.checked : undefined}
        tabindex="-1"
        aria-disabled={it.disabled || undefined}
        onpointerenter={() => { if (!nested) openSub = null; }}
        onclick={() => select(it)}
      >
        {#if it.dot}<span class="dot" style="background:{it.dot}"></span>{/if}
        <span class="label">{it.label}</span>
        {#if it.checked}
          <Icon name="check" size={14} color="var(--accent)" />
        {/if}
      </div>
    {/if}
  {/each}
{/snippet}

<div
  bind:this={rootEl}
  class="menu root"
  role="menu"
  aria-label={label}
  tabindex="-1"
  style="left:{left}px; top:{top}px"
  oncontextmenu={(e) => e.preventDefault()}
  onkeydown={onMenuKeydown}
>
  {@render entries(items, false)}
</div>

<style>
  /* Tokens follow the sort dropdown in Toolbar.svelte. */
  .menu {
    z-index: 70;
    min-width: 180px;
    max-width: 280px;
    background: var(--bg-2);
    border: 1px solid var(--border-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    padding: 5px;
    outline: none;
  }

  .root {
    position: fixed;
  }

  .sub {
    position: absolute;
    top: -5px;
    left: calc(100% + 2px);
  }

  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 13px;
    color: var(--fg-2);
    background: transparent;
    user-select: none;
    outline: none;
  }

  .item:hover,
  .item:focus,
  .item.open {
    background: var(--bg-3);
    color: var(--fg-1);
  }

  .item[aria-disabled='true'] {
    color: var(--fg-3);
    cursor: default;
  }

  .item[aria-disabled='true']:hover,
  .item[aria-disabled='true']:focus {
    background: transparent;
  }

  .item.danger {
    color: var(--danger);
  }

  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }

  .sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border-1);
  }
</style>
