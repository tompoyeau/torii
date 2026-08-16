import type { Friend, FriendsCommon, Game, GameDto, GameMeta, Settings, SteamAchievements, SteamProfile, StoreGame, StoreItem, StoreSuggestion, WishlistItem } from "../types";

/** Champs saisis par l'utilisateur pour ajouter un jeu à la main. */
export interface ManualInput {
  title: string;
  launchTarget: string;
  installDir?: string | null;
  coverUrl?: string | null;
}

/**
 * Pont vers la couche native Rust.
 * Les appels sont protégés pour rester silencieux hors contexte Tauri
 * (ex: `npm run dev` dans un navigateur classique).
 */
export async function launchGame(game: Game): Promise<void> {
  await launchSource(game.platform, game.launchTarget);
}

/** Lance une provenance précise (plateforme + cible), pour les jeux multi-sources. */
export async function launchSource(platform: string, target?: string): Promise<void> {
  let invoke: typeof import("@tauri-apps/api/core").invoke;
  try {
    ({ invoke } = await import("@tauri-apps/api/core"));
  } catch (err) {
    // Hors contexte Tauri (ex: `npm run dev` dans un navigateur) : rien à lancer.
    console.info(`[ludo] launch_game indisponible hors Tauri (${platform})`, err);
    return;
  }
  // Sous Tauri : une erreur du backend est un vrai échec de lancement (≠ « hors Tauri »),
  // on la journalise en `error` pour qu'elle soit visible (au lieu d'être avalée en `info`).
  try {
    await invoke("launch_game", { platform, target: target ?? "" });
  } catch (err) {
    console.error(`[ludo] échec du lancement (${platform} / ${target ?? ""})`, err);
  }
}

/** Déclenche l'installation d'un jeu (ouvre le launcher sur son flux d'installation). */
export async function installGame(game: Game): Promise<void> {
  await installSource(game.platform, game.launchTarget);
}

/** Installe une provenance précise (plateforme + cible), pour les jeux multi-sources. */
export async function installSource(platform: string, target?: string): Promise<void> {
  let invoke: typeof import("@tauri-apps/api/core").invoke;
  try {
    ({ invoke } = await import("@tauri-apps/api/core"));
  } catch (err) {
    console.info(`[ludo] install_game indisponible hors Tauri (${platform})`, err);
    return;
  }
  try {
    await invoke("install_game", { platform, target: target ?? "" });
  } catch (err) {
    console.error(`[ludo] échec de l'installation (${platform} / ${target ?? ""})`, err);
  }
}

/**
 * Enregistre « maintenant » comme dernière session du jeu (au clic sur Jouer).
 * Donne une date de dernière session aux jeux sans stats de launcher (Riot/EA/Battle.net…).
 * Renvoie l'horodatage Unix posé, ou null hors Tauri.
 */
export async function recordLaunch(id: string): Promise<number | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<number>("record_launch", { id });
  } catch {
    return null;
  }
}

/**
 * Ouvre la fenêtre de connexion Steam officielle et récupère la session.
 * Bloque jusqu'à ce que l'utilisateur se connecte (ou expiration).
 */
export async function connectSteam(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("connect_steam");
}

/** Déconnecte Steam (efface la session locale). */
export async function disconnectSteam(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("disconnect_steam");
}

/**
 * Ouvre la fenêtre de connexion GOG officielle (OAuth) et stocke le refresh token.
 * Bloque jusqu'à ce que l'utilisateur se connecte (ou expiration).
 */
export async function connectGog(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("connect_gog");
}

/** Déconnecte GOG (efface le refresh token local). */
export async function disconnectGog(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("disconnect_gog");
}

/**
 * Ouvre la fenêtre de connexion Epic officielle (OAuth) et stocke le refresh token.
 * Bloque jusqu'à ce que l'utilisateur se connecte (ou expiration).
 */
export async function connectEpic(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("connect_epic");
}

/** Déconnecte Epic (efface le refresh token local). */
export async function disconnectEpic(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("disconnect_epic");
}

/**
 * Ouvre la fenêtre de connexion EA, récupère la bibliothèque possédée (API Juno)
 * et la met en cache. Bloque jusqu'à la connexion (ou expiration).
 */
