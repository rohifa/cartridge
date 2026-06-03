export type Platform = 'steam' | 'gog' | 'epic';
export type GameStatus = 'backlog' | 'playing' | 'beaten' | 'dropped' | 'wishlist';

export interface Game {
  id: number;
  title: string;
  cover: string | null;
  platform: Platform;
  genre: string[];
  dev: string;
  year: number;
  playtime: number | null;
  last: string | null;
  rating: number;
  status: GameStatus;
  installed: boolean;
  metacritic: number | null;
  opencritic: number | null;
  userScore: number | null;
  tags: string[];
  note: string;
  links: Partial<Record<'metacritic' | 'opencritic' | 'hltb' | 'store', string>>;
  custom: Array<{ k: string; v: string }>;
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
  accounts: Account[];
  games: Game[];
  lists: GameList[];
  genres: string[];
  statuses: Array<{ id: GameStatus; label: string; icon: string }>;
}
