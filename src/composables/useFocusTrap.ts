import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

/**
 * Enferme le focus clavier dans une surface modale, et le rend là où il était à la
 * fermeture.
 *
 * 🔑 Sans ça, une modale n'existe qu'à la souris : la tabulation continue de parcourir la
 * page **derrière** l'overlay, on se retrouve à activer des boutons qu'on ne voit pas, et
 * à la fermeture le focus repart au tout début du document. Avant ce composable, `.focus()`
 * n'apparaissait qu'UNE fois dans tout le front et `tabindex` pas une seule.
 *
 * Trois responsabilités, et il faut les trois :
 *   1. **entrer** — au premier rendu, le focus va sur le premier élément utile ;
 *   2. **tourner en rond** — Tab sur le dernier revient au premier, Maj+Tab l'inverse ;
 *   3. **rendre** — à la fermeture, le focus retourne à l'élément qui a ouvert la modale,
 *      sans quoi la personne recommence sa navigation depuis le haut de la page.
 *
 * ⚠️ Le conteneur doit porter `tabindex="-1"` : c'est le repli quand la modale ne contient
 * aucun élément focusable, et sans lui le focus retomberait sur `<body>`.
 *
 * L'écouteur est posé en **capture** sur le document : une modale peut contenir des champs
 * qui traitent eux-mêmes la touche Tab, et on veut décider avant eux.
 */
const FOCUSABLES = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

export function useFocusTrap(conteneur: Ref<HTMLElement | null>, ouvert: Ref<boolean>) {
  /** L'élément qui avait le focus avant l'ouverture — celui à qui on le rendra. */
  let rendreA: HTMLElement | null = null;

  function focusables(): HTMLElement[] {
    const racine = conteneur.value;
    if (!racine) return [];
    // `offsetParent === null` écarte ce qui est masqué (`display: none`), donc les
    // boutons d'un repli fermé : les inclure ferait buter la tabulation dans le vide.
    return [...racine.querySelectorAll<HTMLElement>(FOCUSABLES)].filter(
      (el) => el.offsetParent !== null,
    );
  }

  function surTab(e: KeyboardEvent) {
    if (e.key !== "Tab" || !ouvert.value || !conteneur.value) return;
    const liste = focusables();
    const actif = document.activeElement as HTMLElement | null;

    if (!liste.length) {
      e.preventDefault();
      conteneur.value.focus();
      return;
    }
    // Le focus s'est échappé (clic dans la page, élément retiré) : on le ramène.
    if (!actif || !conteneur.value.contains(actif)) {
      e.preventDefault();
      liste[0].focus();
      return;
    }
    const premier = liste[0];
    const dernier = liste[liste.length - 1];
    if (e.shiftKey && actif === premier) {
      e.preventDefault();
      dernier.focus();
    } else if (!e.shiftKey && actif === dernier) {
      e.preventDefault();
      premier.focus();
    }
  }

  function ouvrir() {
    rendreA = document.activeElement as HTMLElement | null;
    document.addEventListener("keydown", surTab, true);
    void nextTick(() => {
      (focusables()[0] ?? conteneur.value)?.focus();
    });
  }

  function fermer() {
    document.removeEventListener("keydown", surTab, true);
    // `isConnected` : l'élément d'origine a pu disparaître pendant que la modale était
    // ouverte (une carte retirée de la grille, par exemple). Lui rendre le focus ne
    // ferait rien du tout, et le focus resterait nulle part.
    if (rendreA?.isConnected) rendreA.focus();
    rendreA = null;
  }

  watch(ouvert, (o) => (o ? ouvrir() : fermer()), { immediate: true });
  onBeforeUnmount(fermer);
}
