import { invoke } from '@tauri-apps/api/core';
import type { Account, CartridgeData, Game, GameList, Prefs } from './types.js';
import type { GamePatch, SyncResult } from './types.js';

// ── Data ──────────────────────────────────────────────────────────────────────

export const loadData  = ()                    => invoke<CartridgeData>('load_data');
export const patchGame = (id: number, patch: GamePatch) => invoke<Game>('patch_game', { id, patch });
export const savePrefs = (prefs: Prefs)        => invoke<void>('save_prefs', { prefs });
export const exportJson = ()                   => invoke<string>('export_json');
/// Writes the library (without credentials) to `path`; returns the game count.
export const exportJsonTo = (path: string)     => invoke<number>('export_json_to', { path });

// ── Lists ─────────────────────────────────────────────────────────────────────

export const createList     = (list: GameList)                    => invoke<GameList>('create_list', { list });
export const deleteList     = (id: string)                        => invoke<void>('delete_list', { id });
/// Both return the updated list. Manual lists only; smart lists are rejected.
export const addToList      = (listId: string, gameId: number)    => invoke<GameList>('add_to_list', { listId, gameId });
export const removeFromList = (listId: string, gameId: number)    => invoke<GameList>('remove_from_list', { listId, gameId });

// ── Launch ────────────────────────────────────────────────────────────────────

export const launchGame = (gameId: number) => invoke<void>('launch_game', { gameId });
export const openUrl    = (url: string)    => invoke<void>('open_url', { url });

// ── Sync ──────────────────────────────────────────────────────────────────────

export const addSteamAccount  = (apiKey: string, steamId: string, prevId?: string) => invoke<Account>('add_steam_account', { apiKey, steamId, prevId: prevId ?? null });
export const startOAuthFlow   = (platform: string)               => invoke<Account>('start_oauth_flow', { platform });
export const syncAccount      = (accountId: string)              => invoke<SyncResult>('sync_account', { accountId });
export const removeAccount    = (accountId: string)              => invoke<void>('remove_account', { accountId });

// ── Scores ────────────────────────────────────────────────────────────────────

/// Starts the paced background score refresh (title matching, Steam reviews,
/// Metacritic); results arrive as `games-updated` events.
export const startScoreRefresh = () => invoke<void>('start_score_refresh');
export const setSteamMatch     = (gameId: number, input: string) => invoke<Game>('set_steam_match', { gameId, input });
export const rejectSteamMatch  = (gameId: number)                => invoke<Game>('reject_steam_match', { gameId });
export const autoSteamMatch    = (gameId: number)                => invoke<Game>('auto_steam_match', { gameId });

// ── Filesystem ────────────────────────────────────────────────────────────────

/// Path of library.json for display, with the home directory as `~`.
export const getLibraryPath = () => invoke<string>('get_library_path');
/// Opens Finder with library.json selected.
export const revealLibrary  = () => invoke<void>('reveal_library');
