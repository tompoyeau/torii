import { computed } from "vue";
import { useFriends } from "./useFriends";
import { useLibrary } from "./useLibrary";
import { useTorii } from "./useTorii";
import type { Friend, Game, ToriiFriend, ToriiStatus } from "../types";

/**
 * Liste d'amis unifiée : **une seule ligne par personne**, alimentée par deux sources
 * complémentaires.
 *
 * 🔑 Aucune des deux ne remplace l'autre :
 *   * **Steam** sait qui est en ligne même quand Torii est fermé, mais ne voit que les
 *     jeux Steam ;
 *   * **Torii** voit les jeux de tous les launchers, mais seulement pendant que la
 *     personne a Torii ouvert.
 *
 * Une personne présente des deux côtés (SteamID commun) apparaît donc une fois, avec
 * l'information la plus riche des deux — et son avatar Steam, que Torii n'a pas.
 */

/** Une personne, quelle que soit sa provenance. */
export interface UnifiedFriend {
  key: string;
  name: string;
  avatarUrl: string;
  state: ToriiStatus;
  /** Jeu en cours, s'il est connu. */
  gameName: string | null;
  /** Début de la partie (Unix) — connu seulement via Torii. */
  since: number | null;
  source: "steam" | "torii" | "both";
  /** Page de profil Steam, quand la personne vient de là. */
  profileUrl: string | null;
  /** Identifiant Torii, pour retirer l'ami ou répondre à sa demande. */
  toriiId: string | null;
  /**
   * Ce jeu dans TA bibliothèque, si tu l'as aussi — quel que soit le launcher.
   * C'est ce que la clé cross-launcher permet enfin de dire.
   */
  ownedGame: Game | null;
}

/**
 * Même normalisation que `social::game_key()` côté Rust : minuscules, lettres et chiffres
 * seulement. Les deux doivent rester d'accord, sinon « il joue au même jeu que toi » ne se
 * déclenche jamais.
 */
function gameKeyOf(title: string): string {
  return `title:${title.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "")}`;
}

/** Plus le rang est petit, plus la personne est « présente ». */
const RANK: Record<string, number> = { "in-game": 0, online: 1, away: 2, offline: 3 };
function rank(state: string): number {
  return RANK[state] ?? 3;
}

/** Les états Steam qui n'existent pas côté Torii sont ramenés aux quatre communs. */
function normalizeSteamState(state: string): ToriiStatus {
  switch (state) {
    case "in-game":
      return "in-game";
    case "offline":
      return "offline";
    case "away":
    case "snooze":
      return "away";
    default:
      return "online"; // online, busy…
  }
}

function fromSteam(f: Friend): UnifiedFriend {
  return {
    key: `steam:${f.steamId}`,
    name: f.name,
    avatarUrl: f.avatarUrl,
    state: normalizeSteamState(f.state),
    gameName: f.gameName ?? null,
    since: null,
    source: "steam",
    profileUrl: f.profileUrl,
    toriiId: null,
    ownedGame: null,
  };
}

function fromTorii(f: ToriiFriend): UnifiedFriend {
  return {
    key: `torii:${f.id}`,
    name: f.displayName,
    avatarUrl: "",
    state: f.status,
    gameName: f.gameTitle ?? null,
    since: f.since ?? null,
    source: "torii",
    profileUrl: null,
    toriiId: f.id,
    ownedGame: null,
  };
}

/**
 * Fusionne les deux fiches d'une même personne. On garde l'état le plus « présent »,
 * et le jeu de la source qui le connaît — Torii d'abord, puisque lui voit les jeux
 * hors Steam.
 */
function merge(steam: UnifiedFriend, torii: UnifiedFriend): UnifiedFriend {
  const best = rank(torii.state) <= rank(steam.state) ? torii : steam;
  return {
    ...best,
    key: torii.key,
    // Le nom Steam est celui que les gens reconnaissent ; l'avatar n'existe que là.
    name: steam.name || torii.name,
    avatarUrl: steam.avatarUrl,
    gameName: torii.gameName ?? steam.gameName,
    since: torii.since,
    source: "both",
    profileUrl: steam.profileUrl,
    toriiId: torii.toriiId,
  };
}

export function useFriendList() {
  const { friends: steamFriends, loading: steamLoading, steamConnected } = useFriends();
  const { circle, connected: toriiConnected, loading: toriiLoading } = useTorii();
  const { games } = useLibrary();

  /** Ma bibliothèque indexée par clé de jeu, pour reconnaître le jeu d'un ami. */
  const mesJeux = computed(() => {
    const map = new Map<string, Game>();
    for (const g of games.value) {
      if (g.hidden) continue;
      const cle = gameKeyOf(g.title);
      // À titre égal, on garde le jeu installé : c'est celui qu'on peut lancer.
      const connu = map.get(cle);
      if (!connu || (!connu.installed && g.installed)) map.set(cle, g);
    }
    return map;
  });

  const all = computed<UnifiedFriend[]>(() => {
    const steamByIdentity = new Map<string, UnifiedFriend>();
    for (const f of steamFriends.value) steamByIdentity.set(f.steamId, fromSteam(f));

    const merged: UnifiedFriend[] = [];
    for (const t of circle.value.friends) {
      const torii = fromTorii(t);
      // Le SteamID d'un ami n'arrive que s'il s'est rendu découvrable ; sans lui, on
      // ne peut pas savoir qu'il s'agit de la même personne, et on affiche deux lignes.
      const twin = t.steamId ? steamByIdentity.get(t.steamId) : undefined;
      if (twin) {
        steamByIdentity.delete(t.steamId!);
        merged.push(merge(twin, torii));
      } else {
        merged.push(torii);
      }
    }
    merged.push(...steamByIdentity.values());

    // « Tu l'as aussi » : on rapproche le jeu de l'ami de la bibliothèque locale. Pour un
    // ami Steam on n'a que le titre — on le normalise donc de la même façon.
    for (const f of merged) {
      if (f.state !== "in-game") continue;
      f.ownedGame = f.gameName ? mesJeux.value.get(gameKeyOf(f.gameName)) ?? null : null;
    }

    return merged.sort(
      (a, b) => rank(a.state) - rank(b.state) || a.name.localeCompare(b.name, "fr"),
    );
  });

  const inGame = computed(() => all.value.filter((f) => f.state === "in-game"));
  const online = computed(() => all.value.filter((f) => f.state === "online" || f.state === "away"));
  const offline = computed(() => all.value.filter((f) => f.state === "offline"));
  const activeCount = computed(() => all.value.filter((f) => f.state !== "offline").length);

  return {
    all,
    inGame,
    online,
    offline,
    activeCount,
    loading: computed(() => steamLoading.value || toriiLoading.value),
    steamConnected,
    toriiConnected,
  };
}
