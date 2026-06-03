import type { Platform, GameStatus, ListColor } from './types.js';

export const PLAT: Record<Platform, { name: string; color: string }> = {
  steam: { name: 'Steam', color: 'var(--plat-steam)' },
  gog:   { name: 'GOG',   color: 'var(--plat-gog)'   },
  epic:  { name: 'Epic',  color: 'var(--plat-epic)'  },
};

export const TAGCOLOR: Record<string, string> = {
  backlog:   'var(--spec-green)',
  finished:  'oklch(0.80 0.16 300)',
  favourite: 'var(--spec-amber)',
  replay:    'oklch(0.74 0.18 25)',
  'co-op':   'var(--spec-blue)',
  short:     'var(--spec-teal)',
};

export const TAGBG: Record<string, string> = {
  backlog:   'oklch(0.74 0.18 145/0.18)',
  finished:  'oklch(0.60 0.20 300/0.20)',
  favourite: 'oklch(0.78 0.13 75/0.18)',
  replay:    'oklch(0.62 0.21 25/0.18)',
  'co-op':   'oklch(0.70 0.13 240/0.18)',
  short:     'oklch(0.72 0.10 195/0.18)',
};

export const LISTDOT: Record<ListColor, string> = {
  green:  'var(--spec-green)',
  purple: 'oklch(0.78 0.16 300)',
  amber:  'var(--spec-amber)',
  teal:   'var(--spec-teal)',
  blue:   'var(--spec-blue)',
  red:    'var(--spec-red)',
};

export const STATUS_META: Record<GameStatus, { label: string; color: string; bg: string; icon: string }> = {
  backlog:  { label: 'Backlog',  color: 'var(--spec-green)',    bg: 'oklch(0.74 0.18 145/0.16)', icon: 'layers'      },
  playing:  { label: 'Playing',  color: 'var(--spec-blue)',     bg: 'oklch(0.70 0.13 240/0.18)', icon: 'play'        },
  beaten:   { label: 'Beaten',   color: 'oklch(0.80 0.16 300)', bg: 'oklch(0.60 0.20 300/0.18)', icon: 'check-check' },
  dropped:  { label: 'Dropped',  color: 'var(--fg-3)',          bg: 'var(--bg-3)',               icon: 'x'           },
  wishlist: { label: 'Wishlist', color: 'var(--spec-amber)',    bg: 'oklch(0.78 0.13 75/0.16)',  icon: 'bookmark'    },
};

export const SCORE_SOURCES = [
  { key: 'metacritic' as const, label: 'Metacritic', abbr: 'MC'  },
  { key: 'opencritic' as const, label: 'OpenCritic', abbr: 'OC'  },
  { key: 'userScore'  as const, label: 'User score', abbr: 'USR' },
];

export const ALLTAGS = ['backlog', 'finished', 'favourite', 'replay', 'co-op', 'short'];
