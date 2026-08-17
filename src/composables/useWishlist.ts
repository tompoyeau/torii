import { computed, ref } from "vue";
import { getSettings, wishlistAll } from "../lib/tauri";
import type { WishlistItem } from "../types";

// État partagé (singleton).
const items = ref<WishlistItem[]>([]);
const loading = ref(false);
const loaded = ref(false);
const steamConnected = ref(false);

let reqToken = 0;

/** Charge (ou recharge) la wishlist enrichie de prix. Source unique : Steam. */
async function refresh() {
  loading.value = true;
  const token = ++reqToken;
  const [list, settings] = await Promise.all([wishlistAll(), getSettings()]);
  if (token !== reqToken) return; // un rafraîchissement plus récent a pris le relais
  if (settings) steamConnected.value = settings.steamConnected;
  if (list) {
    items.value = list;
  } else {
    // Hors Tauri (preview) : données fictives pour la maquette.
    items.value = MOCK;
    steamConnected.value = true;
  }
  loaded.value = true;
  loading.value = false;
}

/** Retire un jeu de la liste affichée sans appel réseau (maj optimiste par id ITAD). */
function removeLocal(gameId: string) {
  if (!gameId) return;
  items.value = items.value.filter((i) => i.gameId !== gameId);
}

/**
 * Ajoute un jeu à la liste affichée sans appel réseau : item minimal (titre + jaquette),
 * prix « à venir » jusqu'au prochain rafraîchissement manuel / relance. Évite de rappeler
 * l'enrichissement ITAD (coûteux, rate-limité) à chaque ajout.
 */
function addLocal(item: { gameId: string; title: string; coverUrl?: string | null }) {
  if (!item.gameId || items.value.some((i) => i.gameId === item.gameId)) return;
  items.value = [
    {
      appId: 0,
      gameId: item.gameId,
      title: item.title,
      coverUrl: item.coverUrl ?? "",
      price: null,
      normalPrice: null,
      savings: 0,
      storeName: "",
      buyUrl: "",
      historyLow: null,
    },
    ...items.value,
  ];
}

export function useWishlist() {
  const onSaleCount = computed(() => items.value.filter((i) => i.savings > 0).length);
  return { items, loading, loaded, steamConnected, onSaleCount, refresh, removeLocal, addLocal };
}

// --- Données fictives (preview web hors Tauri) ---
function steamCover(appid: number): string {
  return `https://cdn.cloudflare.steamstatic.com/steam/apps/${appid}/library_600x900.jpg`;
}
const MOCK: WishlistItem[] = [
  { appId: 1245620, gameId: "w1", title: "Elden Ring", coverUrl: steamCover(1245620), price: 34.99, normalPrice: 59.99, savings: 42, storeName: "GreenManGaming", buyUrl: "#", historyLow: 29.99 },
  { appId: 1086940, gameId: "w2", title: "Baldur's Gate 3", coverUrl: steamCover(1086940), price: 44.99, normalPrice: 59.99, savings: 25, storeName: "Steam", buyUrl: "#", historyLow: 44.99 },
  { appId: 1091500, gameId: "w3", title: "Cyberpunk 2077", coverUrl: steamCover(1091500), price: 17.99, normalPrice: 59.99, savings: 70, storeName: "GOG", buyUrl: "#", historyLow: 14.99 },
  { appId: 292030, gameId: "w4", title: "The Witcher 3: Wild Hunt", coverUrl: steamCover(292030), price: 7.99, normalPrice: 39.99, savings: 80, storeName: "GOG", buyUrl: "#", historyLow: 6.79 },
  { appId: 3008130, gameId: "", title: "Star Wars Zero Company", coverUrl: steamCover(3008130), price: null, normalPrice: null, savings: 0, storeName: "", buyUrl: "", historyLow: null },
];
