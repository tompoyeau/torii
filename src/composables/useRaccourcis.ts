import { nextTick } from "vue";
import { useLibrary } from "./useLibrary";
import { useNavigation } from "./useNavigation";
import { useUi } from "./useUi";
import { hasTauriRuntime } from "../lib/tauri";
import { t } from "../i18n";

/**
 * Raccourcis clavier de l'application.
 *
 * 🔑 ÉCOUTÉS SUR `window`, PAS SUR `document`. Les composants qui consomment une touche
 * pour eux-mêmes (Échap d'une modale, d'un menu, de la visionneuse) écoutent `document`
 * et appellent `preventDefault()` : à la remontée, `document` passe avant `window`, donc
 * ici on sait toujours si la touche a déjà servi — quel que soit l'ordre de montage.
 *
 * ⚠️ `listeRaccourcis()`, plus bas, est ce qu'affiche Paramètres > Raccourcis clavier :
 * un raccourci ajouté ici s'y ajoute aussi.
 */

const ui = useUi();
const nav = useNavigation();
/** Demandée à la première resynchronisation au clavier, pas au chargement du module. */
let bibliotheque: ReturnType<typeof useLibrary> | null = null;

/** Ctrl + chiffre : les grandes sections, dans l'ordre de la barre latérale. */
const SECTIONS: Record<string, () => void> = {
  "1": () => ui.showLibrary(),
  "2": () => ui.showStore(),
  "3": () => ui.showWishlist(),
  "4": () => ui.showFriends(),
};

function estUnChamp(cible: EventTarget | null): boolean {
  return cible instanceof HTMLElement && !!cible.closest("input, textarea, select, [contenteditable='true']");
}

/**
 * Le champ de recherche de l'écran courant. Chaque section a le sien (`data-recherche`),
 * et la barre du haut se masque là où elle ne filtrerait rien : on prend le premier qui
 * est affiché. Les fiches ne cachent pas la page en dessous, elles la recouvrent : son
 * champ compte comme affiché même fiche ouverte.
 */
function champDeRecherche(): HTMLInputElement | null {
  for (const el of document.querySelectorAll<HTMLInputElement>("input[data-recherche]")) {
    if (el.getClientRects().length) return el;
  }
  return null;
}

/**
 * Ctrl+F : referme fiches et Paramètres, et place le curseur dans la recherche. Un écran
 * sans recherche (profil d'un ami, wishlist vide) mène à celle de la bibliothèque — en
 * une seule étape, pour qu'un retour ramène là d'où l'on vient.
 */
async function allerALaRecherche() {
  const surPlace = champDeRecherche() != null;
  nav.quitterLesSurcouches();
  if (!surPlace) ui.showLibrary();
  await nextTick();
  const champ = champDeRecherche();
  champ?.focus();
  champ?.select();
}

function echap(e: KeyboardEvent) {
  // Le plein écran d'une vidéo : Échap en sort, et rien d'autre.
  if (document.fullscreenElement || nav.modaleOuverte()) return;
  const couche = nav.coucheDuDessus();
  if (couche) {
    e.preventDefault();
    nav.fermer(couche);
    return;
  }
  // Dans un champ de recherche : en sortir, pour que le clavier serve de nouveau à naviguer.
  const actif = document.activeElement;
  if (actif instanceof HTMLElement && actif.matches("[data-recherche]")) actif.blur();
}

function surTouche(e: KeyboardEvent) {
  if (e.defaultPrevented || e.isComposing) return;
  const ctrl = e.ctrlKey || e.metaKey;

  if (e.altKey && !ctrl && !e.shiftKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
    e.preventDefault();
    if (e.key === "ArrowLeft") nav.reculer();
    else nav.avancer();
    return;
  }
  if (e.key === "Escape") {
    echap(e);
    return;
  }
  if (e.key === "F1" || (e.key === "?" && !ctrl && !e.altKey && !estUnChamp(e.target))) {
    e.preventDefault();
    if (!nav.modaleOuverte()) ui.openSettings("raccourcis");
    return;
  }
  if (e.key === "F5" || (ctrl && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "r")) {
    // 🔑 Dans l'application, F5 rechargeait la page entière : Torii repartait de l'écran
    // de démarrage. On en fait « Resynchroniser ». La démo web garde le rechargement.
    if (!hasTauriRuntime()) return;
    e.preventDefault();
    bibliotheque ??= useLibrary();
    if (!bibliotheque.loading.value) bibliotheque.reload();
    return;
  }

  // Le reste se fait avec Ctrl. ⚠️ AltGr arrive comme Ctrl+Alt : on l'écarte, il sert à
  // taper @, # ou [ sur un clavier français.
  if (!ctrl || e.altKey) return;
  const touche = e.key.toLowerCase();
  // Le chiffre par sa position (`code`) : sur un clavier AZERTY, Ctrl+1 donne la touche « & ».
  const chiffre = /^(?:Digit|Numpad)([1-4])$/.exec(e.code)?.[1];
  if (touche === "f") {
    e.preventDefault();
    if (!nav.modaleOuverte()) void allerALaRecherche();
  } else if (touche === ",") {
    e.preventDefault();
    if (!nav.modaleOuverte()) ui.openSettings();
  } else if (touche === "n") {
    e.preventDefault();
    if (!nav.modaleOuverte()) ui.openAddGame();
  } else if (chiffre) {
    e.preventDefault();
    if (nav.modaleOuverte()) return;
    nav.quitterLesSurcouches();
    SECTIONS[chiffre]();
  }
}

