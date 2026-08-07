import type { Game, GameDto, GameMeta, Settings } from "../types";

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

/** Un genre résolu pour un jeu (renvoyé par `enrich_genres`). */
export interface GenreUpdate {
  id: string;
  genre: string;
}

/**
 * Peuple le genre de toute la bibliothèque via IGDB (cross-plateforme : Fortnite,
 * Valorant, WoW…), en cache disque côté Rust. Les genres arrivent par lots via
 * l'événement `genre-batch` (`onBatch` appelé au fil de l'eau pour un affichage
 * progressif) ; l'ensemble final est aussi renvoyé en filet de sécurité. No-op hors Tauri.
 */
export async function enrichGenres(
  onBatch: (updates: GenreUpdate[]) => void,
): Promise<void> {
  let unlisten: (() => void) | null = null;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<GenreUpdate[]>("genre-batch", (e) => onBatch(e.payload));
  } catch {
    // Hors Tauri : pas d'événements.
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const all = await invoke<GenreUpdate[]>("enrich_genres");
    onBatch(all); // filet : réapplique le total (fusion idempotente).
  } catch (err) {
    console.info("[ludo] enrich_genres indisponible hors Tauri", err);
  } finally {
    if (unlisten) unlisten();
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

/** Renvoie l'état des connexions de comptes. */
export async function getSettings(): Promise<Settings | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<Settings>("get_settings");
  } catch {
    return null;
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
