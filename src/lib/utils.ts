import { convertFileSrc } from '@tauri-apps/api/core';
import type { Game, GameCopy, Filters } from './types.js';
import { ALLTAGS } from './constants.js';

export function fmtHours(h: number | null): string {
  if (h == null) return '— h';
  return h.toFixed(1).replace(/\.0$/, '') + ' h';
}

/// Caption under the Steam score: review count, or why there is no percentage.
export function fmtReviews(count: number | null | undefined, score: number | null): string | null {
  if (count == null) return null;
  if (count === 0)   return 'No reviews';
  if (score == null) return 'Not enough reviews';
  return `${count.toLocaleString('en-US')} reviews`;
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

const DAY = 86400;
const nowSecs = () => Math.floor(Date.now() / 1000);

/// Unix secs of the copy's last session; null when never played or unknown.
export function lastPlayedTs(copy: GameCopy): number | null {
  return copy.lastPlayedAt ?? null;
}

/// Most recent session across all copies of a game.
export function gameLastPlayed(game: Game): number | null {
  let best: number | null = null;
  for (const c of game.copies) {
    const t = lastPlayedTs(c);
    if (t != null && (best == null || t > best)) best = t;
  }
  return best;
}

/// Relative label for the copy's last session, computed now.
export function fmtLastPlayed(copy: GameCopy): string | null {
  if (copy.lastPlayedAt == null) return null;
  const days = Math.max(0, Math.floor((nowSecs() - copy.lastPlayedAt) / DAY));
  if (days === 0) return 'today';
  if (days === 1) return 'yesterday';
  const ago = (n: number, unit: string) => `${n} ${unit}${n === 1 ? '' : 's'} ago`;
  if (days < 7)   return ago(days, 'day');
  if (days < 30)  return ago(Math.floor(days / 7), 'week');
  if (days < 365) return ago(Math.floor(days / 30), 'month');
  return ago(Math.floor(days / 365), 'year');
}

/// Get primary copy for display (installed > most recent > first)
export function primaryCopy(game: Game): GameCopy | null {
  if (game.copies.length === 0) return null;
  return game.copies.reduce((best, c) => {
    // Prefer installed
    if (c.installed && !best.installed) return c;
    if (!c.installed && best.installed) return best;
    // Then prefer most recently played
    if ((lastPlayedTs(c) ?? -Infinity) > (lastPlayedTs(best) ?? -Infinity)) return c;
    return best;
  });
}

/// Get playtime summary (sum across all platforms)
export function totalPlaytime(game: Game): number | null {
  const times = game.copies.map(c => c.playtime).filter(t => t != null) as number[];
  return times.length > 0 ? times.reduce((a, b) => a + b, 0) : null;
}

export function matchFilters(g: Game, f: Partial<Filters> | null | undefined): boolean {
  if (!f) return true;
  if (f.genres?.length    && !f.genres.some(x => g.genre.includes(x)))              return false;
  if (f.platforms?.length && !g.copies.some(c => f.platforms!.includes(c.platform))) return false;
  if (f.statuses?.length  && !f.statuses.includes(g.status))                        return false;
  if (f.yearLo != null    && g.year > 0 && g.year < f.yearLo)                       return false;
  if (f.yearHi != null    && g.year > 0 && g.year > f.yearHi)                       return false;
  if (f.mcMin  && (g.metacritic == null  || g.metacritic  < f.mcMin))               return false;
  if (f.usrMin && (g.userScore  == null  || g.userScore   < f.usrMin))              return false;
  const playtime = totalPlaytime(g);
  if (f.playtimeMax != null && !(playtime != null && playtime <= f.playtimeMax))    return false;
  return true;
}

export const TAG_MAX = 32;

/// A typed tag as it is stored: trimmed, lowercase, single spaces, at most 32 chars.
export function normTag(raw: string): string {
  return raw.trim().toLowerCase().replace(/\s+/g, ' ').slice(0, TAG_MAX).trim();
}

/// Every tag in the library plus the built-in ones, most used first, then A–Z.
export function allTags(games: Game[]): string[] {
  const counts = new Map<string, number>(ALLTAGS.map(t => [t, 0]));
  for (const g of games) for (const t of g.tags) counts.set(t, (counts.get(t) ?? 0) + 1);
  return [...counts]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([t]) => t);
}