/** Branche les raccourcis ; renvoie de quoi les débrancher. */
export function installerRaccourcis(): () => void {
  window.addEventListener("keydown", surTouche);
  return () => window.removeEventListener("keydown", surTouche);
}

/** Une ligne du tableau des raccourcis : l'action, et les combinaisons qui la déclenchent. */
export interface LigneRaccourci {
  action: string;
  /** Chaque combinaison est une suite de touches : `[["Ctrl", "F"]]`. */
  combinaisons: string[][];
  /** Les combinaisons vont par deux sens (← / →) au lieu d'être des équivalents (F5 ou Ctrl+R). */
  sens?: boolean;
}

/** Le tableau de Paramètres > Raccourcis clavier, par groupe. */
export function listeRaccourcis(): { titre: string; lignes: LigneRaccourci[] }[] {
  const ctrl = t("reglages.raccourcis.touches.ctrl");
  const alt = t("reglages.raccourcis.touches.alt");
  const echap = t("reglages.raccourcis.touches.echap");
  return [
    {
      titre: t("reglages.raccourcis.groupes.navigation"),
      lignes: [
        { action: t("reglages.raccourcis.actions.precedent"), combinaisons: [[alt, "←"], [t("reglages.raccourcis.touches.sourisRetour")]] },
        { action: t("reglages.raccourcis.actions.suivant"), combinaisons: [[alt, "→"], [t("reglages.raccourcis.touches.sourisAvant")]] },
        { action: t("reglages.raccourcis.actions.fermer"), combinaisons: [[echap]] },
        { action: t("reglages.raccourcis.actions.bibliotheque"), combinaisons: [[ctrl, "1"]] },
        { action: t("reglages.raccourcis.actions.boutique"), combinaisons: [[ctrl, "2"]] },
        { action: t("reglages.raccourcis.actions.wishlist"), combinaisons: [[ctrl, "3"]] },
        { action: t("reglages.raccourcis.actions.amis"), combinaisons: [[ctrl, "4"]] },
      ],
    },
    {
      titre: t("reglages.raccourcis.groupes.actions"),
      lignes: [
        { action: t("reglages.raccourcis.actions.rechercher"), combinaisons: [[ctrl, "F"]] },
        { action: t("reglages.raccourcis.actions.ajouter"), combinaisons: [[ctrl, "N"]] },
        { action: t("reglages.raccourcis.actions.resynchroniser"), combinaisons: [["F5"], [ctrl, "R"]] },
        { action: t("reglages.raccourcis.actions.parametres"), combinaisons: [[ctrl, ","]] },
        { action: t("reglages.raccourcis.actions.raccourcis"), combinaisons: [["F1"], ["?"]] },
      ],
    },
    {
      titre: t("reglages.raccourcis.groupes.medias"),
      lignes: [
        { action: t("reglages.raccourcis.actions.capture"), combinaisons: [["←"], ["→"]], sens: true },
        { action: t("reglages.raccourcis.actions.lecture"), combinaisons: [[t("reglages.raccourcis.touches.espace")], ["K"]] },
        { action: t("reglages.raccourcis.actions.avanceVideo"), combinaisons: [["←"], ["→"]], sens: true },
        { action: t("reglages.raccourcis.actions.volume"), combinaisons: [["↑"], ["↓"]], sens: true },
        { action: t("reglages.raccourcis.actions.muet"), combinaisons: [["M"]] },
        { action: t("reglages.raccourcis.actions.pleinEcran"), combinaisons: [["F"]] },
      ],
    },
  ];
}

/** « Paramètres (Ctrl+,) » : le nom d'un bouton suivi de son raccourci, pour les infobulles. */
export function avecRaccourci(action: string, ...touches: string[]): string {
  return t("reglages.raccourcis.infobulle", { action, touches: touches.join("+") });
}
