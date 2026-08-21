export type PlatformId = "steam" | "epic" | "gog" | "riot" | "ubisoft" | "ea" | "battlenet" | "manual";

export interface Platform {
  id: PlatformId;
  name: string;
  /** CSS custom property that holds the brand color, e.g. "var(--steam)" */
  color: string;
}

/** Données brutes renvoyées par la commande Rust `scan_library` (camelCase). */
export interface GameDto {
  id: string;
  title: string;
  platform: PlatformId;
  installed: boolean;
  owned?: boolean;
  familyShared?: boolean;
  /** SteamIDs des membres de la famille qui possèdent ce jeu (nb de copies). */
  familyOwners?: string[];
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
  /**
   * Jaquette telle qu'elle est stockée (URL web ou chemin sur le disque), avant
   * conversion en URL affichable par la webview. Sert à ré-éditer un jeu manuel sans
   * lui coller l'URL `asset://` de rendu dans le formulaire.
   */
  coverSource?: string;
  /** Visuel paysage (hero, tuiles Salon, bannière détail) si disponible. */
  heroUrl?: string;
  /** Captures d'écran (fournisseur en ligne). */
  screenshots?: string[];
  installed: boolean;
  /** Présent dans la bibliothèque du compte (installé ou non). */
  owned?: boolean;
  /** Accessible via le partage familial Steam (possédé par un proche). */
  familyShared?: boolean;
  /** SteamIDs des membres de la famille qui possèdent ce jeu (nb de copies). */
  familyOwners?: string[];
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

// --- Boutique (découverte de jeux à acheter, source CheapShark) ---------------

/** Un jeu de la vitrine ou des résultats de recherche (carte de grille). */
export interface StoreItem {
  /** Identifiant ITAD du jeu (UUID ; clé de la fiche produit). */
  gameId: string;
  title: string;
  /** Jaquette (ITAD boxart) si disponible, sinon dégradé côté front. */
  coverUrl?: string | null;
  /** Prix actuel le plus bas (EUR). */
  price: number;
  /** Prix normal (hors promo) ; == price si inconnu. */
  normalPrice: number;
  /** Remise en % entier (0 = pas de promo / inconnu). */
  savings: number;
  /** Boutique de la meilleure offre (vide si non résolu). */
  storeName: string;
  /** Lien d'achat direct. */
  buyUrl: string;
}

/** Suggestion d'autocomplétion de la barre de recherche (titre + jaquette). */
export interface StoreSuggestion {
  gameId: string;
  title: string;
  coverUrl?: string | null;
}

/** Une offre d'une boutique (ligne du comparatif de la fiche produit). */
export interface StorePrice {
  storeName: string;
  price: number;
  retailPrice: number;
  savings: number;
  /** Lien d'achat direct vers la boutique. */
  buyUrl: string;
  /** false = en rupture de stock (Instant Gaming). Absent/undefined = disponible. */
  available?: boolean;
}

/** Fiche produit boutique : comparatif de prix + métadonnée descriptive (IGDB). */
export interface StoreGame {
  gameId: string;
  title: string;
  coverUrl?: string | null;
  heroUrl?: string | null;
  /** Prix le plus bas jamais atteint (EUR), si connu. */
  cheapestEver?: number | null;
  /** Offres par boutique, triées par prix croissant. */
  prices: StorePrice[];
  description?: string | null;
  genre?: string | null;
  developer?: string | null;
  year?: number | null;
  screenshots: string[];
}

/** Un ami Steam avec sa présence (commande Rust `steam_friends`). */
export interface Friend {
  steamId: string;
  name: string;
  avatarUrl: string;
  /** "in-game" | "online" | "away" | "busy" | "snooze" | "offline" */
  state: string;
  /** Jeu en cours (si en jeu). */
  gameName?: string | null;
  profileUrl: string;
}

/** Profil Steam de l'utilisateur connecté (commande Rust `steam_me`), pour l'en-tête. */
export interface SteamProfile {
  steamId: string;
  name: string;
  avatarUrl: string;
  profileUrl: string;
}

/** Un succès Steam d'un jeu (commande Rust `steam_achievements`). */
export interface SteamAchievement {
  name: string;
  description: string;
  icon: string;
  unlocked: boolean;
  /** Texte de déblocage localisé (ex. « Débloqué le 30 aout 2023 à 10h28 »), si débloqué. */
  unlockedAt?: string | null;
}

/** Les succès d'un jeu Steam pour l'utilisateur (commande Rust `steam_achievements`). */
export interface SteamAchievements {
  unlocked: number;
  total: number;
  items: SteamAchievement[];
}

/** Un ami dans la vue « Jeux en commun » (commande Rust `friends_common`). */
export interface FriendLib {
  steamId: string;
  name: string;
  avatarUrl: string;
  /** Vrai si sa bibliothèque est privée (illisible) → non filtrable. */
  private: boolean;
  /** Nombre de jeux qu'il possède en commun avec moi. */
  commonCount: number;
}

/** Un de MES jeux Steam, avec les amis qui le possèdent aussi. */
export interface CommonGame {
  id: string;
  title: string;
  coverUrl?: string | null;
  /** SteamIDs des amis (lisibles) qui possèdent ce jeu. */
  owners: string[];
}

/** Charge utile de la commande `friends_common`. */
export interface FriendsCommon {
  friends: FriendLib[];
  games: CommonGame[];
  fetchedAt: number;
}

/* ── Service social Torii (comptes, amis, présence) ─────────────────────────── */

/** Le compte Torii de l'utilisateur. */
export interface ToriiAccount {
  id: string;
  email: string;
  displayName: string;
  /** Code à donner de la main à la main pour se faire ajouter. */
  friendCode: string;
  steamId?: string | null;
  /** Autorise les amis Steam à nous retrouver (les deux côtés doivent l'activer). */
  steamDiscoverable: boolean;
}

/**
 * État d'un ami. `offline` signifie « aucune nouvelle depuis 90 s », donc Torii fermé —
 * et non « ne joue pas » : une partie lancée sans Torii reste invisible.
 */
export type ToriiStatus = "in-game" | "online" | "away" | "offline";

export interface ToriiFriend {
  id: string;
  displayName: string;
  /** SteamID si l'ami s'est rendu découvrable : permet de fusionner avec sa fiche Steam. */
  steamId?: string | null;
  status: ToriiStatus;
  /** Clé de jeu cross-launcher, pour reconnaître le même jeu d'un launcher à l'autre. */
  gameKey?: string | null;
  gameTitle?: string | null;
  /** Début de la partie (Unix), pour afficher « depuis 1 h 20 ». */
  since?: number | null;
}

/** Une personne sans présence : demande d'ami en attente, ou suggestion. */
export interface ToriiPerson {
  id: string;
  displayName: string;
  steamId?: string | null;
}

/** Résultat d'une connexion : le compte, et s'il vient d'être créé. */
export interface ToriiSignIn {
  account: ToriiAccount;
  created: boolean;
}

export interface ToriiCircle {
  friends: ToriiFriend[];
  incoming: ToriiPerson[];
  outgoing: ToriiPerson[];
}

/**
 * Ce qu'on laisse voir de soi aux amis :
 *   - `offline`  : personne ne te voit ;
 *   - `online`   : ils te savent connecté, sans savoir à quoi tu joues ;
 *   - `detailed` : ils voient le jeu et depuis quand.
 */
export type PresenceMode = "offline" | "online" | "detailed";

/** Réglages de partage. Le mode est `offline` tant qu'on n'a rien choisi. */
export interface SocialPrefs {
  presenceMode?: PresenceMode | null;
  /** Ancien réglage booléen, encore lu pour ne pas réinitialiser les comptes existants. */
  sharePresence: boolean;
  awayAfterMinutes: number;
}

/** Un jeu de la wishlist Steam enrichi de prix (commande Rust `steam_wishlist`). */
export interface WishlistItem {
  appId: number;
  /** Identifiant ITAD (ouvre la fiche Boutique) ; vide si absent d'ITAD. */
  gameId: string;
  title: string;
  coverUrl: string;
  /**
   * Jaquette de repli (boxart ITAD) si la capsule Steam n'existe pas — c'est le cas
   * de beaucoup de jeux récents ou pas encore sortis.
   */
  coverFallbackUrl?: string | null;
  /** Meilleur prix actuel (EUR) ; null si aucune offre / non résolu. */
  price?: number | null;
  normalPrice?: number | null;
  savings: number;
  storeName: string;
  buyUrl: string;
  /** Plus bas prix historique (EUR), si connu. */
  historyLow?: number | null;
}

export type AppMode = "bureau" | "salon";

/** Critère de tri de la bibliothèque. */
export type SortKey = "recent" | "alpha" | "playtime";
