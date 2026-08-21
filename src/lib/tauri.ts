import type { Friend, FriendsCommon, Game, GameDto, GameMeta, Settings, SocialPrefs, SteamAchievements, SteamProfile, StoreGame, StoreItem, StoreSuggestion, ToriiAccount, ToriiCircle, ToriiPerson, ToriiSignIn, WishlistItem } from "../types";

/** Champs saisis par l'utilisateur pour ajouter un jeu à la main. */
export interface ManualInput {
  title: string;
  launchTarget: string;
  installDir?: string | null;
  coverUrl?: string | null;
}

/**
 * Pont vers la couche native Rust.
 *
 * Tout passe par `call` / `callOrThrow`, qui distinguent deux échecs longtemps
 * confondus dans un même `try/catch` :
 *   - **hors Tauri** (`npm run dev` dans un navigateur) : l'import du module échoue
 *     → repli silencieux, l'app reste utilisable sur des données fictives ;
 *   - **erreur du backend** (commande qui échoue vraiment) : journalisée en
 *     `console.error` au lieu d'être déguisée en « hors Tauri ».
 */

type Args = Record<string, unknown>;
type Invoke = typeof import("@tauri-apps/api/core").invoke;

let cachedInvoke: Invoke | undefined;

/**
 * 🔑 Le module `@tauri-apps/api` s'importe très bien dans un navigateur nu : ce qui
 * échoue, c'est `invoke`, qui lit `window.__TAURI_INTERNALS__.invoke`. Tester la
 * présence de ce global est donc le seul moyen fiable de distinguer « hors Tauri »
 * d'une commande qui a vraiment échoué.
 */
function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function loadInvoke(): Promise<Invoke | null> {
  if (cachedInvoke) return cachedInvoke; // mémorisé une fois le module chargé
  if (!hasTauriRuntime()) return null;
  try {
    ({ invoke: cachedInvoke } = await import("@tauri-apps/api/core"));
  } catch {
    return null;
  }
  return cachedInvoke ?? null;
}

/** `true` dans l'application Tauri, `false` dans un navigateur nu (preview). */
export async function inTauri(): Promise<boolean> {
  return (await loadInvoke()) !== null;
}

/** Appelle une commande Rust ; renvoie `fallback` si elle est indisponible ou échoue. */
async function call<T>(cmd: string, args: Args | undefined, fallback: T): Promise<T> {
  const invoke = await loadInvoke();
  if (!invoke) {
    console.info(`[torii] ${cmd} indisponible hors Tauri`);
    return fallback;
  }
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    console.error(`[torii] échec de ${cmd}`, err);
    return fallback;
  }
}

/** Appelle une commande Rust en laissant remonter l'erreur (l'appelant la traite). */
async function callOrThrow<T>(cmd: string, args?: Args): Promise<T> {
  const invoke = await loadInvoke();
  if (!invoke) throw new Error(`${cmd} : indisponible hors de l'application Torii.`);
  return await invoke<T>(cmd, args);
}

// --- Lancement / installation -------------------------------------------------

export async function launchGame(game: Game): Promise<void> {
  await launchSource(game.platform, game.launchTarget);
}

/** Lance une provenance précise (plateforme + cible), pour les jeux multi-sources. */
export async function launchSource(platform: string, target?: string): Promise<void> {
  await call<void>("launch_game", { platform, target: target ?? "" }, undefined);
}

/** Déclenche l'installation d'un jeu (ouvre le launcher sur son flux d'installation). */
export async function installGame(game: Game): Promise<void> {
  await installSource(game.platform, game.launchTarget);
}

/** Installe une provenance précise (plateforme + cible), pour les jeux multi-sources. */
export async function installSource(platform: string, target?: string): Promise<void> {
  await call<void>("install_game", { platform, target: target ?? "" }, undefined);
}

