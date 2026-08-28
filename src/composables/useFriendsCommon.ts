import { computed, ref } from "vue";
import { friendsCommon, getSettings } from "../lib/tauri";
import { useFriendLibrary } from "./useFriendLibrary";
import { useLibrary } from "./useLibrary";
import { useTorii } from "./useTorii";
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
/**
 * Même normalisation que `social::game_key()` : c'est elle qui permet de reconnaître le
 * même jeu d'une source à l'autre — un ami Steam et un ami sur GOG, ou un de mes jeux
 * Epic et sa jumelle dans la bibliothèque Torii de quelqu'un.
 */
function keyOfTitle(title: string): string {
  return `title:${title.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "")}`;
}

/** Identifiant de propriétaire pour un ami Torii sans compte Steam rattaché. */
const toriiOwnerId = (accountId: string) => `torii:${accountId}`;

async function refresh(force = false) {
  loading.value = true;
  const token = ++reqToken;
  // 🔑 Les deux sources sont chargées ensemble : le croisement Steam (une commande Rust
  // qui interroge l'API Steam) et les bibliothèques Torii des amis qui partagent. La
  // seconde ne doit jamais faire échouer la première — Steam marche sans Torii, et
  // l'inverse aussi.
  const { sharers, loadAllShared } = useFriendLibrary();
  const [data, settings] = await Promise.all([
    friendsCommon(force),
    getSettings(),
    loadAllShared().catch(() => {}),
  ]);
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
  // Purge la sélection des amis disparus (les deux sources confondues : un ami Torii
  // qui cesse de partager doit sortir de la sélection comme un ami Steam disparu).
  const ids = new Set([
    ...friends.value.map((f) => f.steamId),
    ...sharers.value.map(toriiOwnerId),
  ]);
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
  const { games: myGames } = useLibrary();
  const { circle } = useTorii();
  const { libraries, sharers } = useFriendLibrary();

  /* ── Deuxième source : les bibliothèques Torii ─────────────────────────────
   *
   * Steam ne répond qu'à « lesquels de mes amis STEAM ont ce jeu STEAM ». Les
   * bibliothèques Torii élargissent la vue des deux côtés à la fois : un ami qui n'est
   * pas sur Steam peut désormais apparaître, et un de MES jeux GOG/Epic/manuel peut
   * enfin être « en commun » — ce qu'aucun autre launcher ne sait dire.
   */

  /** Mes jeux indexés par clé de titre (toutes plateformes, masqués exclus). */
  const myByKey = computed(() => {
    const m = new Map<string, Game>();
    for (const g of myGames.value) {
      if (g.hidden) continue;
      const k = keyOfTitle(g.title);
      if (!m.has(k)) m.set(k, g);
    }
    return m;
  });

  /** Le SteamID d'un ami Torii, s'il s'est rendu découvrable — c'est ce qui permet de ne
      pas le compter deux fois quand il est aussi dans la liste Steam. */
  const steamIdOfAccount = computed(() => {
    const m = new Map<string, string>();
    for (const f of circle.value.friends) if (f.steamId) m.set(f.id, f.steamId);
    return m;
  });

  /** Pseudo Torii d'un compte (pour les amis absents de la liste Steam). */
  const nameOfAccount = computed(() => {
    const m = new Map<string, string>();
    for (const f of circle.value.friends) m.set(f.id, f.displayName);
    return m;
  });

  /**
   * L'identité sous laquelle un ami Torii compte comme propriétaire : son SteamID s'il en
   * a un connu ET qu'il est déjà dans la liste Steam (une seule pastille pour une seule
   * personne), sinon un identifiant Torii.
   */
  function ownerIdOf(accountId: string): string {
    const steamId = steamIdOfAccount.value.get(accountId);
    if (steamId && friends.value.some((f) => f.steamId === steamId)) return steamId;
    return toriiOwnerId(accountId);
  }

  /** clé de jeu → propriétaires connus par Torii (limité à MES jeux : c'est la vue). */
  const toriiOwnersByKey = computed(() => {
    const m = new Map<string, string[]>();
    for (const accountId of sharers.value) {
      const liste = libraries.value[accountId];
      if (!liste) continue; // pas encore téléchargée
      const owner = ownerIdOf(accountId);
      for (const jeu of liste) {
        if (!myByKey.value.has(jeu.key)) continue;
        const deja = m.get(jeu.key);
        if (deja) {
          if (!deja.includes(owner)) deja.push(owner);
        } else {
          m.set(jeu.key, [owner]);
        }
      }
    }
    return m;
  });

  /** Amis Torii qui partagent et qui ne sont PAS déjà une ligne Steam. */
  const toriiOnlyFriends = computed<FriendLib[]>(() =>
    sharers.value
      .filter((id) => ownerIdOf(id).startsWith("torii:"))
      .map((id) => ({
        steamId: toriiOwnerId(id),
        name: nameOfAccount.value.get(id) ?? "Ami Torii",
        avatarUrl: "",
        // Une bibliothèque Torii partagée est lisible par construction : on ne la voit
        // que parce que la personne a activé le partage.
        private: false,
        commonCount: 0,
      })),
  );

  /** Les deux sources fusionnées : mes jeux, avec tous les amis qui les possèdent. */
  const allGames = computed<CommonGame[]>(() => {
    const parCle = new Map<string, CommonGame>();
    for (const cg of games.value) {
      parCle.set(keyOfTitle(cg.title), { ...cg, owners: [...cg.owners] });
    }
    for (const [cle, owners] of toriiOwnersByKey.value) {
      const existant = parCle.get(cle);
      if (existant) {
        for (const o of owners) if (!existant.owners.includes(o)) existant.owners.push(o);
        continue;
      }
      // Jeu absent du croisement Steam : soit il n'est pas sur Steam, soit aucun ami
      // Steam ne l'a. C'est MA fiche qui sert de base, la vue parle de mes jeux.
      const mien = myByKey.value.get(cle);
      if (!mien) continue;
      parCle.set(cle, {
        id: mien.id,
        title: mien.title,
        coverUrl: mien.coverUrl ?? null,
        owners: [...owners],
      });
    }
    return [...parCle.values()].sort(
      (a, b) => b.owners.length - a.owners.length || a.title.localeCompare(b.title, "fr"),
    );
  });

  /** Toutes les pastilles d'amis, avec un compte « en commun » recalculé sur les deux sources. */
  const allFriends = computed<FriendLib[]>(() => {
    const compte = new Map<string, number>();
    for (const g of allGames.value) {
      for (const o of g.owners) compte.set(o, (compte.get(o) ?? 0) + 1);
    }
    return [...friends.value, ...toriiOnlyFriends.value]
      .map((f) => ({ ...f, commonCount: compte.get(f.steamId) ?? f.commonCount }))
      .sort((a, b) => b.commonCount - a.commonCount || a.name.localeCompare(b.name, "fr"));
  });

  /** Amis dont la bibliothèque est lisible (sélectionnables). */
  const readable = computed(() => allFriends.value.filter((f) => !f.private));
  const privateCount = computed(() => allFriends.value.filter((f) => f.private).length);

  const friendById = computed(() => {
    const m = new Map<string, FriendLib>();
    for (const f of allFriends.value) m.set(f.steamId, f);
    return m;
  });

  /**
   * Amis qui possèdent aussi ce jeu. Le croisement se fait par **clé de titre** et non par
   * appid : sans ça, un jeu GOG ou Epic de ma bibliothèque n'aurait jamais de propriétaire,
   * alors que les bibliothèques Torii savent désormais le dire.
   */
  function ownersOf(game: Game): FriendLib[] {
    const cg = allGames.value.find((g) => g.id === steamKeyOf(game) || g.id === game.id
      || keyOfTitle(g.title) === keyOfTitle(game.title));
    if (!cg) return [];
    return cg.owners
      .map((id) => friendById.value.get(id))
      .filter((f): f is FriendLib => !!f);
  }

  /** Jeux affichés selon la sélection. */
  const shownGames = computed<CommonGame[]>(() => {
    const sel = selected.value;
    // Aucun ami choisi : tous mes jeux partagés, les plus communs en tête.
    if (sel.size === 0) return allGames.value;
    // Intersection : jeux que TOUS les amis sélectionnés (et moi) possèdent.
    return allGames.value
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
    friends: allFriends,
    readable,
    privateCount,
    games: allGames,
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
