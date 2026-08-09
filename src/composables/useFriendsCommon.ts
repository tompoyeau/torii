import { computed, ref } from "vue";
import { friendsCommon, getSettings } from "../lib/tauri";
import type { CommonGame, FriendLib, Game } from "../types";

// État partagé (singleton).
const friends = ref<FriendLib[]>([]);
const games = ref<CommonGame[]>([]);
const loading = ref(false);
const loaded = ref(false);
const steamConnected = ref(false);
/** SteamIDs des amis sélectionnés pour l'intersection. */
const selected = ref<Set<string>>(new Set());

let reqToken = 0;
let loadPromise: Promise<void> | null = null;

/** Clé « steam:appid » d'un jeu de bibliothèque (id direct ou source Steam fusionnée). */
function steamKeyOf(game: Game): string | null {
  if (game.id.startsWith("steam:")) return game.id;
  const s = game.sources?.find((s) => s.platform === "steam" && s.launchTarget);
  return s ? `steam:${s.launchTarget}` : null;
}

/** Charge (ou recharge en `force`) les jeux en commun. Source unique : Steam. */
async function refresh(force = false) {
  loading.value = true;
  const token = ++reqToken;
  const [data, settings] = await Promise.all([friendsCommon(force), getSettings()]);
  if (token !== reqToken) return; // un rafraîchissement plus récent a pris le relais
  if (settings) steamConnected.value = settings.steamConnected;
  if (data) {
    friends.value = data.friends;
    games.value = data.games;
  } else {
    // Hors Tauri (preview) : données fictives pour la maquette.
    friends.value = MOCK.friends;
    games.value = MOCK.games;
    steamConnected.value = true;
  }
  // Purge la sélection des amis disparus.
  const ids = new Set(friends.value.map((f) => f.steamId));
  selected.value = new Set([...selected.value].filter((id) => ids.has(id)));
  loaded.value = true;
  loading.value = false;
}

/** Charge une seule fois (cache 6 h côté Rust) — pour la fiche détail, sans forcer. */
function ensureLoaded(): Promise<void> {
  if (loaded.value) return Promise.resolve();
  if (!loadPromise) loadPromise = refresh().finally(() => (loadPromise = null));
  return loadPromise;
}

export function useFriendsCommon() {
  /** Amis dont la bibliothèque est lisible (sélectionnables). */
  const readable = computed(() => friends.value.filter((f) => !f.private));
  const privateCount = computed(() => friends.value.filter((f) => f.private).length);

  const friendById = computed(() => {
    const m = new Map<string, FriendLib>();
    for (const f of friends.value) m.set(f.steamId, f);
    return m;
  });

  /** Amis qui possèdent aussi ce jeu (par appid Steam). [] si aucun / jeu hors Steam. */
  function ownersOf(game: Game): FriendLib[] {
    const key = steamKeyOf(game);
    if (!key) return [];
    const cg = games.value.find((g) => g.id === key);
    if (!cg) return [];
    return cg.owners
      .map((id) => friendById.value.get(id))
      .filter((f): f is FriendLib => !!f);
  }

  /** Jeux affichés selon la sélection. */
  const shownGames = computed<CommonGame[]>(() => {
    const sel = selected.value;
    if (sel.size === 0) {
      // Aucun ami choisi : tous mes jeux partagés, les plus communs en tête (déjà triés backend).
      return games.value;
    }
    // Intersection : jeux que TOUS les amis sélectionnés (et moi) possèdent.
    return games.value
      .filter((g) => [...sel].every((id) => g.owners.includes(id)))
      .sort((a, b) => a.title.localeCompare(b.title, "fr"));
  });

  function toggleFriend(id: string) {
    const next = new Set(selected.value);
    next.has(id) ? next.delete(id) : next.add(id);
    selected.value = next;
  }
  function clearSelection() {
    selected.value = new Set();
  }
  function isSelected(id: string): boolean {
    return selected.value.has(id);
  }

  return {
    friends,
    readable,
    privateCount,
    games,
    shownGames,
    selected,
    loading,
    loaded,
    steamConnected,
    refresh,
    ensureLoaded,
    ownersOf,
    toggleFriend,
    clearSelection,
    isSelected,
  };
}

// --- Données fictives (preview web hors Tauri) ---
const MOCK: { friends: FriendLib[]; games: CommonGame[] } = {
  friends: [
    { steamId: "1", name: "Sterben", avatarUrl: "", private: false, commonCount: 3 },
    { steamId: "2", name: "Zouze", avatarUrl: "", private: false, commonCount: 2 },
    { steamId: "3", name: "therempard", avatarUrl: "", private: false, commonCount: 2 },
    { steamId: "4", name: "Benator", avatarUrl: "", private: true, commonCount: 0 },
  ],
  games: [
    { id: "steam:730", title: "Counter-Strike 2", coverUrl: null, owners: ["1", "2", "3"] },
    { id: "steam:945360", title: "Among Us", coverUrl: null, owners: ["1", "3"] },
    { id: "steam:1145360", title: "Hades", coverUrl: null, owners: ["1", "2"] },
    { id: "steam:271590", title: "GTA V", coverUrl: null, owners: ["2"] },
  ],
};
