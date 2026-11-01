import type { Forme } from "../index";
import type fr from "../fr/web";

const web: Forme<typeof fr> = {
  navigation: "Main navigation",
  injoignable: "The service isn't responding right now. Try again in a minute.",
  pasSurLeWeb: "This only works in Torii on your PC.",
  connexion: "Your library and the Store, on any device. Sign in with your Torii account.",
  sansCompte: "No account yet? You create one in Torii on your PC.",
  telecharger: "Download Torii",
  pasDeCompte: "No Torii account for this address. Create it in Torii on your PC, then come back here.",
  captcha: {
    echec: "The anti-bot check couldn't load.",
    reessayer: "Try again",
    refuse: "The anti-bot check didn't go through. Try again.",
  },
  bibliotheque: {
    videTitre: "Your library isn't here yet",
    videTexte: "In Torii on your PC, open Settings → Torii Network and turn on library sync. Your games will show up here.",
    versBoutique: "Go to the Store",
    indisponible: "Your library couldn't be loaded.",
  },
  reglages: {
    langueNavigateur: "Follow the browser",
    regionNavigateur: "Follow the browser ({pays})",
    revendeursLocaux: "This list only applies to this browser.",
  },
};
export default web;
