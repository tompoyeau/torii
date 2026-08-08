import { computed, ref } from "vue";
import { getSettings, steamFriends } from "../lib/tauri";
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

const MOCK_FRIENDS: Friend[] = [
  { steamId: "1", name: "Sterben", avatarUrl: "", state: "in-game", gameName: "Baldur's Gate 3", profileUrl: "#" },
  { steamId: "2", name: "Ecrevisse", avatarUrl: "", state: "in-game", gameName: "Counter-Strike 2", profileUrl: "#" },
  { steamId: "3", name: "Illusionnelle", avatarUrl: "", state: "online", gameName: null, profileUrl: "#" },
  { steamId: "4", name: "Jordan", avatarUrl: "", state: "away", gameName: null, profileUrl: "#" },
  { steamId: "5", name: "loginn", avatarUrl: "", state: "online", gameName: null, profileUrl: "#" },
  { steamId: "6", name: "Marceline", avatarUrl: "", state: "offline", gameName: null, profileUrl: "#" },
  { steamId: "7", name: "Nowé", avatarUrl: "", state: "offline", gameName: null, profileUrl: "#" },
];
