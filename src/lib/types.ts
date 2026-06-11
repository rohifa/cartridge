export type Platform = 'steam' | 'gog' | 'epic';
export type GameStatus = 'backlog' | 'playing' | 'beaten' | 'dropped' | 'wishlist';

export interface GameCopy {
  platform: Platform;
  appId: string;
  accountId?: string;
  installed: boolean;
  playtime: number | null;
  last: string | null;
}

export interface Game {
  id: number;
  title: string;
  cover: string | null;
  
  // User-owned, game-level data
  rating: number;
  status: GameStatus;
  tags: string[];
  note: string;
  
  // Store metadata (same across platforms)
  genre: string[];
  dev: string;
  year: number;
  metacritic: number | null;
  opencritic: number | null;
  userScore: number | null;
  links: Partial<Record<'metacritic' | 'opencritic' | 'hltb' | 'store', string>>;
  custom: Array<{ k: string; v: string }>;
  
  // Platform-specific copies
  copies: GameCopy[];
}

export type GamePatch = Partial<Omit<Game, 'id' | 'copies'>>;

export interface SyncResult {
  added: number;
  updated: number;
  errors: string[];
}

export interface Account {
  id: string;
  name: string;
  handle: string;
  platform: Platform;
  count: number;
  synced: string;
  status: 'ok' | 'stale' | 'error';
}

export type ListColor = 'green' | 'purple' | 'amber' | 'teal' | 'blue' | 'red';
export type ListKind = 'smart' | 'manual';

export interface GameList {
  id: string;
  name: string;
  color: ListColor;
  kind: ListKind;
  icon: string;
  rules?: Partial<Filters>;
  gameIds?: number[];
}

export interface Filters {
  genres: string[];
  platforms: Platform[];
  statuses: GameStatus[];
  yearLo: number;
  yearHi: number;
  mcMin: number;
  ocMin: number;
  usrMin: number;
  playtimeMax?: number;
}

export interface Prefs {
  defaultView: 'grid' | 'list';
  showScores: boolean;
  confirmLaunch: boolean;
  autoSync: boolean;
}

export type AppView = 'library' | 'settings';

export type SidebarFilter =
  | { type: 'all' }
  | { type: 'recent' }
  | { type: 'fav' }
  | { type: 'never' }
  | { type: 'platform'; value: Platform }
  | { type: 'list'; value: string };

export interface CartridgeData {
  version?: number;
  accounts: Account[];
  games: Game[];
  lists: GameList[];
  genres: string[];
  statuses: Array<{ id: GameStatus; label: string; icon: string }>;
  prefs: Prefs;
}
