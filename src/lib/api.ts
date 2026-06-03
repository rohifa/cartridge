import { invoke } from '@tauri-apps/api/core';
import type { Account, CartridgeData, Game, GameList, Prefs } from './types.js';
import type { GamePatch, SyncResult } from './types.js';

// ── Data ──────────────────────────────────────────────────────────────────────

export const loadData  = ()                    => invoke<CartridgeData>('load_data');
export const patchGame = (id: number, patch: GamePatch) => invoke<Game>('patch_game', { id, patch });
export const savePrefs = (prefs: Prefs)        => invoke<void>('save_prefs', { prefs });
export const exportJson = ()                   => invoke<string>('export_json');

// ── Lists ─────────────────────────────────────────────────────────────────────

export const createList     = (list: GameList)                    => invoke<GameList>('create_list', { list });
export const deleteList     = (id: string)                        => invoke<void>('delete_list', { id });
export const addToList      = (listId: string, gameId: number)    => invoke<void>('add_to_list', { listId, gameId });
export const removeFromList = (listId: string, gameId: number)    => invoke<void>('remove_from_list', { listId, gameId });

// ── Launch ────────────────────────────────────────────────────────────────────

export const launchGame = (gameId: number) => invoke<void>('launch_game', { gameId });
export const openUrl    = (url: string)    => invoke<void>('open_url', { url });

// ── Sync ──────────────────────────────────────────────────────────────────────

export const addSteamAccount = (apiKey: string, steamId: string) => invoke<Account>('add_steam_account', { apiKey, steamId });
export const syncAccount     = (accountId: string)               => invoke<SyncResult>('sync_account', { accountId });
export const removeAccount   = (accountId: string)               => invoke<void>('remove_account', { accountId });

// ── Filesystem ────────────────────────────────────────────────────────────────

export const getLibraryPath = ()                => invoke<string>('get_library_path');
export const revealInFinder = (path: string)   => invoke<void>('reveal_in_finder', { path });