export async function connectEa(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("connect_ea");
}

/** Déconnecte EA (supprime le snapshot de bibliothèque en cache). */
export async function disconnectEa(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("disconnect_ea");
}

/**
 * Ouvre la fenêtre de connexion Battle.net, récupère la bibliothèque possédée
 * (API games-and-subs) et la met en cache. Bloque jusqu'à la connexion (ou expiration).
 */
export async function connectBattlenet(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("connect_battlenet");
}

/** Déconnecte Battle.net (supprime le snapshot de bibliothèque en cache). */
export async function disconnectBattlenet(): Promise<Settings> {
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Settings>("disconnect_battlenet");
}

/**
 * Remplit les jaquettes manquantes de toute la bibliothèque via le Steam Store
 * (recherche par titre, sans clé), quel que soit le launcher. Résultats mis en cache
 * côté Rust. Renvoie les jaquettes résolues (ou [] hors Tauri / en cas d'échec).
 */
export async function enrichCovers(): Promise<{ id: string; coverUrl: string }[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<{ id: string; coverUrl: string }[]>("enrich_covers");
  } catch (err) {
    console.info("[ludo] enrich_covers indisponible hors Tauri", err);
    return [];
  }
}

/** Métadonnée IGDB résolue pour un jeu (renvoyée par `enrich_igdb`). */
export interface MetaUpdate {
  id: string;
  genre?: string | null;
  description?: string | null;
  coverUrl?: string | null;
  heroUrl?: string | null;
  developer?: string | null;
  year?: number | null;
  screenshots: string[];
}

/**
 * Peuple la métadonnée descriptive de toute la bibliothèque via IGDB (source unique,
 * cross-plateforme : Fortnite, Valorant, WoW…) : genre, description, captures, jaquette
 * de repli, hero, studio, année. Cache disque côté Rust. Les résultats arrivent par lots
 * via l'événement `igdb-batch` (`onBatch` appelé au fil de l'eau pour un affichage
 * progressif) ; l'ensemble final est aussi renvoyé en filet de sécurité. No-op hors Tauri.
 */
export async function enrichIgdb(
  onBatch: (updates: MetaUpdate[]) => void,
): Promise<void> {
  let unlisten: (() => void) | null = null;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<MetaUpdate[]>("igdb-batch", (e) => onBatch(e.payload));
  } catch {
    // Hors Tauri : pas d'événements.
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const all = await invoke<MetaUpdate[]>("enrich_igdb");
    onBatch(all); // filet : réapplique le total (fusion idempotente).
  } catch (err) {
    console.info("[ludo] enrich_igdb indisponible hors Tauri", err);
  } finally {
    if (unlisten) unlisten();
  }
}

/**
 * Boutique — vitrine : une page de jeux mis en avant / en promo (CheapShark), selon
 * le tri (`featured`, `savings`, `price`, `recent`, `rating`). Renvoie `null` hors Tauri.
 */
export async function storeDeals(
  page: number,
  sort: string,
): Promise<StoreItem[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<StoreItem[]>("store_deals", { page, sort });
  } catch (err) {
    console.info("[ludo] store_deals indisponible hors Tauri", err);
    return null;
  }
}

/** Boutique — recherche de jeux par titre. Renvoie `null` hors Tauri. */
export async function storeSearch(query: string): Promise<StoreItem[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<StoreItem[]>("store_search", { query });
  } catch (err) {
    console.info("[ludo] store_search indisponible hors Tauri", err);
    return null;
  }
}

/** Boutique — suggestions d'autocomplétion (léger). `null` hors Tauri. */
export async function storeSuggest(query: string): Promise<StoreSuggestion[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<StoreSuggestion[]>("store_suggest", { query });
  } catch (err) {
    console.info("[ludo] store_suggest indisponible hors Tauri", err);
    return null;
  }
}

/** Boutique — fiche produit (comparatif de prix + méta IGDB). `null` hors Tauri. */
export async function storeGame(gameId: string): Promise<StoreGame | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<StoreGame | null>("store_game", { gameId });
  } catch (err) {
    console.info("[ludo] store_game indisponible hors Tauri", err);
    return null;
  }
}

