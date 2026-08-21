import { nextTick, reactive, toRefs, watch } from "vue";
import type { AppMode, LibraryFilter, SortKey } from "../types";
import { initialPrefs } from "./usePreferences";

/** Section principale de la vue Bureau : bibliothèque, boutique, amis… */
type BureauSection = "library" | "store" | "friends" | "common" | "wishlist";

/** Catégorie affichée dans la pop-in Paramètres. */
export type SettingsCategory = "general" | "hidden" | "stores" | "accounts" | "torii" | "about";

interface UiState {
  mode: AppMode;
  /** Section affichée dans le mode Bureau (bibliothèque vs boutique). */
  section: BureauSection;
  filter: LibraryFilter;
  query: string;
  sort: SortKey;
  listView: boolean;
  /** Dans une vue de plateforme : n'afficher que les jeux installés. */
  installedOnly: boolean;
  /** Catégorie (genre) sélectionnée, ou null pour toutes. Se combine aux autres filtres. */
  genre: string | null;
  /** Pop-in Paramètres ouverte. */
  settingsOpen: boolean;
  /** Catégorie active de la pop-in Paramètres. */
  settingsCategory: SettingsCategory;
  /** Modale « Ajouter un jeu manuellement » ouverte. */
  addGameOpen: boolean;
  /** id du jeu manuel édité dans cette modale, ou null = création. */
  editGameId: string | null;
  /** id du jeu ouvert dans la vue détail, ou null. */
  selectedGameId: string | null;
}

const state = reactive<UiState>({
  // Amorçage depuis les préférences persistées (mode/filtre/tri/vue par défaut).
  mode: initialPrefs.defaultMode,
  section: "library",
  filter: initialPrefs.defaultFilter,
  query: "",
  sort: initialPrefs.defaultSort,
  listView: initialPrefs.listView,
  installedOnly: false,
  genre: null,
  settingsOpen: false,
  settingsCategory: "general",
  addGameOpen: false,
  editGameId: null,
  selectedGameId: null,
});

// --- Historique de navigation (pour le bouton « précédent » de la souris) ------
// On mémorise un instantané des états de navigation à chaque changement, afin de
// pouvoir revenir en arrière (section, filtre, fiche jeu ouverte, pop-in Paramètres).
interface NavSnap {
  section: BureauSection;
  filter: LibraryFilter;
  selectedGameId: string | null;
  settingsOpen: boolean;
  settingsCategory: SettingsCategory;
}
function snap(): NavSnap {
  return {
    section: state.section,
    filter: state.filter,
    selectedGameId: state.selectedGameId,
    settingsOpen: state.settingsOpen,
    settingsCategory: state.settingsCategory,
  };
}
const navStack: NavSnap[] = [];
let restoring = false;
let lastSnap = snap();
watch(
  () => [state.section, state.filter, state.selectedGameId, state.settingsOpen, state.settingsCategory],
  () => {
    if (restoring) {
      lastSnap = snap();
      return;
    }
    navStack.push(lastSnap);
    if (navStack.length > 60) navStack.shift();
    lastSnap = snap();
  },
);
/** Revient à l'état de navigation précédent, s'il y en a un. */
function goBack(): boolean {
  const prev = navStack.pop();
  if (!prev) return false;
  restoring = true;
  state.section = prev.section;
  state.filter = prev.filter;
  state.selectedGameId = prev.selectedGameId;
  state.settingsOpen = prev.settingsOpen;
  state.settingsCategory = prev.settingsCategory;
  void nextTick(() => {
    restoring = false;
  });
  return true;
}

export function useUi() {
  return {
    ...toRefs(state),
    setMode: (mode: AppMode) => {
      state.mode = mode;
      window.scrollTo({ top: 0 });
    },
    // Choisir un filtre ramène toujours à la bibliothèque (quitte la boutique).
    setFilter: (filter: LibraryFilter) => {
      state.filter = filter;
      state.section = "library";
    },
    showStore: () => {
      state.section = "store";
      window.scrollTo({ top: 0 });
    },
    showFriends: () => {
      state.section = "friends";
      window.scrollTo({ top: 0 });
    },
    showCommon: () => {
      state.section = "common";
      window.scrollTo({ top: 0 });
    },
    showWishlist: () => {
      state.section = "wishlist";
      window.scrollTo({ top: 0 });
    },
    setSort: (sort: SortKey) => (state.sort = sort),
    toggleListView: () => (state.listView = !state.listView),
    toggleInstalledOnly: () => (state.installedOnly = !state.installedOnly),
    setGenre: (genre: string | null) => (state.genre = genre),
    /** Ouvre la pop-in Paramètres, éventuellement sur une catégorie précise. */
    openSettings: (category: SettingsCategory = "general") => {
      state.settingsCategory = category;
      state.settingsOpen = true;
    },
    setSettingsCategory: (category: SettingsCategory) => (state.settingsCategory = category),
    closeSettings: () => (state.settingsOpen = false),
    openAddGame: () => {
      state.editGameId = null;
      state.addGameOpen = true;
    },
    /** Ouvre la même modale, pré-remplie, pour corriger un jeu manuel existant. */
    openEditGame: (id: string) => {
      state.editGameId = id;
      state.addGameOpen = true;
    },
    closeAddGame: () => (state.addGameOpen = false),
    openGame: (id: string) => (state.selectedGameId = id),
    closeGame: () => (state.selectedGameId = null),
    goBack,
  };
}