/**
 * Déclenche la désinstallation d'un jeu installé : délègue à l'UI native du launcher
 * (Steam/Epic/GOG/Ubisoft/EA), qui affiche sa propre confirmation. Pour les jeux fusionnés,
 * on cible la provenance installée. L'erreur remonte (l'appelant sort de l'état « en cours »).
 */
export async function uninstallGame(game: Game): Promise<void> {
  const src = game.sources?.find((s) => s.installed);
  const platform = src?.platform ?? game.platform;
  const target = src?.launchTarget ?? game.launchTarget ?? "";
  if (!(await inTauri())) return; // preview navigateur : rien à désinstaller
  await callOrThrow<void>("uninstall_game", {
    platform,
    target,
    installDir: game.installDir ?? null,
  });
}

/**
 * Enregistre « maintenant » comme dernière session du jeu (au clic sur Jouer).
 * Donne une date de dernière session aux jeux sans stats de launcher (Riot/EA/Battle.net…).
 * Renvoie l'horodatage Unix posé, ou null hors Tauri.
 */
export async function recordLaunch(id: string): Promise<number | null> {
  return await call<number | null>("record_launch", { id }, null);
}

/** Ouvre le dossier d'installation d'un jeu dans l'explorateur de fichiers. No-op hors Tauri. */
export async function openInstallDir(dir: string): Promise<void> {
  await call<void>("open_install_dir", { path: dir }, undefined);
}

/**
 * Arme le suivi d'une session de jeu : Torii se minimise, surveille le process du
 * jeu (sous `installDir`) et, à sa fermeture, revient au premier plan puis émet
 * l'événement `game-exited`. No-op hors Tauri.
 */
export async function startGameWatch(gameId: string, installDir: string): Promise<void> {
  await call<void>("start_game_watch", { gameId, installDir }, undefined);
}

/** S'abonne à la fermeture d'un jeu suivi ; renvoie une fonction de désabonnement. */
export async function onGameLaunched(
  cb: (gameId: string, at: number) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<{ id: string; at: number }>("game-launched", (e) =>
      cb(e.payload.id, e.payload.at),
    );
  } catch {
    return () => {};
  }
}

export async function onGameExited(cb: (gameId: string) => void): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<{ id: string }>("game-exited", (e) => cb(e.payload.id));
  } catch {
    return () => {};
  }
}

// --- Comptes (connexion / déconnexion) ----------------------------------------
// Ces commandes ouvrent une fenêtre de login et bloquent jusqu'à la connexion (ou
// l'expiration) : leur erreur est affichée à l'utilisateur, donc on la laisse remonter.

export async function connectSteam(): Promise<Settings> {
  return await callOrThrow<Settings>("connect_steam");
}
export async function disconnectSteam(): Promise<Settings> {
  return await callOrThrow<Settings>("disconnect_steam");
}
export async function connectGog(): Promise<Settings> {
  return await callOrThrow<Settings>("connect_gog");
}
export async function disconnectGog(): Promise<Settings> {
  return await callOrThrow<Settings>("disconnect_gog");
}
export async function connectEpic(): Promise<Settings> {
  return await callOrThrow<Settings>("connect_epic");
}
export async function disconnectEpic(): Promise<Settings> {
  return await callOrThrow<Settings>("disconnect_epic");
}
export async function connectEa(): Promise<Settings> {
  return await callOrThrow<Settings>("connect_ea");
}
export async function disconnectEa(): Promise<Settings> {
  return await callOrThrow<Settings>("disconnect_ea");
}
export async function connectBattlenet(): Promise<Settings> {
  return await callOrThrow<Settings>("connect_battlenet");
}
export async function disconnectBattlenet(): Promise<Settings> {
  return await callOrThrow<Settings>("disconnect_battlenet");
}

/** Enregistre la clé API Steam (chemin avancé, chaîne vide = effacement). */
export async function setSteamKey(key: string): Promise<Settings | null> {
  return await call<Settings | null>("set_steam_key", { key }, null);
}

/** Renvoie l'état des connexions de comptes. */
export async function getSettings(): Promise<Settings | null> {
  return await call<Settings | null>("get_settings", undefined, null);
}

