import type { Forme } from "../index";
import type fr from "../fr/systeme";

const en: Forme<typeof fr> = {
  fenetre: {
    titre: "Window",
    reduire: "Minimize",
    agrandir: "Maximize",
    restaurer: "Restore",
    fermer: "Close",
  },
  etat: {
    enJeu: "Playing",
    enLigne: "Online",
    invisible: "Invisible",
    synchro: "Syncing…",
    minutes: "{m} min",
    heures: "{h} h {m}",
    ouvrirFiche: "Open {titre}'s page",
    changer: "{etat} — change what your friends see",
  },
  demo: {
    pastille: "Demo",
    texte: "You're trying Torii in your browser, on a made-up library.",
    limite: "Launching a game or connecting an account needs the app.",
    telecharger: "Download Torii",
    masquer: "Hide this banner",
  },

  maj: {
    disponible: "Update available",
    pret: "Torii {version} is ready to install.",
    installer: "Install and restart",
    plusTard: "Later",
    telechargement: "Downloading Torii {version}…",
    enCours: "In progress…",
    installee: "Update installed",
    redemarrage: "Restarting Torii…",
    echec: "Update failed",
  },

  titrePage: "Torii — game library",
  commandeHorsApplication: "{commande}: not available outside the Torii app.",
  socialHorsApplication: "The Torii service is only available in the app.",
};
export default en;
