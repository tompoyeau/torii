/**
 * Cadre de l'application et bibliothèque : barre latérale, barre du haut, grille, cartes,
 * vedette, menu contextuel.
 */
export default {
  jeux: "{n} jeu | {n} jeux",
  heures: "{n} h",
  heuresDeJeu: "{n} h de jeu",
  installe: "Installé",
  nonInstalle: "Non installé",
  jouer: "Jouer",
  installer: "Installer",

  // Noms de plateformes qui ne sont pas des marques : les autres (Steam, GOG…) sont des
  // noms propres et ne se traduisent pas.
  horsLauncher: "Hors launcher",

  temps: {
    moinsDUneHeure: "il y a moins d'une heure",
  },

  laterale: {
    bibliotheque: "Bibliothèque",
    tous: "Tous les jeux",
    mesJeux: "Mes jeux",
    famille: "Famille",
    enCommun: "En commun",
    favoris: "Favoris",
    installes: "Installés",
    decouvrir: "Découvrir",
    boutique: "Boutique",
    wishlist: "Wishlist",
    plateformes: "Plateformes",
    gererConnexions: "Gérer les connexions",
    connecterLauncher: "Connecter un launcher",
  },

  barre: {
    rechercher: "Rechercher dans la bibliothèque…",
    effacer: "Effacer la recherche",
    actualisation: "Actualisation…",
    ajouterJeu: "Ajouter un jeu manuellement",
    ajouter: "Ajouter",
    amis: "Amis",
    resynchroniser: "Resynchroniser",
    theme: "Thème clair / sombre",
    parametres: "Paramètres",
    monCompte: "Mon compte",
    monCompteTorii: "{nom} — mon compte Torii",
    toriiConnecte: "Compte Torii connecté",
  },

  grille: {
    filtres: {
      all: "Tous les jeux",
      mine: "Mes jeux",
      family: "Partagés en famille",
      recent: "Joués récemment",
      favorite: "Favoris",
      installed: "Installés",
      hidden: "Masqués",
      horsLauncher: "Hors launcher",
    },
    tris: {
      recent: "Récemment joué",
      alpha: "A → Z",
      playtime: "Temps de jeu",
    },
    toutesCategoriesCourt: "Toutes catégories",
    toutesCategories: "Toutes les catégories",
    installesSeulement: "Installés uniquement",
    installesSeulementAide: "N'afficher que les jeux installés",
    aucun: "Aucun jeu ne correspond à ta recherche.",
  },

  carte: {
    masquer: "Masquer ce jeu",
    reafficher: "Réafficher",
    ajouterFavoris: "Ajouter aux favoris",
    retirerFavoris: "Retirer des favoris",
    copiesFamille: "{n} copies dans ta famille Steam",
  },

  vedette: {
    reprendre: "Reprendre la partie",
    honneur: "À l'honneur",
    joue: "Joué {quand}",
    details: "Détails",
  },

  menu: {
    voirFiche: "Voir la fiche",
    ouvrirEmplacement: "Ouvrir l'emplacement du fichier",
    diffuser: "Diffuser ce jeu aux amis",
    nePasDiffuser: "Ne pas diffuser ce jeu",
    modifier: "Modifier les informations",
    retirer: "Retirer de la bibliothèque",
    desinstaller: "Désinstaller",
  },
} as const;
