import { convertFileSrc } from '@tauri-apps/api/core';
import type { Game, Filters } from './types.js';

export function fmtHours(h: number | null): string {
  if (h == null) return '— h';
  return h.toFixed(1).replace(/\.0$/, '') + ' h';
}

export function scoreTone(v: number | null): { fg: string; bg: string; bd: string } {
  if (v == null) return { fg: 'var(--fg-3)', bg: 'var(--bg-3)', bd: 'var(--border-1)' };
  if (v >= 75)   return { fg: 'oklch(0.96 0.02 150)', bg: 'oklch(0.55 0.15 150)', bd: 'transparent' };
  if (v >= 50)   return { fg: 'oklch(0.22 0.05 80)',  bg: 'oklch(0.80 0.15 85)',  bd: 'transparent' };
  return               { fg: 'oklch(0.97 0.02 25)',  bg: 'oklch(0.58 0.20 25)',  bd: 'transparent' };
}

export function coverSrc(game: Game): string | null {
  if (!game.cover) return null;
  if (game.cover.startsWith('/') || game.cover.startsWith('~')) {
    return convertFileSrc(game.cover);
  }
  return `/covers/${game.cover}.png`;
}

export function matchFilters(g: Game, f: Partial<Filters> | null | undefined): boolean {
  if (!f) return true;
  if (f.genres?.length    && !f.genres.some(x => g.genre.includes(x)))              return false;
  if (f.platforms?.length && !f.platforms.includes(g.platform))                     return false;
  if (f.statuses?.length  && !f.statuses.includes(g.status))                        return false;
  if (f.yearLo != null    && g.year < f.yearLo)                                     return false;
  if (f.yearHi != null    && g.year > f.yearHi)                                     return false;
  if (f.mcMin  && (g.metacritic == null  || g.metacritic  < f.mcMin))               return false;
  if (f.ocMin  && (g.opencritic == null  || g.opencritic  < f.ocMin))               return false;
  if (f.usrMin && (g.userScore  == null  || g.userScore   < f.usrMin))              return false;
  if (f.playtimeMax != null && !(g.playtime != null && g.playtime <= f.playtimeMax)) return false;
  return true;
}