/** Ouvre un lien externe (achat) dans le navigateur par défaut, hors de l'app. */
export async function openExternal(url: string): Promise<void> {
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch {
    // Hors Tauri (preview navigateur) : ouverture classique.
    window.open(url, "_blank", "noopener");
  }
}

/** Enregistre la clé API Steam (chemin avancé, chaîne vide = effacement). */
export async function setSteamKey(key: string): Promise<Settings | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<Settings>("set_steam_key", { key });
  } catch (err) {
    console.info("[ludo] set_steam_key indisponible hors Tauri", err);
    return null;
  }
}

/**
 * Enrichit un seul jeu à la demande (ouverture du détail) : description,
 * captures, développeur, année, genre. Résultat mis en cache côté Rust.
 * Renvoie `null` hors Tauri.
 */
export async function enrichGame(game: Game): Promise<GameMeta | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<GameMeta>("enrich_game", {
      id: game.id,
      platform: game.platform,
      launchTarget: game.launchTarget ?? "",
      title: game.title,
      installed: game.installed,
    });
  } catch (err) {
    console.info(`[ludo] enrich_game indisponible hors Tauri (${game.title})`, err);
    return null;
  }
}

/** Masque ou réaffiche un jeu (liste d'exclusion). Renvoie les ids masqués. */
export async function setGameHidden(id: string, hidden: boolean): Promise<string[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<string[]>("set_game_hidden", { id, hidden });
  } catch (err) {
    console.info("[ludo] set_game_hidden indisponible hors Tauri", err);
    return [];
  }
}

/** Épingle ou retire un jeu des favoris. Renvoie les ids favoris. */
export async function setGameFavorite(id: string, favorite: boolean): Promise<string[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<string[]>("set_game_favorite", { id, favorite });
  } catch (err) {
    console.info("[ludo] set_game_favorite indisponible hors Tauri", err);
    return [];
  }
}

/** Ouvre le dossier d'installation d'un jeu dans l'explorateur de fichiers. No-op hors Tauri. */
export async function openInstallDir(dir: string): Promise<void> {
  let invoke: typeof import("@tauri-apps/api/core").invoke;
  try {
    ({ invoke } = await import("@tauri-apps/api/core"));
  } catch (err) {
    console.info("[ludo] open_install_dir indisponible hors Tauri", err);
    return;
  }
  try {
    await invoke("open_install_dir", { path: dir });
  } catch (err) {
    // Erreur backend réelle (ex. dossier disparu) : on la remonte visiblement en console.
    console.error(`[ludo] impossible d'ouvrir le dossier « ${dir} »`, err);
  }
}

/**
 * Déclenche la désinstallation d'un jeu installé : délègue à l'UI native du launcher
 * (Steam/Epic/GOG/Ubisoft/EA), qui affiche sa propre confirmation. Pour les jeux fusionnés,
 * on cible la provenance installée.
 */
export async function uninstallGame(game: Game): Promise<void> {
  const src = game.sources?.find((s) => s.installed);
  const platform = src?.platform ?? game.platform;
  const target = src?.launchTarget ?? game.launchTarget ?? "";
  let invoke: typeof import("@tauri-apps/api/core").invoke;
  try {
    ({ invoke } = await import("@tauri-apps/api/core"));
  } catch (err) {
    console.info(`[ludo] uninstall_game indisponible hors Tauri (${platform})`, err);
    return;
  }
  try {
    await invoke("uninstall_game", { platform, target, installDir: game.installDir ?? null });
  } catch (err) {
    console.error(`[ludo] échec de la désinstallation (${platform} / ${target})`, err);
    throw err;
  }
}

/**
 * Ajoute un jeu saisi manuellement (persisté dans `manual_games.json`).
 * Renvoie la liste à jour des jeux manuels (ou `null` hors Tauri).
 */
export async function addManualGame(input: ManualInput): Promise<GameDto[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<GameDto[]>("add_manual_game", { input });
  } catch (err) {
    console.info("[ludo] add_manual_game indisponible hors Tauri", err);
    return null;
  }
}

/** Retire un jeu manuel par son id. Renvoie la liste à jour (ou `null` hors Tauri). */
export async function removeManualGame(id: string): Promise<GameDto[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<GameDto[]>("remove_manual_game", { id });
  } catch (err) {
    console.info("[ludo] remove_manual_game indisponible hors Tauri", err);
    return null;
  }
}

