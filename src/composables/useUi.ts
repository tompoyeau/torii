import { reactive, toRefs } from "vue";
import type { LibraryFilter, SortKey } from "../types";
import { initialPrefs } from "./usePreferences";

/** Section affichée dans la zone principale : bibliothèque, boutique, amis… */
export type MainSection =
  | "library" | "store" | "friends" | "common" | "wishlist"
  | "friendProfile" | "friendLibrary" | "web";

/** Catégorie affichée dans la pop-in Paramètres. */
export type SettingsCategory = "general" | "hidden" | "stores" | "accounts" | "torii" | "about" | "raccourcis";

interface UiState {
  section: MainSection;
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
  /** Ami dont on consulte le profil (section `friendProfile`), ou null.
      🔑 C'est la **clé unifiée** (`UnifiedFriend.key` : `torii:<id>` ou `steam:<id>`) et
      pas un identifiant de compte Torii : un ami Steam sans compte Torii a lui aussi sa
      page, où on lui explique justement pourquoi on n'y sait presque rien de lui. */
  friendProfileKey: string | null;
  /** Compte Torii dont on consulte la bibliothèque (section `friendLibrary`), ou null. */
  friendLibraryId: string | null;
  /** Adresse affichée par la page web intégrée (section `web`), ou null. Suit la page :
      elle change quand on y clique un lien, sans faire d'étape d'historique. */
  pageWeb: string | null;
}

const state = reactive<UiState>({
  // Amorçage depuis les préférences persistées (filtre/tri/vue par défaut).
  section: "library",
  friendProfileKey: null,
  friendLibraryId: null,
  pageWeb: null,
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

// L'historique de navigation (précédent / suivant) vit dans `useNavigation.ts` : il
// suit aussi la fiche Boutique, qui n'est pas un état de ce module.

export function useUi() {
  return {
    ...toRefs(state),
    // Choisir un filtre ramène toujours à la bibliothèque (quitte la boutique).
    setFilter: (filter: LibraryFilter) => {
      state.filter = filter;
      state.section = "library";
    },
    /** Revient à la bibliothèque, sur le filtre où on l'avait laissée. */
    showLibrary: () => {
      state.section = "library";
      window.scrollTo({ top: 0 });
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
    /**
     * Ouvre la page profil d'un ami, Torii ou Steam.
     *
     * 🔑 Prend la **clé unifiée** (`UnifiedFriend.key`) et non un identifiant de compte
     * Torii : tout le monde a une page. Celle d'un ami Steam est presque vide — c'est le
     * propos, elle dit ce que Torii ne peut pas savoir de lui et pourquoi.
     */
    showFriendProfile: (key: string) => {
      state.friendProfileKey = key;
      state.section = "friendProfile";
      window.scrollTo({ top: 0 });
    },
    /** Ouvre la bibliothèque partagée d'un ami (section à part entière, pas une modale :
        elle se parcourt et se filtre comme la sienne). */
    showFriendLibrary: (accountId: string) => {
      state.friendLibraryId = accountId;
      state.section = "friendLibrary";
      window.scrollTo({ top: 0 });
    },
    showWishlist: () => {
      state.section = "wishlist";
      window.scrollTo({ top: 0 });
    },
    /** Affiche une page web dans Torii. Passer par `ouvrirPage` (`usePageWeb`), qui
        referme d'abord les fiches et retombe sur le navigateur hors de l'application. */
    showWeb: (url: string) => {
      state.pageWeb = url;
      state.section = "web";
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
  };
}
