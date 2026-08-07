export type PlatformId = "steam" | "epic" | "gog" | "riot" | "ubisoft" | "ea" | "battlenet" | "manual";

export interface Platform {
  id: PlatformId;
  name: string;
  /** CSS custom property that holds the brand color, e.g. "var(--steam)" */
  color: string;
}

/** Données brutes renvoyées par les commandes Rust `scan_library` / `enrich_metadata` (camelCase). */
export interface GameDto {
  id: string;
  title: string;
  platform: PlatformId;
  installed: boolean;
  owned?: boolean;
  familyShared?: boolean;
  playtimeMinutes?: number | null;
  sizeGb: number;
  installDir?: string | null;
  coverUrl?: string | null;
  heroUrl?: string | null;
  launchTarget: string;
  lastPlayed?: number | null;
  genre?: string | null;
  description?: string | null;
  developer?: string | null;
  year?: number | null;
  screenshots?: string[];
  hidden?: boolean;
  favorite?: boolean;
}

/** Métadonnées enrichies d'un jeu (commande Rust `enrich_game`, camelCase). */
export interface GameMeta {
  name?: string | null;
  genre?: string | null;
  description?: string | null;
  developer?: string | null;
  year?: number | null;
  coverUrl?: string | null;
  heroUrl?: string | null;
  screenshots?: string[];
  appType?: string | null;
  sizeGb?: number | null;
}

/** État des connexions de comptes (commande Rust `get_settings`). */
export interface Settings {
  steamConnected: boolean;
  steamId?: string | null;
  gogConnected: boolean;
  epicConnected: boolean;
  eaConnected: boolean;
  battlenetConnected: boolean;
}

/** Une provenance jouable d'un jeu (une plateforme où il est possédé). */
export interface GameSource {
  platform: PlatformId;
  launchTarget?: string;
  installed: boolean;
}

export interface Game {
  id: string;
  title: string;
  platform: PlatformId;
  /** Dégradé CSS servant de jaquette de secours (toujours présent). */
  cover: string;
  /** Vraie jaquette portrait (Steam CDN, fichier local…) si disponible. */
  coverUrl?: string;
  /** Visuel paysage (hero, tuiles Salon, bannière détail) si disponible. */
  heroUrl?: string;
  /** Captures d'écran (fournisseur en ligne). */
  screenshots?: string[];
  installed: boolean;
  /** Présent dans la bibliothèque du compte (installé ou non). */
  owned?: boolean;
  /** Accessible via le partage familial Steam (possédé par un proche). */
  familyShared?: boolean;
  favorite: boolean;
  recent: boolean;
  /** Cible de lancement transmise à Rust (appid, URI, exe…). */
  launchTarget?: string;
  installDir?: string;
  // Métadonnées enrichies — optionnelles (absentes d'un simple scan local).
  sizeGb?: number;
  hoursPlayed?: number;
  /** Dernière session (affichage relatif, ex. « il y a 2 h »). */
  lastPlayed?: string;
  /** Dernière session en horodatage Unix (secondes) — pour le tri. */
  lastPlayedAt?: number;
  developer?: string;
  year?: number;
  genre?: string;
  description?: string;
  achievements?: { unlocked: number; total: number };
  /** Masqué par l'utilisateur (liste d'exclusion). */
  hidden?: boolean;
  /** Provenances jouables si le jeu est possédé sur plusieurs plateformes
   * (doublons fusionnés). Absent/1 élément = jeu mono-plateforme. */
  sources?: GameSource[];
}

export type LibraryFilter =
  | "all"
  | "mine"
  | "family"
  | "recent"
  | "favorite"
  | "installed"
  | "hidden"
  | PlatformId;

export type AppMode = "bureau" | "salon";

/** Critère de tri de la bibliothèque. */
export type SortKey = "recent" | "alpha" | "playtime";
