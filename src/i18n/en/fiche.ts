import type { Forme } from "../index";
import type fr from "../fr/fiche";

const en: Forme<typeof fr> = {
  fiche: "Game details",
  retour: "Library",
  jouerDepuis: "Play from…",
  installerDepuis: "Install from…",
  voirBoutique: "View in store",
  options: "Game options",
  optionsTitre: "Options",
  actualiserInfos: "Refresh details",
  retrait: "Removing…",
  desinstallation: "Uninstalling…",

  apropos: "About",
  chargementDetails: "Loading details…",
  aucuneDescription: "No description available for this game.",
  enAnglais: "in English · IGDB",

  bandeau: {
    tempsDeJeu: "Playtime",
    selon: "according to {plateforme}",
    derniereSession: "Last session",
    installe: "Installed",
    surDisque: "on disk",
    aInstaller: "Not installed",
    aTelecharger: "to download",
  },

  actualites: {
    titre: "Game news",
    sousTitre: "The latest announcements from {studio} on Steam: updates, events, patch notes.",
    sousTitreEditeur: "The latest announcements from the publisher on Steam: updates, events, patch notes.",
    toutes: "All news",
  },

  succesFiche: {
    rares: "Your rarest achievements",
    aViser: "Next to go for",
    tous: "See all {n} achievements, rarest first",
    masquer: "Back to summary",
    part: "{pct} of players",
    debloqueLe: "Unlocked {date}",
    caches: "{n} hidden achievement left | {n} hidden achievements left",
    cachesAide: "Revealed once unlocked",
    toutDebloque: "You've unlocked every achievement in this game.",
  },

  dlc: {
    titre: "Downloadable content",
    possedes: "{n} owned | {n} owned",
    possede: "Owned",
    toutVoir: "See all on Steam",
  },

  liens: {
    titre: "Links",
    pageSteam: "Steam page",
    site: "Official website",
    communaute: "Steam Community",
  },

  amis: "Friends who own this game",
  voirProfil: "View {nom}'s profile",

  capturesPrecedentes: "Previous screenshots",
  capturesSuivantes: "Next screenshots",
  capturePrecedente: "Previous screenshot",
  captureSuivante: "Next screenshot",

  succes: "Achievements",
  chargementSucces: "Loading achievements…",

  stats: {
    heures: "h",
    go: "GB",
    tailleGo: "{n} GB",
    surDisque: "on disk",
    tailleJeu: "game size",
    statut: "Status",
    derniereSession: "Last session",
    enCeMoment: "Playing now",
    developpeur: "Developer",
    sortie: "Released",
    genre: "Genre",
    taille: "Size",
    familleSteam: "Steam family",
    copies: "{n} copy | {n} copies",
  },
};
export default en;
