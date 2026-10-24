import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useUi, type MainSection, type SettingsCategory } from "./useUi";
import { useStore } from "./useStore";
import { useStoreCategories } from "./useStoreCategories";
import { useLibrary } from "./useLibrary";
import { useFriendList, type UnifiedFriend } from "./useFriendList";
import { titreFiltre } from "../data/platforms";
import { t } from "../i18n";
import type { LibraryFilter } from "../types";

/**
 * Historique de navigation, comme celui d'un navigateur : une pile « précédent », une
 * pile « suivant ». Le bouton retour de la souris, Alt+←, les flèches de la barre du
 * haut et le bouton retour des fiches reculent d'une étape ; leurs jumeaux avancent.
 *
 * 🔑 UNE ÉTAPE = TOUT CE QUI SE VOIT : la section (et l'ami consulté), le filtre de la
 * bibliothèque, la fiche d'un jeu, la fiche Boutique, la pop-in Paramètres et la page
 * web intégrée. La fiche
 * Boutique manquait à l'ancien historique (elle vit dans `useStore`, pas dans `useUi`) :
 * ouvrir un jeu similaire remplaçait la fiche sans rien retenir, et le retour refermait
 * tout d'un coup — on retombait sur la vitrine au lieu du premier jeu ouvert.
 *
 * 🔑 LES ÉTAPES S'ENREGISTRENT TOUTES SEULES. Un `watch` sur ces états suffit : aucun
 * bouton n'a à penser à l'historique. Ne passent par ce module que les gestes qui
 * DÉPLACENT dans l'historique au lieu d'y ajouter (précédent, suivant, fermer une fiche).
 *
 * 🔑 FERMER N'EST PAS AJOUTER. Fermer une fiche ou les Paramètres (Échap, croix, clic à
 * côté) revient à l'étape d'avant leur ouverture. Enregistrer la fermeture comme une
 * nouvelle étape faisait rouvrir la fiche au retour suivant, au lieu de reculer.
 */

/** Ce qu'Échap referme, du plus haut au plus bas : les surcouches, puis la page web. */
export type Couche = "parametres" | "produit" | "jeu" | "page";

/** Ce qui défile : la page, et chacune des deux fiches (qui ont leur propre défilement). */
type Zone = "page" | "jeu" | "produit";

interface Etape {
  section: MainSection;
  filter: LibraryFilter;
  friendProfileKey: string | null;
  friendLibraryId: string | null;
  /** Fiche d'un jeu de la bibliothèque. */
  jeu: string | null;
  /** Fiche Boutique, par-dessus la section ou par-dessus la fiche d'un jeu. */
  produit: string | null;
  /**
   * Catégories cochées dans la Boutique. Un tag cliqué dans une fiche ouvre la Boutique
   * sur sa catégorie : le retour doit ramener à la fiche, puis aux catégories d'avant.
   */
  boutique: number[];
  parametres: boolean;
  categorie: SettingsCategory;
  /**
   * Page web intégrée (section `web`) : la dernière adresse où l'on était. Les liens
   * suivis DANS la page la mettent à jour sans faire d'étape — c'est l'historique de la
   * page elle-même qui les retient — et revenir à cette étape rouvre la page là.
   */
  page: string | null;
  /**
   * Où on en était, tenu à jour à chaque défilement tant que l'étape est affichée.
   * Revenir au premier jeu doit ramener en bas de sa fiche, là où on avait cliqué le
   * jeu similaire — pas en haut de la page.
   */
  defilement: Record<Zone, number>;
}

/** Valeur provisoire de la fiche Boutique pendant que « Voir dans la boutique » cherche le jeu. */
const CHARGEMENT = "__loading__";
/** Au-delà, les étapes les plus anciennes sont oubliées. */
const LIMITE = 60;

const ui = useUi();
const store = useStore();
const categoriesBoutique = useStoreCategories();

function capturer(precedente?: Etape): Etape {
  const jeu = ui.selectedGameId.value;
  const produit = store.selectedGameId.value;
  return {
    section: ui.section.value,
    filter: ui.filter.value,
    friendProfileKey: ui.friendProfileKey.value,
    friendLibraryId: ui.friendLibraryId.value,
    jeu,
    produit,
    boutique: [...categoriesBoutique.choisis.value],
    parametres: ui.settingsOpen.value,
    categorie: ui.settingsCategory.value,
    page: ui.pageWeb.value,
    // Une fiche qui reste la même garde son défilement ; une autre fiche repart du haut.
    defilement: {
      page: window.scrollY,
      jeu: precedente && precedente.jeu === jeu ? precedente.defilement.jeu : 0,
      produit: precedente && precedente.produit === produit ? precedente.defilement.produit : 0,
    },
  };
}

