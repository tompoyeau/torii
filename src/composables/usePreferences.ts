import { reactive, watch } from "vue";
import type { SortKey } from "../types";
import { appliquerLangue, langueDuSysteme, langueParDefaut, type Langue } from "../i18n";
import { appliquerRegion, REGIONS, regionDuSysteme, type CodeRegion } from "../i18n/regions";
import { choixLocaleRetenu, langueHeritee, setLocale } from "../lib/tauri";

/** Densité de la grille de bibliothèque (taille des jaquettes). */
export type Density = "compact" | "normal" | "large";
/** Filtre de départ proposé dans les préférences (sous-ensemble simple). */
export type DefaultFilter = "all" | "recent" | "favorite" | "installed";

interface Prefs {
  /** Filtre de bibliothèque sélectionné au démarrage. */
  defaultFilter: DefaultFilter;
  /** Tri de bibliothèque par défaut. */
  defaultSort: SortKey;
  /** Affichage liste (true) ou grille (false) par défaut. */
  listView: boolean;
  /** Densité de la grille (taille des jaquettes). */
  density: Density;
  /** Réduire les animations/transitions (accessibilité, machines modestes). */
  reduceMotion: boolean;
  /**
   * Suivre les sessions de jeu : Torii se minimise au lancement d'un jeu (installé,
   * lancé depuis Torii) et, à la fermeture, revient au premier plan sur la fiche du jeu.
   */
  returnOnGameExit: boolean;
  /** Notifier quand un jeu de la wishlist passe en promo ou atteint son plus bas historique. */
  wishlistNotifications: boolean;
  /**
   * L'invitation à partager sa bibliothèque a été écartée. 🔑 Une proposition se fait une
   * fois : la reproposer à chaque ouverture de la vue Amis transformerait une suggestion
   * en harcèlement — même principe que `steamAutoLinked` côté Rust.
   */
  libraryInviteDismissed: boolean;
  /**
   * Langue de l'interface, et langue demandée aux boutiques pour les descriptions.
   *
   * - `"fr"` / `"en"` : choisie (ou épinglée pour une installation antérieure) ;
   * - `"system"` : suivre Windows, choisi explicitement dans les Paramètres — relu à
   *   chaque démarrage, pour suivre un changement de langue du système ;
   * - `null` : jamais choisie → `langueParDefaut()`, c'est-à-dire l'anglais.
   *
   * ⚠️ `null` NE VEUT PLUS DIRE « SUIVRE WINDOWS ». C'était le cas avant que l'anglais
   * devienne le défaut ; aucune version publiée n'a enregistré ce `null`-là.
   */
  language: Langue | "system" | null;
  /**
   * Région commerciale : devise, tarification, boutiques proposées. `null` = suivre
   * Windows, même raisonnement.
   *
   * ⚠️ INDÉPENDANTE DE `language`, et c'est tout l'intérêt. Voir `i18n/regions.ts`.
   */
  region: CodeRegion | null;
}

const DEFAULTS: Prefs = {
  defaultFilter: "all",
  defaultSort: "recent",
  listView: false,
  density: "normal",
  reduceMotion: false,
  returnOnGameExit: false,
  wishlistNotifications: false,
  libraryInviteDismissed: false,
  language: null,
  region: null,
};

const KEY = "ludo-prefs";
function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? { ...DEFAULTS, ...(JSON.parse(raw) as Partial<Prefs>) } : { ...DEFAULTS };
  } catch {
    return { ...DEFAULTS };
  }
}

const prefs = reactive<Prefs>(loadPrefs());

/** Copie figée des préférences au chargement (pour amorcer l'état initial de l'UI). */
export const initialPrefs: Prefs = { ...prefs };

// Largeur minimale d'une carte selon la densité (pilote `--card-min` de la grille).
const DENSITY_MIN: Record<Density, string> = {
  compact: "150px",
  normal: "178px",
  large: "220px",
};

/** Applique les préférences « visuelles » au document (densité, animations). */
function apply() {
  const root = document.documentElement;
  root.style.setProperty("--card-min", DENSITY_MIN[prefs.density]);
  if (prefs.reduceMotion) root.setAttribute("data-reduce-motion", "");
  else root.removeAttribute("data-reduce-motion");

  // Région : `null` veut dire « suivre Windows », relu à chaque application.
  appliquerLangue(langueEffective());
  appliquerRegion(prefs.region ?? regionDuSysteme());
}