// --- Métadonnées ---------------------------------------------------------------

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
    const all = await call<MetaUpdate[]>("enrich_igdb", undefined, []);
    onBatch(all); // filet : réapplique le total (fusion idempotente).
  } finally {
    if (unlisten) unlisten();
  }
}

/**
 * Enrichit un seul jeu à la demande (ouverture du détail) : description,
 * captures, développeur, année, genre. Résultat mis en cache côté Rust.
 * Renvoie `null` hors Tauri.
 */
export async function enrichGame(game: Game): Promise<GameMeta | null> {
  return await call<GameMeta | null>(
    "enrich_game",
    {
      id: game.id,
      platform: game.platform,
      launchTarget: game.launchTarget ?? "",
      title: game.title,
      installed: game.installed,
    },
    null,
  );
}

// --- Service social (comptes, amis, présence) -----------------------------------
//
// Ces ponts NE masquent PAS les erreurs, contrairement au reste du fichier : le message
// du serveur (« Code incorrect. », « Aucun compte ne correspond à ce code. ») est écrit
// pour être montré tel quel à l'utilisateur. Les avaler laisserait un formulaire muet.

async function social<T>(cmd: string, args?: Args): Promise<T> {
  const invoke = await loadInvoke();
  if (!invoke) throw new Error("Le service Torii n'est disponible que dans l'application.");
  return await invoke<T>(cmd, args);
}

/**
 * Demande un code de connexion. Renvoie le code lui-même quand le serveur tourne en
 * mode développement — dans ce cas aucun e-mail ne part, et l'interface l'affiche.
 */
export async function toriiRequestCode(email: string): Promise<string | null> {
  return (await social<string | null>("torii_request_code", { email })) ?? null;
}

/** Vérifie le code et ouvre la session (persistée chiffrée côté Rust). */
export async function toriiVerify(email: string, code: string): Promise<ToriiSignIn> {
  return await social<ToriiSignIn>("torii_verify", { email, code });
}

/** Compte connecté, ou `null` (session absente, révoquée, ou hors application). */
export async function toriiMe(): Promise<ToriiAccount | null> {
  return await call<ToriiAccount | null>("torii_me", undefined, null);
}

export async function toriiLogout(): Promise<void> {
  await social<void>("torii_logout");
}

export async function toriiSetProfile(patch: {
  displayName?: string;
  steamId?: string | null;
  steamDiscoverable?: boolean;
}): Promise<ToriiAccount> {
  return await social<ToriiAccount>("torii_set_profile", patch);
}

export async function toriiCircle(): Promise<ToriiCircle> {
  return await social<ToriiCircle>("torii_circle");
}

export async function toriiInvite(friendCode: string): Promise<void> {
  await social<void>("torii_invite", { friendCode });
}

/** Invite une personne trouvée par suggestion : on a son identifiant, pas son code. */
export async function toriiInviteAccount(accountId: string): Promise<void> {
  await social<void>("torii_invite_account", { accountId });
}

export async function toriiRespond(accountId: string, accept: boolean): Promise<void> {
  await social<void>("torii_respond", { accountId, accept });
}

export async function toriiRemoveFriend(accountId: string): Promise<void> {
  await social<void>("torii_remove_friend", { accountId });
}

/** Régénère son code d'ami ; l'ancien cesse aussitôt de fonctionner. */
export async function toriiRotateCode(): Promise<string> {
  return await social<string>("torii_rotate_code");
}

/** Amis Steam déjà sur Torii (les deux comptes doivent être découvrables). */
export async function toriiSuggestions(steamIds: string[]): Promise<ToriiPerson[]> {
  return await social<ToriiPerson[]>("torii_suggestions", { steamIds });
}

export async function toriiPrefs(): Promise<SocialPrefs> {
  return await call<SocialPrefs>("torii_prefs", undefined, {
    sharePresence: false,
    awayAfterMinutes: 10,
  });
}