/** Liste d'amis Steam + présence (vide si Steam non connecté). `null` hors Tauri. */
export async function steamFriends(): Promise<Friend[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<Friend[]>("steam_friends");
  } catch (err) {
    console.info("[ludo] steam_friends indisponible hors Tauri", err);
    return null;
  }
}

/** Profil Steam de l'utilisateur (pseudo + avatar). `null` hors Tauri ou si non connecté. */
export async function steamMe(): Promise<SteamProfile | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SteamProfile | null>("steam_me");
  } catch (err) {
    console.info("[ludo] steam_me indisponible hors Tauri", err);
    // Hors Tauri (preview) : profil fictif pour la maquette.
    return {
      steamId: "0",
      name: "PomPoteau",
      avatarUrl: "https://avatars.fastly.steamstatic.com/3604ac34b47c87e187d151f22aa17e107253ce34_full.jpg",
      profileUrl: "#",
    };
  }
}

/**
 * Succès Steam d'un jeu (`appid`). `null` hors Tauri, jeu non-Steam, jeu sans succès,
 * ou Steam non connecté. Hors Tauri (preview) : jeu de succès fictif pour la maquette.
 */
export async function steamAchievements(appid: string): Promise<SteamAchievements | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SteamAchievements | null>("steam_achievements", { appid: Number(appid) });
  } catch (err) {
    console.info("[ludo] steam_achievements indisponible hors Tauri", err);
    const icon = (h: string) =>
      `https://shared.fastly.steamstatic.com/community_assets/images/apps/1086940/${h}.jpg`;
    return {
      unlocked: 3,
      total: 8,
      items: [
        { name: "Fuite de l'Avernus", description: "Prendre le contrôle du nautiloïde et vous enfuir des Enfers.", icon: icon("0cb31fd9ec036550a374aa702a37464a98da3bfa"), unlocked: true, unlockedAt: "Débloqué le 30 aout 2023 à 10h28" },
        { name: "De Charybde en Scylla", description: "Quitter l'acte 1 pour vous rendre dans un lieu bien plus sombre.", icon: icon("628cdbbfd2e731735e4817252ce6633bf3bcd8ed"), unlocked: true, unlockedAt: "Débloqué le 19 nov. 2023 à 8h24" },
        { name: "La cité vous attend", description: "Quitter l'acte 2 pour rejoindre la Porte de Baldur.", icon: icon("3c6d05ff648b66925238963a658ee307e31ff870"), unlocked: true, unlockedAt: "Débloqué le 26 janv. 2024 à 14h31" },
        { name: "Tout est bien qui finit bien", description: "Terminer le jeu.", icon: icon("0cb31fd9ec036550a374aa702a37464a98da3bfa"), unlocked: false, unlockedAt: null },
        { name: "L'appel du sang", description: "Boire le sang d'un ennemi vaincu.", icon: icon("628cdbbfd2e731735e4817252ce6633bf3bcd8ed"), unlocked: false, unlockedAt: null },
      ],
    };
  }
}

/**
 * Nombre de joueurs en ce moment sur un jeu Steam (`appid`), via l'API publique.
 * `null` hors Tauri (→ mock) ou si indisponible.
 */
export async function steamCurrentPlayers(appid: string): Promise<number | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<number | null>("steam_current_players", { appid: Number(appid) });
  } catch (err) {
    console.info("[ludo] steam_current_players indisponible hors Tauri", err);
    return 40356; // mock preview
  }
}

/** Wishlist Steam enrichie de prix (ITAD). `null` hors Tauri (→ mock). */
export async function steamWishlist(): Promise<WishlistItem[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<WishlistItem[]>("steam_wishlist");
  } catch (err) {
    console.info("[ludo] steam_wishlist indisponible hors Tauri", err);
    return null;
  }
}

/** Wishlist unifiée (Steam native ∪ Torii) enrichie de prix. `null` hors Tauri (→ mock). */
export async function wishlistAll(): Promise<WishlistItem[] | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<WishlistItem[]>("wishlist_all");
  } catch {
    return null;
  }
}

