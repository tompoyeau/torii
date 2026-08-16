import { ref } from "vue";
import { wishlistAdd, wishlistIds, wishlistRemove } from "../lib/tauri";
import { showToast } from "./useToast";

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
  const next = new Set(ids.value);
  if (next.has(id)) {
    next.delete(id);
    ids.value = next;
    showToast(`« ${item.title} » retiré de la wishlist`);
    await wishlistRemove(id);
  } else {
    next.add(id);
    ids.value = next;
    showToast(`« ${item.title} » ajouté à la wishlist`);
    await wishlistAdd(id, item.title, item.coverUrl ?? null);
  }
}

export function useToriiWishlist() {
  void ensureLoaded();
  return { ids, isWishlisted, toggle };
}
