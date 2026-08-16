import { computed, ref } from "vue";
import { fetchGames, fromDto, mergeDuplicates } from "../data/games";
import { relativeTime } from "../lib/covers";
import {
  addManualGame,
  enrichGame,
  enrichIgdb,
  installGame,
  launchGame,
  recordLaunch,
  removeManualGame,
  setGameFavorite,
  setGameHidden,
  startGameWatch,
  type ManualInput,
} from "../lib/tauri";
import { usePreferences } from "./usePreferences";
import type { Game, LibraryFilter } from "../types";

const { prefs } = usePreferences();

const games = ref<Game[]>([]);
const loading = ref(false);
const enriching = ref(false);
const enrichProgress = ref<{ done: number; total: number } | null>(null);
const enrichingId = ref<string | null>(null);
let started = false;

// Jeux déjà enrichis (ou en cours) dans cette session : on ne rappelle pas l'API.
const enrichedIds = new Set<string>();

/**
 * Enrichit un jeu à la demande (ouverture de sa vue détail) : description,
 * captures, développeur, année, genre. N'écrase jamais une donnée déjà présente,
 * ne s'exécute qu'une fois par jeu, et met à jour le store de façon réactive.
 */
async function ensureEnriched(id: string) {
  if (enrichedIds.has(id)) return;
  const game = games.value.find((g) => g.id === id);
  if (!game) return;
  enrichedIds.add(id);

  enrichingId.value = id;
  const meta = await enrichGame(game);
  if (enrichingId.value === id) enrichingId.value = null;
  if (!meta) return;

  const idx = games.value.findIndex((g) => g.id === id);
  if (idx === -1) return;
  const cur = games.value[idx];
  const shots = meta.screenshots ?? undefined;
  games.value[idx] = {
    ...cur,
    // Résout les titres provisoires « App <id> » des jeux possédés.
    title: cur.title.startsWith("App ") && meta.name ? meta.name : cur.title,
    genre: cur.genre ?? meta.genre ?? undefined,
    description: cur.description ?? meta.description ?? undefined,
    developer: cur.developer ?? meta.developer ?? undefined,
    year: cur.year ?? meta.year ?? undefined,
    coverUrl: cur.coverUrl ?? meta.coverUrl ?? undefined,
    heroUrl: cur.heroUrl ?? meta.heroUrl ?? undefined,
    screenshots: cur.screenshots?.length ? cur.screenshots : shots,
    // Taille de téléchargement (jeu non installé) : 0/absent = à compléter.
    sizeGb: cur.sizeGb ? cur.sizeGb : meta.sizeGb ?? undefined,
  };
}

function load() {
  if (started) return;
  started = true;

  loading.value = true;
  fetchGames()
    .then((list) => {
      // Fusionne les doublons cross-plateforme en cartes uniques multi-sources.
      games.value = mergeDuplicates(list);
      // En tâche de fond : IGDB peuple toute la métadonnée descriptive.
      void fillIgdb();
    })
    .finally(() => (loading.value = false));
}

/**
 * Source unique de métadonnée descriptive : IGDB (cross-plateforme) peuple genre,
 * description, captures, hero, studio, année et complète les jaquettes manquantes.
 * Fusion réactive au fil des lots, SANS écraser ce que le launcher a déjà fourni
 * (jaquette native prioritaire, IGDB en repli — décision utilisateur).
 */
async function fillIgdb() {
  await enrichIgdb((updates) => {
    if (!updates.length) return;
    const byId = new Map(updates.map((u) => [u.id, u]));
    games.value = games.value.map((g) => {
      const u = byId.get(g.id);
      if (!u) return g;
      return {
        ...g,
        genre: g.genre ?? u.genre ?? undefined,
        description: g.description ?? u.description ?? undefined,
        developer: g.developer ?? u.developer ?? undefined,
        year: g.year ?? u.year ?? undefined,
        // Jaquette/hero : launcher d'abord, IGDB en repli.
        coverUrl: g.coverUrl ?? u.coverUrl ?? undefined,
        heroUrl: g.heroUrl ?? u.heroUrl ?? undefined,
        screenshots: g.screenshots?.length
          ? g.screenshots
          : u.screenshots?.length
            ? u.screenshots
            : undefined,
      };
    });
  });
}

/** Force un nouveau scan (ex: après connexion d'un compte). */
function reload() {
  started = false;
  // Les objets jeu sont recréés : on autorise un ré-enrichissement au besoin
  // (le cache disque Rust rend l'appel quasi instantané).
  enrichedIds.clear();
  load();
}

