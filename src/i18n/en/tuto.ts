import type { Forme } from "../index";
import type fr from "../fr/tuto";

const tuto: Forme<typeof fr> = {
  passer: "Skip the tutorial",
  retour: "Back",
  suivant: "Next",
  etape: "Step {n} of {total}",
  accueil: {
    titre: "Welcome to Torii",
    texte: "All your games in one place, whatever the launcher, and the best price for the rest. Three quick steps and you're set.",
  },
  launchers: {
    titre: "Connect your launchers",
    texte: "Connect your accounts to also see the games you haven't installed. No password goes through Torii: you sign in on each launcher's official window. It's optional, and you can do it later in Settings.",
    recherche: "Torii is looking for games installed on this PC…",
    trouves: "Torii has already found one game. | Torii has already found {n} games.",
    aucun: "Torii hasn't found any games on this PC yet.",
  },
  compte: {
    titre: "Take Torii everywhere with you",
    texte: "A Torii account is optional, but it's what takes Torii beyond your PC.",
    avantages: {
      partout: {
        titre: "Your library and the Store on your phone",
        texte: "See your games and compare prices from any browser.",
      },
      amis: {
        titre: "Your friends, whatever the launcher",
        texte: "See who's playing what live, and what you have in common, on Steam or anywhere else.",
      },
      alertes: {
        titre: "Know when a friend jumps in",
        texte: "A discreet banner tells you the moment they start a game.",
      },
      suivi: {
        titre: "Your settings follow you",
        texte: "Your hidden resellers are the same on all your devices, and your playtime too if you want.",
      },
    },
    rassurer: "Free and passwordless: a code by email is all it takes. Nothing is shared until you turn it on.",
    creer: "Create my account",
    plusTard: "Later",
    connecte: "All set, {nom}!",
    connecteTexte: "Your account is ready. You'll find all its settings in Settings → Torii network.",
    decouvrir: "Discover Torii",
  },
  visite: {
    bulle: "{n} of {total}",
    terminer: "Finish",
    bibliotheque: {
      titre: "Your library",
      texte: "All your games together, whatever the launcher. Filter by platform and launch any game from here.",
    },
    boutique: {
      titre: "The Store",
      texte: "Compare a game's price across every PC store, and spot the real deals.",
    },
    wishlist: {
      titre: "Your wishlist",
      texte: "Keep the games you're after close at hand, wherever they're sold. Torii can tell you when their price drops.",
    },
    amis: {
      titre: "Your friends",
      texte: "See what your friends are playing and what you have in common. To see this tutorial again: Settings → General.",
    },
  },
};
export default tuto;