/** La langue à afficher, d'après la préférence — voir `Prefs.language`. */
function langueEffective(): Langue {
  if (prefs.language === "system") return langueDuSysteme();
  return prefs.language ?? langueParDefaut();
}
apply();

/**
 * Pousse langue et région vers la couche native, qui les applique à ses appels réseau.
 * L'échec est déjà géré par `call`, qui retombe silencieusement hors Tauri (aperçu
 * navigateur, démo du site).
 */
function pousserVersRust(): Promise<void> {
  return setLocale(langueEffective(), prefs.region ?? regionDuSysteme(), {
    language: prefs.language,
    region: prefs.region,
  });
}

/**
 * Reprend le réglage de langue et de région retenu côté natif.
 *
 * 🔑 LA COPIE NATIVE GAGNE SUR `localStorage`. Après un redémarrage lancé depuis les
 * Paramètres, WebView2 a déjà relu un `localStorage` antérieur au changement : on passait
 * en français, on redémarrait, et l'anglais revenait. La copie native, elle, est écrite
 * de façon synchrone à chaque changement. Renvoie `false` s'il n'y en a pas.
 */
async function reprendreChoixRetenu(): Promise<boolean> {
  const choix = await choixLocaleRetenu();
  if (!choix) return false;
  const { language, region } = choix;
  if (language === null || language === "system" || language === "fr" || language === "en") {
    if (prefs.language !== language) prefs.language = language;
  }
  if (region === null || REGIONS.some((r) => r.code === region)) {
    if (prefs.region !== region) prefs.region = region as CodeRegion | null;
  }
  return true;
}

/**
 * Épingle le français pour quelqu'un qui utilisait Torii avant qu'il ait des langues.
 *
 * 🔑 L'anglais est le défaut des NOUVELLES installations. Mais un utilisateur qui met à jour
 * n'a jamais choisi de langue — le réglage n'existait pas — et le passer en anglais du jour
 * au lendemain serait le trahir. Rust reconnaît une installation antérieure à ses caches
 * (`locale::charger`) ; ici, on inscrit « fr » dans les préférences pour que ce choix
 * survive, et qu'il apparaisse comme tel dans les Paramètres.
 *
 * ⚠️ Seulement si aucune langue n'a jamais été choisie : un choix explicite gagne toujours.
 */
async function epinglerLangueHeritee(): Promise<void> {
  if (prefs.language !== null && prefs.region !== null) return;
  const heritee = await langueHeritee();
  if (!heritee) return;
  // Le `watch` persiste, réapplique et pousse.
  if (prefs.language === null) prefs.language = heritee;
  // 🔑 LA RÉGION AUSSI. Avant les langues, les prix étaient TOUJOURS français
  // (`country=FR` en dur). « Suivre Windows » donnerait les États-Unis à un utilisateur
  // français équipé d'un Windows en anglais — des dollars du jour au lendemain.
  if (prefs.region === null) prefs.region = "FR";
}

/**
 * Résolue quand Rust connaît la langue et la région de cette session.
 *
 * 🔑 `main.ts` L'ATTEND AVANT DE MONTER L'APPLICATION. Sans ça, la bibliothèque peut
 * lancer son premier chargement de métadonnées avant que la langue soit arrivée côté
 * natif : Rust interrogerait les boutiques avec la langue de la session précédente, et un
 * utilisateur qui vient de passer en anglais recevrait un lot de genres français. Rust
 * relit bien sa copie disque au démarrage, mais au tout premier lancement il n'en a pas —
 * c'est ici que la langue de Windows est détectée pour la première fois.
 */
export const localeTransmise: Promise<void> = reprendreChoixRetenu()
  .then((repris) => (repris ? undefined : epinglerLangueHeritee()))
  .then(pousserVersRust);

// Persiste + réapplique à chaque changement.
watch(prefs, () => {
  void pousserVersRust();
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    /* stockage indisponible : préférences gardées pour la session. */
  }
  apply();
});

export function usePreferences() {
  return { prefs };
}
