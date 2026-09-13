import type { Forme } from "../index";
import type fr from "../fr/bibliotheque";

const en: Forme<typeof fr> = {
  jeux: "{n} game | {n} games",
  heures: "{n} h",
  heuresDeJeu: "{n} h played",
  installe: "Installed",
  nonInstalle: "Not installed",
  jouer: "Play",
  installer: "Install",

  horsLauncher: "No launcher",

  temps: {
    moinsDUneHeure: "less than an hour ago",
  },

  laterale: {
    bibliotheque: "Library",
    tous: "All games",
    mesJeux: "My games",
    famille: "Family",
    enCommun: "In common",
    favoris: "Favorites",
    installes: "Installed",
    decouvrir: "Discover",
    boutique: "Store",
    wishlist: "Wishlist",
    plateformes: "Platforms",
    gererConnexions: "Manage connections",
    connecterLauncher: "Connect a launcher",
  },

  barre: {
    rechercher: "Search your library…",
    effacer: "Clear search",
    actualisation: "Refreshing…",
    ajouterJeu: "Add a game manually",
    ajouter: "Add",
    amis: "Friends",
    resynchroniser: "Sync again",
    theme: "Light / dark theme",
    parametres: "Settings",
    monCompte: "My account",
    monCompteTorii: "{nom} — my Torii account",
    toriiConnecte: "Torii account connected",
  },

  grille: {
    filtres: {
      all: "All games",
      mine: "My games",
      family: "Shared by family",
      recent: "Recently played",
      favorite: "Favorites",
      installed: "Installed",
      hidden: "Hidden",
      horsLauncher: "No launcher",
    },
    tris: {
      recent: "Recently played",
      alpha: "A → Z",
      playtime: "Playtime",
    },
    toutesCategoriesCourt: "All categories",
    toutesCategories: "All categories",
    installesSeulement: "Installed only",
    installesSeulementAide: "Show installed games only",
    aucun: "No games match your search.",
  },

  carte: {
    masquer: "Hide this game",
    reafficher: "Show again",
    ajouterFavoris: "Add to favorites",
    retirerFavoris: "Remove from favorites",
    copiesFamille: "{n} copies in your Steam family",
  },

  vedette: {
    reprendre: "Pick up where you left off",
    honneur: "In the spotlight",
    joue: "Played {quand}",
    details: "Details",
  },

  menu: {
    voirFiche: "View details",
    ouvrirEmplacement: "Open file location",
    diffuser: "Share this game with friends",
    nePasDiffuser: "Don't share this game",
    modifier: "Edit details",
    retirer: "Remove from library",
    desinstaller: "Uninstall",
  },
};
export default en;
