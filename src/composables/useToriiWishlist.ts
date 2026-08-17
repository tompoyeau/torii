import { ref } from "vue";
import { wishlistAdd, wishlistIds, wishlistRemove } from "../lib/tauri";
import { showToast } from "./useToast";
import { useWishlist } from "./useWishlist";

/**
 * État de la wishlist Torii (universelle) pour les boutons « ♥ » de la Boutique.
 * On garde l'ensemble des ids ITAD wishlistés ; l'ajout/retrait est optimiste et
 * pousse en bonus vers Steam côté backend quand le jeu y existe.
 */
const ids = ref<Set<string>>(new Set());
let loaded = false;

async function ensureLoaded() {
  if (loaded) return;
  loaded = true;
  ids.value = new Set(await wishlistIds());
}

function isWishlisted(id: string): boolean {
  return ids.value.has(id);
}

async function toggle(item: { gameId: string; title: string; coverUrl?: string | null }) {
  const id = item.gameId;
  if (!id) return;
  // La page Wishlist (liste enrichie de prix) reflète l'ajout/retrait tout de suite,
  // en local — SANS rappeler l'enrichissement ITAD (coûteux et rate-limité, cf. 429).
  const { removeLocal, addLocal } = useWishlist();
  const next = new Set(ids.value);
  if (next.has(id)) {
    next.delete(id);
    ids.value = next;
    showToast(`« ${item.title} » retiré de la wishlist`);
    removeLocal(id); // disparition immédiate de la grille
    await wishlistRemove(id);
  } else {
    next.add(id);
    ids.value = next;
    showToast(`« ${item.title} » ajouté à la wishlist`);
    addLocal(item); // apparition immédiate (prix « à venir » jusqu'au prochain refresh)
    await wishlistAdd(id, item.title, item.coverUrl ?? null);
  }
}

export function useToriiWishlist() {
  void ensureLoaded();
  return { ids, isWishlisted, toggle };
}