/** Ids (ITAD) présents dans la wishlist Torii (état des boutons ♥). [] hors Tauri. */
export async function wishlistIds(): Promise<string[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<string[]>("wishlist_ids");
  } catch {
    return [];
  }
}

/**
 * Ajoute un jeu à la wishlist Torii (et à Steam en bonus si le jeu y existe).
 * Renvoie `true` si le push vers Steam a réussi. No-op → false hors Tauri.
 */
export async function wishlistAdd(id: string, title: string, coverUrl?: string | null): Promise<boolean> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<boolean>("wishlist_add", { id, title, coverUrl: coverUrl ?? null });
  } catch {
    return false;
  }
}

/** Retire un jeu de la wishlist Torii (et de Steam si applicable). */
export async function wishlistRemove(id: string): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("wishlist_remove", { id });
  } catch {
    /* hors Tauri : no-op. */
  }
}

/** Jeux en commun avec les amis Steam. `null` hors Tauri (→ mock). */
export async function friendsCommon(force = false): Promise<FriendsCommon | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<FriendsCommon>("friends_common", { force });
  } catch (err) {
    console.info("[ludo] friends_common indisponible hors Tauri", err);
    return null;
  }
}

/** Renvoie l'état des connexions de comptes. */
export async function getSettings(): Promise<Settings | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<Settings>("get_settings");
  } catch {
    return null;
  }
}

/**
 * Vide les caches de métadonnées/jaquettes/prix. Renvoie le nombre de fichiers
 * supprimés, ou null hors Tauri.
 */
export async function clearCaches(): Promise<number | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<number>("clear_caches");
  } catch {
    return null;
  }
}

/**
 * Arme le suivi d'une session de jeu : Torii se minimise, surveille le process du
 * jeu (sous `installDir`) et, à sa fermeture, revient au premier plan puis émet
 * l'événement `game-exited`. No-op hors Tauri.
 */
export async function startGameWatch(gameId: string, installDir: string): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("start_game_watch", { gameId, installDir });
  } catch {
    /* hors Tauri : no-op. */
  }
}

/** S'abonne à la fermeture d'un jeu suivi ; renvoie une fonction de désabonnement. */
export async function onGameExited(cb: (gameId: string) => void): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<{ id: string }>("game-exited", (e) => cb(e.payload.id));
  } catch {
    return () => {};
  }
}

/** Affiche une notification système (no-op hors Tauri). */
export async function notify(title: string, body: string): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("notify_user", { title, body });
  } catch {
    /* hors Tauri : no-op. */
  }
}

/** Préférences de fenêtre (démarrage minimisé, fermeture dans le tray). */
export interface WindowPrefs {
  startMinimized: boolean;
  closeToTray: boolean;
}
export async function getWindowPrefs(): Promise<WindowPrefs> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<WindowPrefs>("get_window_prefs");
  } catch {
    return { startMinimized: false, closeToTray: false };
  }
}
export async function setWindowPrefs(prefs: WindowPrefs): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("set_window_prefs", {
      startMinimized: prefs.startMinimized,
      closeToTray: prefs.closeToTray,
    });
  } catch {
    /* hors Tauri : no-op. */
  }
}

/** Version de l'application (depuis Tauri), ou null hors Tauri. */
export async function appVersion(): Promise<string | null> {
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return await getVersion();
  } catch {
    return null;
  }
}

/** Indique si Torii démarre automatiquement avec Windows (false hors Tauri). */
export async function getAutostart(): Promise<boolean> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<boolean>("get_autostart");
  } catch {
    return false;
  }
}

/**
 * Active/désactive le démarrage automatique avec Windows. Renvoie l'état effectif
 * (hors Tauri : renvoie la valeur demandée, sans effet).
 */
export async function setAutostart(enabled: boolean): Promise<boolean> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<boolean>("set_autostart", { enabled });
  } catch {
    return enabled;
  }
}

/** S'abonne à la progression de l'enrichissement. Renvoie une fonction de désabonnement. */
export async function onEnrichProgress(
  cb: (done: number, total: number) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<{ done: number; total: number }>("enrich-progress", (e) =>
      cb(e.payload.done, e.payload.total),
    );
  } catch {
    return () => {};
  }
}
