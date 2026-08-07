import { reactive, toRefs } from "vue";
import type { AppMode, LibraryFilter, SortKey } from "../types";

interface UiState {
  mode: AppMode;
  filter: LibraryFilter;
  query: string;
  sort: SortKey;
  listView: boolean;
  /** Dans une vue de plateforme : n'afficher que les jeux installés. */
  installedOnly: boolean;
  /** Catégorie (genre) sélectionnée, ou null pour toutes. Se combine aux autres filtres. */
  genre: string | null;
  settingsOpen: boolean;
  /** Modale « Ajouter un jeu manuellement » ouverte. */
  addGameOpen: boolean;
  /** id du jeu ouvert dans la vue détail, ou null. */
  selectedGameId: string | null;
}

const state = reactive<UiState>({
  mode: "bureau",
  filter: "all",
  query: "",
  sort: "recent",
  listView: false,
  installedOnly: false,
  genre: null,
  settingsOpen: false,
  addGameOpen: false,
  selectedGameId: null,
});

export function useUi() {
  return {
    ...toRefs(state),
    setMode: (mode: AppMode) => {
      state.mode = mode;
      window.scrollTo({ top: 0 });
    },
    setFilter: (filter: LibraryFilter) => (state.filter = filter),
    setSort: (sort: SortKey) => (state.sort = sort),
    toggleListView: () => (state.listView = !state.listView),
    toggleInstalledOnly: () => (state.installedOnly = !state.installedOnly),
    setGenre: (genre: string | null) => (state.genre = genre),
    openSettings: () => (state.settingsOpen = true),
    closeSettings: () => (state.settingsOpen = false),
    openAddGame: () => (state.addGameOpen = true),
    closeAddGame: () => (state.addGameOpen = false),
    openGame: (id: string) => (state.selectedGameId = id),
    closeGame: () => (state.selectedGameId = null),
  };
}