/** Même écran ? (Ni la catégorie des Paramètres ni l'adresse de la page web n'en
    décident : voir le `watch`.) */
function memeEtape(a: Etape, b: Etape): boolean {
  return (
    a.section === b.section && a.filter === b.filter &&
    a.friendProfileKey === b.friendProfileKey && a.friendLibraryId === b.friendLibraryId &&
    a.jeu === b.jeu && a.produit === b.produit && a.parametres === b.parametres &&
    a.boutique.join(",") === b.boutique.join(",")
  );
}

let courante = capturer();
const arriere: Etape[] = [];
const avant: Etape[] = [];
/** Les piles sont de simples tableaux : ce compteur prévient l'interface qu'elles ont bougé. */
const version = ref(0);
/** Vrai pendant qu'on rejoue une étape : ce changement-là n'en crée pas une nouvelle. */
let restauration = false;

watch(
  () => [
    ui.section.value, ui.filter.value, ui.friendProfileKey.value, ui.friendLibraryId.value,
    ui.selectedGameId.value, store.selectedGameId.value, ui.settingsOpen.value, ui.settingsCategory.value,
    ui.pageWeb.value, categoriesBoutique.choisis.value,
  ],
  () => {
    if (restauration) return;
    // La fiche en chargement n'est pas une étape : on attend le jeu trouvé (ou la
    // recherche de repli), qui en fera une à partir de l'écran d'où l'on vient.
    if (store.selectedGameId.value === CHARGEMENT) return;
    const nouvelle = capturer(courante);
    if (memeEtape(nouvelle, courante)) {
      // Un onglet des Paramètres remplace l'étape au lieu de s'y ajouter : le retour
      // referme la pop-in, il ne rejoue pas chaque onglet visité. Idem pour un lien
      // suivi dans la page web : son propre historique le retient déjà.
      courante.categorie = nouvelle.categorie;
      courante.page = nouvelle.page;
      return;
    }
    arriere.push(courante);
    if (arriere.length > LIMITE) arriere.shift();
    avant.length = 0;
    courante = nouvelle;
    version.value++;
  },
);

// --- Défilement ---------------------------------------------------------------
// Les deux fiches portent `data-defilement="jeu|produit"` : c'est ainsi qu'on les
// reconnaît ici sans dépendre de leurs composants.

document.addEventListener(
  "scroll",
  (e) => {
    const cible = e.target;
    if (cible === document) courante.defilement.page = window.scrollY;
    else if (cible instanceof HTMLElement && cible.dataset.defilement) {
      courante.defilement[cible.dataset.defilement as Zone] = cible.scrollTop;
    }
  },
  { capture: true, passive: true },
);

/**
 * Un geste de l'utilisateur interrompt une restauration en cours : on ne lui dispute
 * pas la page s'il s'est déjà mis à la faire défiler.
 */
let gestes = 0;
for (const type of ["wheel", "pointerdown", "keydown", "touchstart"]) {
  window.addEventListener(type, () => gestes++, { capture: true, passive: true });
}

/**
 * Remet chaque zone à sa position. Une page dont le contenu arrive encore (fiche
 * rechargée, vitrine qui se remplit) n'est pas assez haute du premier coup : on
 * réessaie quelques images, le temps qu'elle grandisse.
 */
function restaurerDefilement(cible: Record<Zone, number>) {
  const depart = gestes;
  let essais = 0;
  const viser = () => {
    if (gestes !== depart) return;
    let atteint = true;
    if (Math.abs(window.scrollY - cible.page) > 1) {
      window.scrollTo({ top: cible.page });
      atteint = Math.abs(window.scrollY - cible.page) <= 1;
    }
    for (const zone of ["jeu", "produit"] as const) {
      const el = document.querySelector<HTMLElement>(`[data-defilement="${zone}"]`);
      if (!el || Math.abs(el.scrollTop - cible[zone]) <= 1) continue;
      el.scrollTop = cible[zone];
      if (Math.abs(el.scrollTop - cible[zone]) > 1) atteint = false;
    }
    if (!atteint && ++essais < 40) requestAnimationFrame(viser);
  };
  viser();
}

// --- Déplacements -------------------------------------------------------------

