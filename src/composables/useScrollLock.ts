import { onBeforeUnmount, watch, type Ref } from "vue";

/**
 * Fige le défilement de la page tant qu'une surcouche plein écran est ouverte.
 *
 * Les fiches (jeu, produit Boutique) et les pop-in sont en `position: fixed` avec leur
 * propre défilement : la vue en dessous restait scrollable et sa barre de défilement
 * restait visible à droite — on pouvait faire défiler la bibliothèque en lisant la fiche
 * d'un jeu, ce qui n'a aucun sens.
 *
 * Le compteur permet d'empiler les surcouches (fiche produit ouverte par-dessus une fiche
 * de jeu) : le défilement n'est rendu qu'à la fermeture de la dernière.
 */
let locks = 0;
let savedPadding = "";

function apply() {
  const body = document.body;
  if (locks === 1) {
    // Masquer la barre élargit la page : on compense par une marge de sa largeur exacte,
    // sinon le contenu sursaute à chaque ouverture/fermeture.
    const gap = window.innerWidth - document.documentElement.clientWidth;
    savedPadding = body.style.paddingRight;
    if (gap > 0) body.style.paddingRight = `${gap}px`;
    body.style.overflow = "hidden";
  } else if (locks === 0) {
    body.style.overflow = "";
    body.style.paddingRight = savedPadding;
  }
}

export function useScrollLock(active: Ref<boolean>) {
  let held = false;
  const set = (on: boolean) => {
    if (on === held) return;
    held = on;
    locks += on ? 1 : -1;
    apply();
  };
  watch(active, set, { immediate: true });
  // Un composant démonté alors qu'il tenait le verrou le laisserait posé à jamais.
  onBeforeUnmount(() => set(false));
}
