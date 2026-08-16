import { reactive, watch } from "vue";
import type { AppMode, SortKey } from "../types";

/** Densité de la grille de bibliothèque (taille des jaquettes). */
export type Density = "compact" | "normal" | "large";
/** Filtre de départ proposé dans les préférences (sous-ensemble simple). */
export type DefaultFilter = "all" | "recent" | "favorite" | "installed";

interface Prefs {
  /** Mode ouvert au démarrage (Bureau ou Salon). */
  defaultMode: AppMode;
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
}

const DEFAULTS: Prefs = {
  defaultMode: "bureau",
  defaultFilter: "all",
  defaultSort: "recent",
  listView: false,
  density: "normal",
  reduceMotion: false,
  returnOnGameExit: false,
  wishlistNotifications: false,
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
}
apply();

// Persiste + réapplique à chaque changement.
watch(prefs, () => {
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
