import { computed, ref } from "vue";
import { hasTauriRuntime, libraryOf } from "../lib/tauri";
import { useLibrary } from "./useLibrary";
import { useTorii } from "./useTorii";
import type { Game, LibGame, LibraryEntry } from "../types";

/**
 * La bibliothèque partagée d'un ami : ce qu'il **possède**, tous launchers confondus.
 *
 * C'est ce que ni Steam ni Discord ne savent dire — Steam ne connaît que Steam, et un ami
 * sur GOG ou Epic n'existe pour personne. La présence répondait déjà à « à quoi joue-t-il
 * maintenant » ; ici on répond à « qu'est-ce qu'il a ».
 *
 * ## Le croisement se fait ici, et pas ailleurs
 *
 * R2 ne sait pas chercher : le serveur rend des instantanés bruts, un par appareil. C'est
 * donc ce composable qui fait l'union des appareils d'une personne (deux PC = deux
 * bibliothèques différentes, la même personne) et qui recoupe avec la nôtre. À quelques
 * dizaines de Ko par ami, c'est indolore — et c'est exactement le compromis choisi en
 * mettant la charge utile dans R2 plutôt qu'en base.
 */

/**
 * Bibliothèques déjà téléchargées, par compte. Repartir du réseau à chaque aller-retour
 * entre la vue Amis et une bibliothèque serait gratuit en agacement seulement.
 *
 * 🔑 **Réactif et multi-comptes**, pas un simple cache de la vue courante : « En commun »
 * a besoin des bibliothèques de TOUS les amis à la fois pour croiser, pas seulement de
 * celle qu'on regarde.
 */
const libraries = ref<Record<string, LibGame[]>>({});
const loading = ref(false);
const error = ref<string | null>(null);
/** Compte actuellement chargé, pour ne pas réafficher la bibliothèque du précédent. */
const loadedId = ref<string | null>(null);

/**
 * Même normalisation que `social::game_key()` côté Rust et que `useFriendList` : minuscules,
 * lettres et chiffres seulement. Les trois doivent rester d'accord, sinon « tu l'as aussi »
 * ne se déclenche jamais.
 */
function gameKeyOf(title: string): string {
  return `title:${title.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "")}`;
}

/** Fusionne les appareils d'une personne : un jeu présent sur deux PC ne compte qu'une fois. */
function merge(snapshots: LibGame[][]): LibGame[] {
  const parCle = new Map<string, LibGame>();
  for (const liste of snapshots) {
    for (const jeu of liste) {
      const deja = parCle.get(jeu.key);
      if (!deja) {
        parCle.set(jeu.key, { ...jeu, platforms: [...jeu.platforms] });
        continue;
      }
      // Le même jeu sur deux machines : on cumule les launchers, et on récupère la
      // jaquette si l'un des deux appareils l'avait et pas l'autre.
      for (const p of jeu.platforms) if (!deja.platforms.includes(p)) deja.platforms.push(p);
      if (!deja.cover && jeu.cover) deja.cover = jeu.cover;
    }
  }
  return [...parCle.values()].sort((a, b) => a.title.localeCompare(b.title, "fr"));
}

/** Maquette pour le navigateur seul (hors Tauri), comme le fait `useFriendsCommon`. */
const MOCK: LibGame[] = [
  { key: "title:ashenkingdoms", title: "Ashen Kingdoms", platforms: ["steam"] },
  { key: "title:auroradrift", title: "Aurora Drift", platforms: ["epic", "steam"] },
  { key: "title:baldursgate3", title: "Baldur's Gate 3", platforms: ["gog"] },
  { key: "title:chronostatic", title: "Chrono Static", platforms: ["steam"] },
  { key: "title:duskrunner", title: "Dusk Runner", platforms: ["gog"] },
  { key: "title:emberfall", title: "Emberfall", platforms: ["epic"] },
  { key: "title:hollowknight", title: "Hollow Knight", platforms: ["gog", "steam"] },
  { key: "title:novaprotocol", title: "Nova Protocol", platforms: ["ubisoft"] },
  { key: "title:silentharbor", title: "Silent Harbor", platforms: ["steam"] },
  { key: "title:tidewalker", title: "Tidewalker", platforms: ["riot"] },
];