/** Redessine une étape de l'historique. */
function appliquer(e: Etape) {
  // Copiée tout de suite : les fiches remettent leur défilement à zéro en changeant de
  // jeu, et cet événement arrive avant qu'on ait pu restaurer la position retenue.
  const cible = { ...e.defilement };
  restauration = true;
  courante = e;
  ui.section.value = e.section;
  ui.filter.value = e.filter;
  ui.friendProfileKey.value = e.friendProfileKey;
  ui.friendLibraryId.value = e.friendLibraryId;
  ui.selectedGameId.value = e.jeu;
  ui.settingsOpen.value = e.parametres;
  ui.settingsCategory.value = e.categorie;
  ui.pageWeb.value = e.page;
  if (e.boutique.join(",") !== categoriesBoutique.choisis.value.join(",")) {
    categoriesBoutique.choisis.value = [...e.boutique];
  }
  if (e.produit !== store.selectedGameId.value) {
    // Rouvrir la fiche, et pas seulement poser son identifiant : c'est `openProduct` qui
    // la charge (depuis le cache des fiches vues, la plupart du temps).
    if (e.produit) void store.openProduct(e.produit);
    else store.closeProduct();
  }
  version.value++;
  void nextTick(() => {
    restauration = false;
    restaurerDefilement(cible);
  });
}

/** Recule (n < 0) ou avance (n > 0) de |n| étapes. */
function deplacer(n: number): boolean {
  const depuis = n < 0 ? arriere : avant;
  const vers = n < 0 ? avant : arriere;
  if (!n || !depuis.length) return false;
  let cible = courante;
  for (let k = 0; k < Math.abs(n) && depuis.length; k++) {
    vers.push(cible);
    cible = depuis.pop()!;
  }
  appliquer(cible);
  return true;
}

// --- Fenêtres modales ---------------------------------------------------------

/**
 * Ce qui se ferme au retour sans être une étape : la modale d'ajout d'un jeu, la
 * connexion Torii, le menu contextuel, la visionneuse de captures. Le retour les ferme
 * d'abord ; suivant et les raccourcis de navigation attendent qu'elles le soient.
 */
interface Modale {
  ouverte: () => boolean;
  fermer: () => void;
}
const modales: Modale[] = [{ ouverte: () => ui.addGameOpen.value, fermer: () => ui.closeAddGame() }];

function modaleOuverte(): Modale | null {
  for (let i = modales.length - 1; i >= 0; i--) if (modales[i].ouverte()) return modales[i];
  return null;
}

/** Déclare une modale d'un composant, le temps qu'il est monté. */
export function useModaleDeNavigation(modale: Modale) {
  onMounted(() => modales.push(modale));
  onBeforeUnmount(() => {
    const i = modales.indexOf(modale);
    if (i >= 0) modales.splice(i, 1);
  });
}

// --- Gestes -------------------------------------------------------------------

/**
 * L'historique de la page web intégrée (`usePageWeb`). Sur cette page, précédent et
 * suivant passent d'abord par lui, comme dans un navigateur : chaque fonction renvoie
 * `false` quand la page est au bout de son propre historique.
 */
interface HistoriqueInterne {
  reculer: () => boolean;
  avancer: () => boolean;
}
let historiqueInterne: HistoriqueInterne | null = null;
export function brancherHistoriqueInterne(h: HistoriqueInterne) {
  historiqueInterne = h;
}

function reculer(): boolean {
  const modale = modaleOuverte();
  if (modale) {
    modale.fermer();
    return true;
  }
  if (ui.section.value === "web" && historiqueInterne?.reculer()) return true;
  return deplacer(-1);
}

function avancer(): boolean {
  if (modaleOuverte()) return false;
  if (ui.section.value === "web" && historiqueInterne?.avancer()) return true;
  return deplacer(1);
}

const estOuverte: Record<Couche, (e: Etape) => boolean> = {
  parametres: (e) => e.parametres,
  produit: (e) => e.produit != null,
  jeu: (e) => e.jeu != null,
  page: (e) => e.section === "web",
};

/**
 * Ferme une surcouche : revient à l'étape d'avant son ouverture. De la fiche B, ouverte
 * depuis la fiche A, elle-même ouverte depuis la vitrine, on revient à la vitrine — B et
 * A restent accessibles par « suivant ».
 */
function fermer(couche: Couche) {
  if (couche === "produit" && store.selectedGameId.value === CHARGEMENT) {
    store.closeProduct();
    return;
  }
  if (!estOuverte[couche](courante)) return;
  for (let i = arriere.length - 1; i >= 0; i--) {
    if (!estOuverte[couche](arriere[i])) {
      deplacer(i - arriere.length);
      return;
    }
  }
  // Ouverte avant tout historique (au démarrage, ou oubliée au-delà de la limite) : on
  // la referme simplement, ce qui fait une étape.
  if (couche === "parametres") ui.closeSettings();
  else if (couche === "produit") store.closeProduct();
  else if (couche === "jeu") ui.closeGame();
  else ui.showLibrary();
}