export async function toriiSetPrefs(prefs: SocialPrefs): Promise<SocialPrefs> {
  return await social<SocialPrefs>("torii_set_prefs", { prefs });
}

/** Jeux qu'on ne diffuse jamais (applications permanentes, jeux qu'on garde pour soi). */
export async function toriiMutedGames(): Promise<string[]> {
  return await call<string[]>("torii_muted_games", undefined, []);
}

export async function toriiMuteGame(id: string, muted: boolean): Promise<string[]> {
  return await social<string[]>("torii_mute_game", { id, muted });
}

/**
 * Cercle poussé par le battement de cœur (toutes les 30 s) : publier sa présence
 * renvoie celle des amis, donc l'interface se met à jour sans rien demander.
 */
export async function onToriiCircle(cb: (circle: ToriiCircle) => void): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<ToriiCircle>("torii-circle", (e) => cb(e.payload));
  } catch {
    return () => {};
  }
}

// --- Bibliothèque ---------------------------------------------------------------

/**
 * Scan complet : fichiers locaux + comptes en ligne. Lent (réseau), c'est la source
 * de vérité. `null` hors Tauri → l'appelant retombe sur les données fictives.
 */
export async function scanLibrary(): Promise<GameDto[] | null> {
  return await call<GameDto[] | null>("scan_library", undefined, null);
}

/**
 * Bibliothèque du dernier scan, relue du disque : instantanée et sans réseau. Sert à
 * remplir l'écran au lancement pendant que le vrai scan tourne. Vide au premier
 * lancement (et hors Tauri).
 */
export async function cachedLibrary(): Promise<GameDto[]> {
  return await call<GameDto[]>("cached_library", undefined, []);
}

// --- Bibliothèque (masqués, favoris, jeux manuels) ------------------------------

/** Masque ou réaffiche un jeu (liste d'exclusion). Renvoie les ids masqués. */
export async function setGameHidden(id: string, hidden: boolean): Promise<string[]> {
  return await call<string[]>("set_game_hidden", { id, hidden }, []);
}

/** Épingle ou retire un jeu des favoris. Renvoie les ids favoris. */
export async function setGameFavorite(id: string, favorite: boolean): Promise<string[]> {
  return await call<string[]>("set_game_favorite", { id, favorite }, []);
}

/**
 * Ajoute un jeu saisi manuellement (persisté dans `manual_games.json`).
 * Renvoie la liste à jour des jeux manuels (ou `null` hors Tauri).
 */
export async function addManualGame(input: ManualInput): Promise<GameDto[] | null> {
  return await call<GameDto[] | null>("add_manual_game", { input }, null);
}

/** Met à jour un jeu manuel existant. Renvoie la liste à jour (ou `null` hors Tauri). */
export async function updateManualGame(
  id: string,
  input: ManualInput,
): Promise<GameDto[] | null> {
  return await call<GameDto[] | null>("update_manual_game", { id, input }, null);
}

/** Retire un jeu manuel par son id. Renvoie la liste à jour (ou `null` hors Tauri). */
export async function removeManualGame(id: string): Promise<GameDto[] | null> {
  return await call<GameDto[] | null>("remove_manual_game", { id }, null);
}

// --- Sélecteurs de fichiers natifs ----------------------------------------------

interface PickFilter {
  name: string;
  extensions: string[];
}

/**
 * Ouvre l'explorateur Windows pour choisir un fichier. Renvoie le chemin, ou `null`
 * si l'utilisateur annule (et hors Tauri, où aucun dialogue natif n'existe).
 */
export async function pickFile(title: string, filters?: PickFilter[]): Promise<string | null> {
  if (!hasTauriRuntime()) return null;
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ title, multiple: false, directory: false, filters });
    return typeof res === "string" ? res : null;
  } catch {
    return null;
  }
}

/** Comme [`pickFile`] mais pour choisir un dossier. */
export async function pickFolder(title: string): Promise<string | null> {
  if (!hasTauriRuntime()) return null;
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ title, multiple: false, directory: true });
    return typeof res === "string" ? res : null;
  } catch {
    return null;
  }
}

