import type { Forme } from "../index";
import type fr from "../fr/reglages";

const en: Forme<typeof fr> = {
  titre: "Settings",

  groupes: {
    application: "App",
    bibliotheque: "Library & Store",
    comptes: "Accounts",
  },

  categories: {
    general: "General",
    raccourcis: "Keyboard shortcuts",
    apropos: "About & maintenance",
    masques: "Hidden games",
    revendeurs: "Hidden sellers",
    comptes: "Accounts & launchers",
    torii: "Torii network",
  },

  general: {
    autostart: {
      titre: "Launch when Windows starts",
      sous: "Torii opens on its own when you turn on your PC.",
    },
    minimise: {
      titre: "Start minimized",
      sous: "Torii starts in the background, without opening its window.",
    },
    tray: {
      titre: "Close to the notification area",
      sous:
        "The close button tucks Torii next to the clock instead of quitting, so it keeps counting your play sessions.",
    },
    retourJeu: {
      titre: "Come back when a game closes",
      sous:
        "Torii steps aside while you play, then comes back to the game's page when you quit it.",
    },
    theme: {
      titre: "Theme",
      sous: "Light, dark, or like Windows.",
      systeme: "System",
      clair: "Light",
      sombre: "Dark",
    },
    vue: {
      titre: "Default view",
      sous: "What the library shows when it opens.",
      tous: "All",
      favoris: "Favorites",
      installes: "Installed",
      recent: "Recently played",
      alpha: "A → Z",
      tempsDeJeu: "Playtime",
      grille: "Grid",
      liste: "List",
    },
    densite: {
      titre: "Library density",
      sous: "Cover size.",
      compact: "Compact",
      normal: "Normal",
      grand: "Large",
    },
    animations: {
      titre: "Reduce motion",
      sous: "Less movement on screen.",
    },
    tuto: {
      titre: "Welcome tutorial",
      sous: "Take the first-launch tour again.",
      bouton: "See the tutorial again",
    },
    alertesPrix: {
      titre: "Price alerts (wishlist)",
      sous:
        "A notification when a game on your wishlist goes on sale or hits its lowest price.",
    },
  },

  langue: {
    titre: "Language",
    description:
      "The language of Torii and of game descriptions.",
    systeme: "Follow Windows",
    appliquer:
      "Pages already open will switch language the next time Torii starts.",
    redemarrer: "Restart now",
  },

  region: {
    titre: "Region",
    description:
      "The country for your prices: it sets the currency in the Store and your wishlist.",
    note: "Independent from the language: Torii in English with prices in euros works fine.",
    systeme: "Follow Windows ({pays})",
    devise: "Currency: {devise}",
  },

  apropos: {
    version: "Torii version",
    maj: {
      verification: "Checking…",
      disponible: "Update available: {version}",
      telechargement: "Downloading…",
      installee: "Installed — restarting…",
      erreur: "Couldn't check for updates.",
      aJour: "Torii is up to date.",
    },
    verifier: "Check for updates",
    installer: "Install now",
    cache: {
      titre: "Clear cache",
      sous:
        "Frees up space, or fixes a cover or price that won't update. Your accounts and favorites stay as they are.",
      bouton: "Clear cache",
      enCours: "Clearing…",
      indisponible: "Not available outside the app.",
      vide:
        "Cache cleared.",
    },
    journal: {
      titre: "App log",
      sous:
        "Attach it when you report a problem.",
      bouton: "Open log",
    },
    signaler: {
      titre: "Report a problem",
      sous:
        "Tell us what's wrong: your Torii version is added automatically.",
      bouton: "Report",
      rapport: {
        cequisepasse: "**What happens**",
        attendu: "**What you expected**",
        reproduire: "**How to reproduce it**",
        demo: "Web demo (not installed)",
        journal1: "Please attach the log: Settings → About & maintenance →",
        journal2: "“Open log”, then paste its contents here.",
      },
    },
  },

  masques: {
    sous: "Games you removed from your library.",
    reafficher: "Show again",
    aucun: "No hidden games.",
  },

  revendeurs: {
    sous: "Stores you no longer want to see in prices.",
    reafficher: "Show again",
    toutReafficher: "Show all again",
    aucun: "No hidden sellers. You can hide one from a game's page in the Store.",
  },

  torii: {
    intro:
      "See what your friends are playing, whatever their launcher — and show them what " +
      "you're playing, if you choose to.",

    pseudo: {
      titre: "Your display name",
      sous:
        "The name your friends see.",
      misAJour: "Display name updated.",
    },

    code: {
      titre: "Your friend code",
      sous:
        "Give it to your friends so they can add you. Renewing it turns off the old one.",
      renouveler: "Renew",
      renouvele: "New friend code: the old one no longer works.",
    },

    presence: {
      titre: "What your friends see",
      sous: "When invisible, your friends see nothing of what you're doing.",
      detaille: "Game visible",
      detailleAide: "Your friends see what you're playing and for how long.",
      enLigne: "Online",
      enLigneAide: "They know you're online, not what you're playing.",
      invisible: "Invisible",
      invisibleAide: "Nobody sees anything. You still see your friends.",
    },

    absent: {
      titre: "Go “away” after",
      sous: "When you stop using the keyboard and mouse.",
      minutes: "{n} min",
    },

    steam: {
      titre: "Visible to my Steam friends",
      sous:
        "Your Steam friends who use Torii can find you, if they've turned this on too.",
      sousSansSteam:
        "Connect your Steam account first in “Accounts & launchers”.",
      retrouver: "Find my Steam friends",
      retrouverAide:
        "Only people who turned this on too show up.",
      chercher: "Search my Steam friends",
      surSteam: "{nom} on Steam",
      aucun: "None of your Steam friends has a visible Torii account yet.",
    },

    bibliotheque: {
      titre: "My library",
      aide:
        "Get your games on your phone and on the web, and show them to your friends if you like. Hidden games are never included.",
      synchro: "Sync my library",
      synchroSous:
        "It updates on its own. Turning it off deletes your online library.",
      partage: "Visible to my Torii friends",
      partageSous:
        "Your friends see your games, from all your launchers.",
      tempsDeJeu: "My playtime on my other devices",
      tempsDeJeuSous:
        "Your playtime and recent sessions, on the web and your phone. Your friends don't see them.",
      partageSansSynchro: "Turn on sync first.",
      envoye: "{n} game sent from “{appareil}”, {quand}. | {n} games sent from “{appareil}”, {quand}.",
      rienEnvoye: "Nothing has been sent from this device yet.",
      maintenant: "Sync now",
      envoi: "Sending…",
      activee: "Library synced.",
      coupee: "Sync turned off, library deleted from the server.",
      envoyee: "Library sent ({n} game). | Library sent ({n} games).",
      rienAEnvoyer: "Nothing to send.",
      autres: "My other devices",
      autresAide:
        "Removing an old PC deletes its online library.",
      ligneAppareil: "{n} game · {quand} | {n} games · {quand}",
    },

    notifAmi: {
      titre: "Notify me when a friend starts a game",
      sous:
        "A small banner for a few seconds, at the top right of the screen.",
    },

    silence: {
      titre: "Games never shared",
      aide:
        "These games never show up in what your friends see. Handy for apps that are always open.",
      rediffuser: "Share again",
      aucun: "No games here. Right-click a game to add it.",
    },

    appareils: {
      titre: "My devices",
      aide:
        "Devices signed in to your account. If you don't recognize one, sign it out.",
      celuiCi: "this device",
      connecte: "Signed in {quand}",
      actif: "active {quand}",
      deconnecter: "Sign out",
      aucun: "No signed-in devices to show.",
      deconnecterTous: "Sign out all other devices",
      confirmationTous:
        "The other device will have to sign in again. | The {n} other devices will have to sign in again.",
      deconnecte: "“{nom}” has been signed out.",
      deconnectes: "The other device has been signed out. | {n} devices signed out.",
      deconnecterCompte: "Sign out of this account",
    },

    suppression: {
      titre: "Delete my account",
      aide:
        "Your account, display name and friends are removed. Your games and settings stay on this PC.",
      bouton: "Delete my Torii account",
      avant: "This is permanent: there's no trash, and the same friend code won't come back. Type",
      apres: "to confirm.",
      enCours: "Deleting…",
      definitif: "Delete permanently",
      fait: "Your Torii account has been deleted.",
    },
  },

  raccourcis: {
    sous:
      "Torii works from the keyboard too.",
    infobulle: "{action} ({touches})",
    ou: "or",
    groupes: {
      navigation: "Getting around",
      actions: "Actions",
      medias: "Trailers and screenshots",
    },
    actions: {
      precedent: "Go back to the previous screen",
      suivant: "Go forward to the next screen",
      fermer: "Close the open page or window",
      bibliotheque: "Library",
      boutique: "Store",
      wishlist: "Wishlist",
      amis: "Contacts",
      rechercher: "Jump to the search bar",
      ajouter: "Add a game manually",
      resynchroniser: "Sync the library again",
      parametres: "Open settings",
      raccourcis: "Show these shortcuts",
      capture: "Previous or next screenshot",
      lecture: "Play / pause",
      avanceVideo: "Skip back or forward 5 seconds",
      volume: "Turn the volume up or down",
      muet: "Mute",
      pleinEcran: "Full-screen video",
    },
    touches: {
      ctrl: "Ctrl",
      alt: "Alt",
      echap: "Esc",
      espace: "Space",
      sourisRetour: "Mouse back button",
      sourisAvant: "Mouse forward button",
    },
  },
};
export default en;
