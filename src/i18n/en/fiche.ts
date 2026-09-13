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
  retrait: "Removing…",
  desinstallation: "Uninstalling…",

  apropos: "About",
  chargementDetails: "Loading details…",
  aucuneDescription: "No description available for this game.",

  amis: "Friends who own this game",
  voirProfil: "View {nom}'s profile",

  captures: "Screenshots",
  capturesPrecedentes: "Previous screenshots",
  capturesSuivantes: "Next screenshots",
  agrandir: "Enlarge screenshot",
  capturePrecedente: "Previous screenshot",
  captureSuivante: "Next screenshot",

  succes: "Achievements",
  chargementSucces: "Loading achievements…",
  debloque: "Unlocked",
  verrouille: "Locked",
  reduire: "Show less",
  toutAfficher: "Show all ({n})",

  stats: {
    heures: "h",
    tempsDeJeu: "played",
    go: "GB",
    tailleGo: "{n} GB",
    surDisque: "on disk",
    tailleJeu: "game size",
    aucune: "no stats yet",
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
