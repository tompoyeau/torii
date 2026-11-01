/** La version web : ce qui n'existe pas dans l'application. */
export default {
  injoignable: "Le service ne répond pas pour l'instant. Réessaie dans une minute.",
  pasSurLeWeb: "Ça ne se fait que dans Torii sur ton PC.",
  connexion: "Ta bibliothèque et la Boutique, sur n'importe quel appareil. Connecte-toi avec ton compte Torii.",
  bibliotheque: {
    videTitre: "Ta bibliothèque n'est pas encore ici",
    videTexte: "Dans Torii sur ton PC, ouvre Paramètres → Réseau Torii et active la synchronisation de la bibliothèque. Tes jeux apparaîtront ici.",
    versBoutique: "Aller à la Boutique",
    indisponible: "Ta bibliothèque n'a pas pu être chargée.",
  },
  reglages: {
    langueNavigateur: "Suivre le navigateur",
    regionNavigateur: "Suivre le navigateur ({pays})",
    revendeursLocaux: "Cette liste vaut pour ce navigateur seulement.",
  },
} as const;
