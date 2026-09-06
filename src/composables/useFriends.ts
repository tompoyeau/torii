import { computed, ref } from "vue";
import { getSettings, steamFriends } from "../lib/tauri";
import { avatarFictif } from "../lib/covers";
import type { Friend } from "../types";

// État partagé (singleton).
const friends = ref<Friend[]>([]);
const loading = ref(false);
const loaded = ref(false);
const steamConnected = ref(false);

let reqToken = 0;

/** Rafraîchit la liste d'amis + la présence (source unique : Steam). */
async function refresh() {
  loading.value = true;
  const token = ++reqToken;
  const [list, settings] = await Promise.all([steamFriends(), getSettings()]);
  if (token !== reqToken) return; // un rafraîchissement plus récent a pris le relais
  if (settings) steamConnected.value = settings.steamConnected;
  // Hors Tauri (preview) : données fictives pour la maquette.
  friends.value = list ?? MOCK_FRIENDS;
  if (!list) steamConnected.value = true;
  loaded.value = true;
  loading.value = false;
}

/** Ordre de présence : en jeu, puis en ligne, puis hors ligne. */
function rank(state: string): number {
  if (state === "in-game") return 0;
  if (state === "offline") return 2;
  return 1;
}

export function useFriends() {
  // Groupes triés pour l'affichage.
  const inGame = computed(() => friends.value.filter((f) => f.state === "in-game"));
  const online = computed(() =>
    friends.value.filter((f) => f.state !== "in-game" && f.state !== "offline"),
  );
  const offline = computed(() => friends.value.filter((f) => f.state === "offline"));

  const sorted = computed(() =>
    [...friends.value].sort(
      (a, b) => rank(a.state) - rank(b.state) || a.name.localeCompare(b.name, "fr"),
    ),
  );

  /** Nombre d'amis actifs (en ligne ou en jeu). */
  const activeCount = computed(
    () => friends.value.filter((f) => f.state !== "offline").length,
  );

  return {
    friends,
    loading,
    loaded,
    steamConnected,
    inGame,
    online,
    offline,
    sorted,
    activeCount,
    refresh,
  };
}

// --- Données fictives (hors Tauri : preview) ---------------------------------

/**
 * Amis fictifs affiches hors Tauri.
 *
 * ⚠️ CETTE LISTE EST PUBLIEE. Elle alimente la demo en ligne du site : n'y mettre aucune
 * personne reelle. Elle a longtemps contenu les vrais pseudonymes des amis du
 * developpeur, ce qui revenait a publier les donnees personnelles de tiers sur une page
 * ouverte a tous. Pseudonymes inventes, avatars generes, et rien d'autre.
 *
 * 🔑 Les memes noms et les memes couleurs figurent sur les captures d'ecran du site :
 * une personne doit se ressembler d'un ecran a l'autre, sinon la demonstration sonne faux.
 */
const MOCK_FRIENDS: Friend[] = [
  { steamId: "d1", name: "Kobalt", avatarUrl: avatarFictif("K", "#6d4bd6", "#c0399a"), state: "in-game", gameName: "Sons Of The Forest", profileUrl: "#" },
  { steamId: "d2", name: "grizel", avatarUrl: avatarFictif("G", "#1f7a8c", "#3fd0c9"), state: "in-game", gameName: "HELLDIVERS 2", profileUrl: "#" },
  { steamId: "d3", name: "Marmotte", avatarUrl: avatarFictif("M", "#c25b2a", "#e8a33d"), state: "in-game", gameName: "Baldur's Gate 3", profileUrl: "#" },
  { steamId: "d4", name: "Ambre", avatarUrl: avatarFictif("A", "#3b3f8c", "#7f86e0"), state: "in-game", gameName: "Deep Rock Galactic", profileUrl: "#" },
  { steamId: "d5", name: "Orval2178", avatarUrl: avatarFictif("O", "#7a2f5f", "#d1568f"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d6", name: "Vantar.exe", avatarUrl: avatarFictif("V", "#25706b", "#57c1a8"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d7", name: "tibou", avatarUrl: avatarFictif("t", "#2f7a45", "#8fd15a"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d8", name: "loupio", avatarUrl: avatarFictif("L", "#2a5eb8", "#5ec8f0"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d9", name: "milo207", avatarUrl: avatarFictif("m", "#a8322f", "#e06a56"), state: "away", gameName: null, profileUrl: "#" },
  { steamId: "d10", name: "Grimald", avatarUrl: avatarFictif("G", "#2f7a45", "#8fd15a"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d11", name: "Poivrade", avatarUrl: avatarFictif("P", "#5c2f8c", "#9d6ae0"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d12", name: "Falk", avatarUrl: avatarFictif("F", "#8c6a1f", "#dfc154"), state: "online", gameName: null, profileUrl: "#" },
  { steamId: "d13", name: "Bigorneau", avatarUrl: avatarFictif("B", "#1f7a8c", "#3fd0c9"), state: "offline", gameName: null, profileUrl: "#" },
  { steamId: "d14", name: "Ocelot", avatarUrl: avatarFictif("O", "#c25b2a", "#e8a33d"), state: "offline", gameName: null, profileUrl: "#" },
  { steamId: "d15", name: "MrPistache", avatarUrl: avatarFictif("M", "#7a2f5f", "#d1568f"), state: "offline", gameName: null, profileUrl: "#" },
  { steamId: "d16", name: "leperchoir", avatarUrl: avatarFictif("L", "#8c3b1f", "#d98246"), state: "offline", gameName: null, profileUrl: "#" },
  { steamId: "d17", name: "Orphee", avatarUrl: avatarFictif("O", "#3b3f8c", "#7f86e0"), state: "offline", gameName: null, profileUrl: "#" },
  { steamId: "d18", name: "Pilou", avatarUrl: avatarFictif("P", "#a8322f", "#e06a56"), state: "offline", gameName: null, profileUrl: "#" },
];