/**
 * Rend affichable une jaquette choisie sur le disque : la webview ne peut pas charger
 * un `C:\…` brut, il lui faut une URL du protocole `asset` (activé dans `tauri.conf.json`).
 * Les URL http(s)/data déjà utilisables sont renvoyées telles quelles.
 */
export function displayableCover(url: string): string {
  if (/^(https?|data|blob|asset):/i.test(url)) return url;
  if (!hasTauriRuntime()) return url;
  try {
    // Import statique impossible (le module lit `window.__TAURI_INTERNALS__` à l'appel).
    const internals = (window as unknown as {
      __TAURI_INTERNALS__: { convertFileSrc(p: string, protocol?: string): string };
    }).__TAURI_INTERNALS__;
    return internals.convertFileSrc(url);
  } catch {
    return url;
  }
}

// --- Boutique -------------------------------------------------------------------

/**
 * Boutique — vitrine : une page de jeux mis en avant / en promo, selon le tri
 * (`featured`, `savings`, `price`, `recent`, `rating`). Renvoie `null` hors Tauri.
 */
export async function storeDeals(
  page: number,
  sort: string,
): Promise<StoreItem[] | null> {
  return await call<StoreItem[] | null>("store_deals", { page, sort }, null);
}

/** Boutique — recherche de jeux par titre. Renvoie `null` hors Tauri. */
export async function storeSearch(query: string): Promise<StoreItem[] | null> {
  return await call<StoreItem[] | null>("store_search", { query }, null);
}

/** Boutique — suggestions d'autocomplétion (léger). `null` hors Tauri. */
export async function storeSuggest(query: string): Promise<StoreSuggestion[] | null> {
  return await call<StoreSuggestion[] | null>("store_suggest", { query }, null);
}

/** Boutique — fiche produit (comparatif de prix + méta IGDB). `null` hors Tauri. */
export async function storeGame(gameId: string): Promise<StoreGame | null> {
  return await call<StoreGame | null>("store_game", { gameId }, null);
}

/** Renvoie les boutiques masquées (revendeurs exclus), persistées côté Rust. `null` hors Tauri. */
export async function getExcludedStores(): Promise<string[] | null> {
  return await call<string[] | null>("get_excluded_stores", undefined, null);
}

/** Masque ou réaffiche une boutique (revendeur). Renvoie les noms exclus à jour. */
export async function setStoreExcluded(name: string, excluded: boolean): Promise<string[] | null> {
  return await call<string[] | null>("set_store_excluded", { name, excluded }, null);
}