export function useFriendLibrary() {
  const { libraryIndex, refreshLibraryIndex } = useTorii();
  const { games: myGames } = useLibrary();

  /** Les appareils d'une personne (une bibliothèque par PC). */
  function devicesOf(accountId: string): LibraryEntry[] {
    return libraryIndex.value.friends.filter((e) => e.accountId === accountId);
  }

  /** Cette personne partage-t-elle quelque chose ? Sert à n'afficher le bouton que là où
      il mène quelque part. */
  function hasLibrary(accountId: string | null | undefined): boolean {
    return !!accountId && devicesOf(accountId).length > 0;
  }

  /**
   * Charge la bibliothèque d'un ami (tous ses appareils). Le cache évite de retélécharger
   * à chaque aller-retour ; `force` le contourne.
   */
  async function load(accountId: string, force = false) {
    loadedId.value = accountId;
    error.value = null;
    if (!force && libraries.value[accountId]) return;

    loading.value = true;
    try {
      await fetchInto(accountId, force);
      if (!libraries.value[accountId]?.length && !devicesOf(accountId).length) {
        error.value = "Cette personne ne partage pas sa bibliothèque.";
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  /** Télécharge et range la bibliothèque d'un compte. Sans état d'écran : sert aussi bien
      à la vue « bibliothèque d'un ami » qu'au chargement groupé d'« En commun ». */
  async function fetchInto(accountId: string, force = false) {
    if (!force && libraries.value[accountId]) return;
    if (!hasTauriRuntime()) {
      libraries.value = { ...libraries.value, [accountId]: MOCK };
      return;
    }
    if (!devicesOf(accountId).length) {
      // L'index est peut-être simplement périmé (l'ami vient d'activer le partage).
      await refreshLibraryIndex();
    }
    const snapshots = await Promise.all(
      devicesOf(accountId).map((d) =>
        libraryOf(accountId, d.deviceId)
          .then((s) => s.games)
          // Un appareil illisible (retiré entre-temps) ne doit pas vider toute la vue :
          // on garde ce que les autres donnent.
          .catch(() => [] as LibGame[]),
      ),
    );
    libraries.value = { ...libraries.value, [accountId]: merge(snapshots) };
  }

  /** Comptes dont on peut lire la bibliothèque (ceux qui partagent). */
  const sharers = computed(() => [...new Set(libraryIndex.value.friends.map((e) => e.accountId))]);

  /**
   * Charge les bibliothèques de TOUS les amis qui partagent — ce dont « En commun » a
   * besoin pour croiser. Tolérant : un ami illisible n'empêche pas les autres de compter.
   */
  async function loadAllShared() {
    await Promise.all(sharers.value.map((id) => fetchInto(id).catch(() => {})));
  }

  /** La bibliothèque actuellement consultée. */
  const games = computed<LibGame[]>(() =>
    loadedId.value ? libraries.value[loadedId.value] ?? [] : [],
  );

  /** Ma bibliothèque indexée par clé de jeu — c'est elle qui dit « tu l'as aussi ». */
  const mineByKey = computed(() => {
    const m = new Map<string, Game>();
    for (const g of myGames.value) {
      const k = gameKeyOf(g.title);
      if (!m.has(k)) m.set(k, g);
    }
    return m;
  });

  /** Le jeu correspondant dans MA bibliothèque, quel que soit le launcher. */
  function mine(jeu: LibGame): Game | null {
    return mineByKey.value.get(jeu.key) ?? null;
  }

  const commonCount = computed(() => games.value.filter((g) => !!mine(g)).length);

  return {
    /** Relit l'index (qui partage, et depuis quand). Silencieux : hors ligne, on garde
        simplement ce qu'on affichait. */
    refreshIndex: () => refreshLibraryIndex().catch(() => {}),
    libraries,
    sharers,
    loadAllShared,
    games,
    loading,
    error,
    loadedId,
    devicesOf,
    hasLibrary,
    load,
    mine,
    commonCount,
  };
}