export function useLibrary() {
  load();

  const byId = (id: string | null) =>
    id ? games.value.find((g) => g.id === id) ?? null : null;

  /** Jeux mis en avant : les récents en priorité, complétés par le reste, triés du
   *  dernier joué au plus ancien (le 1er = le vrai dernier jeu joué). */
  const spotlight = computed(() => {
    const visible = games.value.filter((g) => !g.hidden);
    const recent = visible.filter((g) => g.recent);
    const pool = recent.length ? recent : visible;
    return [...pool]
      .sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0))
      .slice(0, 5);
  });

  function matches(game: Game, filter: LibraryFilter): boolean {
    // La vue « Masqués » ne montre QUE les masqués ; toutes les autres les cachent.
    if (filter === "hidden") return !!game.hidden;
    if (game.hidden) return false;
    switch (filter) {
      case "all": return true;
      case "mine": return !game.familyShared;
      case "family": return !!game.familyShared;
      case "recent": return game.recent;
      case "favorite": return game.favorite;
      case "installed": return game.installed;
      // Un jeu fusionné apparaît sous chacune de ses plateformes.
      default:
        return game.sources
          ? game.sources.some((s) => s.platform === filter)
          : game.platform === filter;
    }
  }

  /** Masque ou réaffiche un jeu ; met à jour le store et persiste côté Rust. */
  async function setHidden(id: string, hidden: boolean) {
    const idx = games.value.findIndex((g) => g.id === id);
    if (idx !== -1) games.value[idx] = { ...games.value[idx], hidden };
    await setGameHidden(id, hidden);
  }

  /** Épingle ou retire un jeu des favoris ; met à jour le store et persiste côté Rust. */
  async function setFavorite(id: string, favorite: boolean) {
    const idx = games.value.findIndex((g) => g.id === id);
    if (idx !== -1) games.value[idx] = { ...games.value[idx], favorite };
    await setGameFavorite(id, favorite);
  }

  /**
   * Ajoute un jeu saisi manuellement, le persiste côté Rust et l'insère dans le
   * store sans re-scanner toute la bibliothèque. Renvoie le jeu créé (ou null).
   */
  async function addManual(input: ManualInput): Promise<Game | null> {
    const list = await addManualGame(input);
    if (!list) return null;
    // Le backend renvoie tous les jeux manuels ; on n'insère que les nouveaux.
    const known = new Set(games.value.map((g) => g.id));
    const added = list.filter((d) => !known.has(d.id)).map(fromDto);
    if (added.length) games.value = [...games.value, ...added];
    return added[added.length - 1] ?? null;
  }

  /** Retire un jeu manuel (persisté côté Rust + retiré du store). */
  async function removeManual(id: string) {
    await removeManualGame(id);
    games.value = games.value.filter((g) => g.id !== id);
  }

  /**
   * Note le jeu comme joué « maintenant » (au clic sur Jouer) : met à jour le store
   * de façon optimiste (remonte dans « Récemment joué ») et persiste côté Rust. Fournit
   * une date de dernière session aux jeux sans stats de launcher.
   */
  /**
   * Action du bouton « Jouer / Installer » : lance le jeu s'il est installé (et note la
   * session), sinon déclenche son installation via le launcher — **sans** enregistrer de
   * session (installer n'est pas jouer). Centralisé pour tous les points de lancement.
   */
  function launchOrInstall(game: Game) {
    if (game.installed) {
      void markPlayed(game.id);
      void launchGame(game);
      // Suivi de session : minimise Torii puis rouvre la fiche à la fermeture du jeu.
      if (prefs.returnOnGameExit && game.installDir) {
        void startGameWatch(game.id, game.installDir);
      }
    } else {
      void installGame(game);
    }
  }

  async function markPlayed(id: string) {
    const now = Math.floor(Date.now() / 1000);
    const idx = games.value.findIndex((g) => g.id === id);
    if (idx !== -1) {
      games.value[idx] = {
        ...games.value[idx],
        lastPlayedAt: now,
        lastPlayed: relativeTime(now),
        recent: true,
      };
    }
    await recordLaunch(id);
  }

  function filtered(filter: LibraryFilter, query: string): Game[] {
    const q = query.trim().toLowerCase();
    return games.value.filter(
      (g) => matches(g, filter) && (!q || g.title.toLowerCase().includes(q)),
    );
  }

  return {
    games,
    loading,
    enriching,
    enrichProgress,
    enrichingId,
    byId,
    spotlight,
    matches,
    filtered,
    reload,
    ensureEnriched,
    setHidden,
    setFavorite,
    addManual,
    removeManual,
    markPlayed,
    launchOrInstall,
  };
}
