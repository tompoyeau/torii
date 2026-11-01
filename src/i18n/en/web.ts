import type { Forme } from "../index";
import type fr from "../fr/web";

const web: Forme<typeof fr> = {
  injoignable: "The service isn't responding right now. Try again in a minute.",
  pasSurLeWeb: "This only works in Torii on your PC.",
  connexion: "Your library and the Store, on any device. Sign in with your Torii account.",
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
