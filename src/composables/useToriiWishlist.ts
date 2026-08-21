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
/**
 * Basculements faits par l'utilisateur avant que la liste du disque ne soit revenue.
 * Sans ça, un clic sur ♥ dans la première seconde était écrasé par `ensureLoaded`
 * (qui lisait un état antérieur à l'ajout) et le cœur se re-vidait tout seul.
 */
const pending = new Map<string, boolean>();

async function ensureLoaded() {
  if (loaded) return;
  loaded = true;
  const disk = new Set(await wishlistIds());
  for (const [id, on] of pending) {
    if (on) disk.add(id);
    else disk.delete(id);
  }
  ids.value = disk;
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
    pending.set(id, false);
    removeLocal(id); // disparition immédiate de la grille
    await wishlistRemove(id);
    showToast(`« ${item.title} » retiré de ta wishlist`);
  } else {
    next.add(id);
    ids.value = next;
    pending.set(id, true);
    addLocal(item); // apparition immédiate (prix « à venir » jusqu'au prochain refresh)
    // Le retour dit si le jeu a AUSSI été poussé vers la vraie wishlist Steam : autant
    // le dire, plutôt qu'un « ajouté » qui laissait croire que Steam avait suivi.
    const onSteam = await wishlistAdd(id, item.title, item.coverUrl ?? null);
    showToast(
      onSteam
        ? `« ${item.title} » ajouté à ta wishlist, Steam compris`
        : `« ${item.title} » ajouté à ta wishlist`,
    );
  }
}

export function useToriiWishlist() {
  void ensureLoaded();
  return { ids, isWishlisted, toggle };
}
