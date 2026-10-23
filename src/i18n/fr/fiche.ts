/** Fiche d'un jeu de la bibliothèque (`GameDetail.vue`). */
export default {
  fiche: "Fiche du jeu",
  retour: "Bibliothèque",
  jouerDepuis: "Jouer depuis…",
  installerDepuis: "Installer depuis…",
  voirBoutique: "Voir dans la boutique",
  options: "Options du jeu",
  optionsTitre: "Options",
  actualiserInfos: "Actualiser les infos",
  retrait: "Retrait…",
  desinstallation: "Désinstallation…",

  apropos: "À propos",
  chargementDetails: "Chargement des détails…",
  aucuneDescription: "Aucune description disponible pour ce jeu.",
  // Description qu'aucune source n'a donnée en français : celle d'IGDB, en anglais.
  enAnglais: "en anglais · IGDB",

  // Sous le titre : ton rapport au jeu.
  bandeau: {
    tempsDeJeu: "Temps de jeu",
    selon: "selon {plateforme}",
    derniereSession: "Dernière session",
    installe: "Installé",
    surDisque: "sur le disque",
    aInstaller: "À installer",
    aTelecharger: "à télécharger",
  },

  actualites: {
    titre: "Actualités du jeu",
    sousTitre: "Les dernières annonces de {studio} sur Steam : mises à jour, événements, notes de version.",
    sousTitreEditeur: "Les dernières annonces de l'éditeur sur Steam : mises à jour, événements, notes de version.",
    toutes: "Toutes les actualités",
  },

  succesFiche: {
    rares: "Tes succès les plus rares",
    aViser: "Prochains à viser",
    tous: "Voir les {n} succès, triés par rareté",
    masquer: "Revenir au résumé",
    part: "{pct} des joueurs",
    debloqueLe: "Débloqué le {date}",
    caches: "{n} succès caché restant | {n} succès cachés restants",
    cachesAide: "Révélés une fois débloqués",
    toutDebloque: "Tu as débloqué tous les succès du jeu.",
  },

  dlc: {
    titre: "Contenus additionnels",
    possedes: "{n} possédé | {n} possédés",
    possede: "Possédé",
    toutVoir: "Tout voir sur Steam",
  },

  liens: {
    titre: "Liens",
    pageSteam: "Page Steam",
    site: "Site officiel",
    communaute: "Communauté Steam",
  },

  amis: "Amis qui possèdent ce jeu",
  voirProfil: "Voir le profil de {nom}",

  capturesPrecedentes: "Captures précédentes",
  capturesSuivantes: "Captures suivantes",
  capturePrecedente: "Capture précédente",
  captureSuivante: "Capture suivante",

  succes: "Succès",
  chargementSucces: "Chargement des succès…",

  stats: {
    heures: "h",
    // ⚠️ L'unité se traduit : « Go » en français, « GB » en anglais.
    go: "Go",
    tailleGo: "{n} Go",
    surDisque: "sur le disque",
    tailleJeu: "taille du jeu",
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