/** Réaffiche toutes les boutiques (vide la liste d'exclusion). Renvoie la liste vide. */
export async function clearExcludedStores(): Promise<string[] | null> {
  return await call<string[] | null>("clear_excluded_stores", undefined, null);
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

// --- Wishlist -------------------------------------------------------------------

/** Wishlist unifiée (Steam native ∪ Torii) enrichie de prix. `null` hors Tauri (→ mock). */
export async function wishlistAll(): Promise<WishlistItem[] | null> {
  return await call<WishlistItem[] | null>("wishlist_all", undefined, null);
}

/** Ids (ITAD) présents dans la wishlist Torii (état des boutons ♥). [] hors Tauri. */
export async function wishlistIds(): Promise<string[]> {
  return await call<string[]>("wishlist_ids", undefined, []);
}

/**
 * Ajoute un jeu à la wishlist Torii (et à Steam en bonus si le jeu y existe).
 * Renvoie `true` si le push vers Steam a réussi. No-op → false hors Tauri.
 */
export async function wishlistAdd(id: string, title: string, coverUrl?: string | null): Promise<boolean> {
  return await call<boolean>("wishlist_add", { id, title, coverUrl: coverUrl ?? null }, false);
}

/** Retire un jeu de la wishlist Torii (et de Steam si applicable). */
export async function wishlistRemove(id: string): Promise<void> {
  await call<void>("wishlist_remove", { id }, undefined);
}

// --- Steam (amis, profil, succès) -----------------------------------------------

/** Liste d'amis Steam + présence (vide si Steam non connecté). `null` hors Tauri. */
export async function steamFriends(): Promise<Friend[] | null> {
  return await call<Friend[] | null>("steam_friends", undefined, null);
}

/** Jeux en commun avec les amis Steam. `null` hors Tauri (→ mock). */
export async function friendsCommon(force = false): Promise<FriendsCommon | null> {
  return await call<FriendsCommon | null>("friends_common", { force }, null);
}

/**
 * Profil Steam de l'utilisateur (pseudo + avatar). `null` si non connecté ou en cas
 * d'échec ; hors Tauri (preview), profil fictif pour la maquette.
 */
export async function steamMe(): Promise<SteamProfile | null> {
  if (!(await inTauri())) return MOCK_PROFILE;
  return await call<SteamProfile | null>("steam_me", undefined, null);
}

/**
 * Succès Steam d'un jeu (`appid`). `null` pour un jeu non-Steam, sans succès, ou si
 * Steam n'est pas connecté. Hors Tauri (preview) : jeu de succès fictif pour la maquette.
 */
export async function steamAchievements(appid: string): Promise<SteamAchievements | null> {
  if (!(await inTauri())) return mockAchievements();
  return await call<SteamAchievements | null>("steam_achievements", { appid: Number(appid) }, null);
}

/**
 * Nombre de joueurs en ce moment sur un jeu Steam (`appid`), via l'API publique.
 * `null` si indisponible ; valeur fictive hors Tauri (preview).
 */
export async function steamCurrentPlayers(appid: string): Promise<number | null> {
  if (!(await inTauri())) return 40356; // mock preview
  return await call<number | null>("steam_current_players", { appid: Number(appid) }, null);
}

// --- Application (fenêtre, caches, mises à jour) ---------------------------------

/** Affiche une notification système (no-op hors Tauri). */
export async function notify(title: string, body: string): Promise<void> {
  await call<void>("notify_user", { title, body }, undefined);
}

/**
 * Vide les caches de métadonnées/jaquettes/prix. Renvoie le nombre de fichiers
 * supprimés, ou null hors Tauri.
 */
export async function clearCaches(): Promise<number | null> {
  return await call<number | null>("clear_caches", undefined, null);
}

/** Préférences de fenêtre (démarrage minimisé, fermeture dans le tray). */
export interface WindowPrefs {
  startMinimized: boolean;
  closeToTray: boolean;
}

export async function getWindowPrefs(): Promise<WindowPrefs> {
  return await call<WindowPrefs>("get_window_prefs", undefined, {
    startMinimized: false,
    closeToTray: false,
  });
}

export async function setWindowPrefs(prefs: WindowPrefs): Promise<void> {
  await call<void>(
    "set_window_prefs",
    { startMinimized: prefs.startMinimized, closeToTray: prefs.closeToTray },
    undefined,
  );
}

/** Indique si Torii démarre automatiquement avec Windows (false hors Tauri). */
export async function getAutostart(): Promise<boolean> {
  return await call<boolean>("get_autostart", undefined, false);
}

/**
 * Active/désactive le démarrage automatique avec Windows. Renvoie l'état effectif
 * (hors Tauri : renvoie la valeur demandée, sans effet).
 */
export async function setAutostart(enabled: boolean): Promise<boolean> {
  return await call<boolean>("set_autostart", { enabled }, enabled);
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

// --- Données fictives (preview web hors Tauri) -----------------------------------

const MOCK_PROFILE: SteamProfile = {
  steamId: "0",
  name: "PomPoteau",
  avatarUrl: "https://avatars.fastly.steamstatic.com/3604ac34b47c87e187d151f22aa17e107253ce34_full.jpg",
  profileUrl: "#",
};

function mockAchievements(): SteamAchievements {
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
