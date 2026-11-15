import { nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";

/**
 * Une fiche (jeu, Boutique) est-elle tout en haut ? Tant qu'elle l'est, la barre du haut
 * s'efface et la bannière occupe toute la fenêtre ; dès qu'on descend, la barre revient
 * par-dessus la fiche (`TopBar.vue`).
 *
 * Un état par fiche, car celle de la Boutique peut s'ouvrir par-dessus celle d'un jeu :
 * c'est la fiche du dessus qui décide.
 */

/** En dessous de ce défilement (px), on compte la fiche comme tout en haut. */
const SEUIL = 24;

const enHaut = { jeu: ref(true), produit: ref(true) };

export function useHautDeFiche() {
  return enHaut;
}

/**
 * Relève la position de la fiche à chaque défilement, et à chaque ouverture : le retour
 * d'historique peut la rouvrir déjà défilée (`data-defilement`, `useNavigation.ts`).
 */
export function suivreHautDeFiche(fiche: "jeu" | "produit", panneau: Ref<HTMLElement | null>, ouverte: Ref<boolean>) {
  const etat = enHaut[fiche];
  const relever = () => {
    etat.value = (panneau.value?.scrollTop ?? 0) < SEUIL;
  };
  onMounted(() => panneau.value?.addEventListener("scroll", relever, { passive: true }));
  onBeforeUnmount(() => panneau.value?.removeEventListener("scroll", relever));
  watch(ouverte, () => void nextTick(relever));
}
