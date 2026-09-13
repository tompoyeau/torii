/** Fiche d'un jeu de la bibliothèque (`GameDetail.vue`). */
export default {
  fiche: "Fiche du jeu",
  retour: "Bibliothèque",
  jouerDepuis: "Jouer depuis…",
  installerDepuis: "Installer depuis…",
  voirBoutique: "Voir dans la boutique",
  options: "Options du jeu",
  optionsTitre: "Options",
  retrait: "Retrait…",
  desinstallation: "Désinstallation…",

  apropos: "À propos",
  chargementDetails: "Chargement des détails…",
  aucuneDescription: "Aucune description disponible pour ce jeu.",

  amis: "Amis qui possèdent ce jeu",
  voirProfil: "Voir le profil de {nom}",

  captures: "Captures d'écran",
  capturesPrecedentes: "Captures précédentes",
  capturesSuivantes: "Captures suivantes",
  agrandir: "Agrandir la capture",
  capturePrecedente: "Capture précédente",
  captureSuivante: "Capture suivante",

  succes: "Succès",
  chargementSucces: "Chargement des succès…",
  debloque: "Débloqué",
  verrouille: "Verrouillé",
  reduire: "Réduire",
  toutAfficher: "Afficher tout ({n})",

  stats: {
    heures: "h",
    tempsDeJeu: "de temps de jeu",
    // ⚠️ L'unité se traduit : « Go » en français, « GB » en anglais.
    go: "Go",
    tailleGo: "{n} Go",
    surDisque: "sur le disque",
    tailleJeu: "taille du jeu",
    aucune: "aucune statistique",
    statut: "Statut",
    derniereSession: "Dernière session",
    enCeMoment: "En ce moment",
    developpeur: "Développeur",
    sortie: "Sortie",
    genre: "Genre",
    taille: "Taille",
    familleSteam: "Famille Steam",
    copies: "{n} copie | {n} copies",
  },
} as const;