/** La surcouche du dessus, celle qu'Échap referme. */
function coucheDuDessus(): Couche | null {
  if (ui.settingsOpen.value) return "parametres";
  if (store.selectedGameId.value != null) return "produit";
  if (ui.selectedGameId.value != null) return "jeu";
  if (ui.section.value === "web") return "page";
  return null;
}

/**
 * Referme fiches et Paramètres d'un coup, avant d'aller ailleurs au clavier. C'est UNE
 * étape avec la destination : le retour ramène sur la fiche qu'on lisait.
 */
function quitterLesSurcouches() {
  ui.closeGame();
  store.closeProduct();
  ui.closeSettings();
}

// --- Libellés -----------------------------------------------------------------
// Le bouton retour des fiches et l'infobulle des flèches disent où l'on va.

/** Titres des fiches Boutique vues : une étape ne garde que l'identifiant du jeu. */
const titresProduits = reactive(new Map<string, string>());
/** Titres des pages web, par adresse (envoyés par la page, voir `usePageWeb`). */
const titresPages = reactive(new Map<string, string>());

/** Retient le titre d'une page web, pour nommer l'étape dans le bouton retour. */
export function noterTitrePage(url: string, titre: string) {
  if (url && titre) titresPages.set(url, titre);
}

/** Le nom d'un site, à défaut de titre : `store.steampowered.com` sans le `www.`. */
function domaine(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
watch(store.product, (p) => {
  const id = store.selectedGameId.value;
  if (p && id && id !== CHARGEMENT) titresProduits.set(id, p.title);
});

// Appelés au premier libellé seulement : ce module est chargé avant que l'application
// ne démarre, et la bibliothèque n'a pas à commencer son chargement plus tôt pour lui.
let bibliotheque: ReturnType<typeof useLibrary> | null = null;
let amis: ReturnType<typeof useFriendList> | null = null;

function nomAmi(trouve: (f: UnifiedFriend) => boolean): string {
  amis ??= useFriendList();
  const liste = [...amis.inGame.value, ...amis.online.value, ...amis.offline.value];
  return liste.find(trouve)?.name ?? t("amis.profil.cetAmi");
}

function libelle(e: Etape): string {
  if (e.parametres) return t("reglages.titre");
  if (e.produit) return titresProduits.get(e.produit) ?? t("boutique.produit.fiche");
  if (e.jeu) return (bibliotheque ??= useLibrary()).byId(e.jeu)?.title ?? t("fiche.fiche");
  switch (e.section) {
    case "web": return e.page ? titresPages.get(e.page) ?? domaine(e.page) : t("page.titre");
    case "store":
      // « Revenir à : Coop + Survie » plutôt qu'un « Boutique » qui ne dirait pas lesquelles.
      return e.boutique.length
        ? e.boutique.map((id) => categoriesBoutique.noms.get(id) ?? `#${id}`).join(" + ")
        : t("bibliotheque.laterale.boutique");
    case "wishlist": return t("bibliotheque.laterale.wishlist");
    case "common": return t("bibliotheque.laterale.enCommun");
    case "friends": return t("amis.titre");
    case "friendProfile": return nomAmi((f) => f.key === e.friendProfileKey);
    case "friendLibrary":
      return t("amis.bibliotheque.titre", { nom: nomAmi((f) => f.toriiId === e.friendLibraryId) });
    default: return titreFiltre(e.filter);
  }
}

const precedente = computed(() => {
  void version.value;
  return arriere[arriere.length - 1] ?? null;
});
const suivante = computed(() => {
  void version.value;
  return avant[avant.length - 1] ?? null;
});

export function useNavigation() {
  return {
    peutReculer: computed(() => precedente.value != null),
    peutAvancer: computed(() => suivante.value != null),
    /** Où mène « précédent », ou null s'il n'y a rien avant. */
    libellePrecedent: computed(() => (precedente.value ? libelle(precedente.value) : null)),
    libelleSuivant: computed(() => (suivante.value ? libelle(suivante.value) : null)),
    reculer,
    avancer,
    fermer,
    coucheDuDessus,
    modaleOuverte: () => modaleOuverte() != null,
    quitterLesSurcouches,
  };
}
