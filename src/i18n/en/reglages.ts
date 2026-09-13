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
    apropos: "About & maintenance",
    masques: "Hidden games",
    revendeurs: "Hidden sellers",
    comptes: "Accounts & launchers",
    torii: "Torii network",
  },

  general: {
    autostart: {
      titre: "Launch when Windows starts",
      sous: "Torii opens automatically when you sign in.",
    },
    minimise: {
      titre: "Start minimized",
      sous: "Starts hidden in the notification area (next to the clock).",
    },
    tray: {
      titre: "Close to the notification area",
      sous:
        "On by default: Torii keeps tracking your sessions after the window is closed. " +
        "Turn it off to make the close button actually quit — “Quit” is still available " +
        "by right-clicking the icon next to the clock.",
    },
    retourJeu: {
      titre: "Come back when a game closes",
      sous:
        "Torii minimizes when a game starts, then comes back to that game's page when you " +
        "close it. (Installed games launched from Torii.)",
    },
    theme: {
      titre: "Theme",
      sous: "How the app looks.",
      systeme: "System",
      clair: "Light",
      sombre: "Dark",
    },
    vue: {
      titre: "Default view",
      sous: "Filter, sort and layout when the library opens.",
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
      sous: "Cover size in the grid.",
      compact: "Compact",
      normal: "Normal",
      grand: "Large",
    },
    animations: {
      titre: "Reduce motion",
      sous: "Turns off transitions and effects (accessibility, low-end machines).",
    },
    alertesPrix: {
      titre: "Price alerts (wishlist)",
      sous:
        "A notification when a game on your Steam wishlist goes on sale or hits its " +
        "all-time low. (Torii has to be running, even minimized to the tray.)",
    },
  },

  langue: {
    titre: "Language",
    description:
      "The language of the interface, and the one Torii asks stores for when it fetches " +
      "descriptions and genres.",
    systeme: "Follow Windows",
    appliquer:
      "Descriptions and genres of games already on screen will switch language the next " +
      "time Torii starts.",
    redemarrer: "Restart now",
  },

  region: {
    titre: "Region",
    description:
      "The country used for pricing: it sets the currency and the amounts shown in the " +
      "Store and your wishlist.",
    note: "Independent from the language: you can read Torii in English with prices in euros.",
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
        "Deletes cached metadata, covers and prices (downloaded again when needed). Your " +
        "accounts and favorites aren't affected.",
      bouton: "Clear cache",
      enCours: "Clearing…",
      indisponible: "Not available outside the app.",
      vide:
        "Cache cleared ({n} file). Data will be downloaded again when needed. | " +
        "Cache cleared ({n} files). Data will be downloaded again when needed.",
    },
    journal: {
      titre: "App log",
      sous:
        "Startups, errors and incidents. Attach it when you report a problem: it's what " +
        "makes it possible to understand what happened on your machine.",
      bouton: "Open log",
    },
    signaler: {
      titre: "Report a problem",
      sous:
        "Opens a report already filled in with your Torii version. It's the only way a bug " +
        "reaches someone who can fix it.",
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
    sous: "Hidden games are removed from the library. Bring them back here.",
    reafficher: "Show again",
    aucun: "No hidden games.",
  },

  revendeurs: {
    sous: "Stores hidden from the Store's price comparison.",
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
        "The name your friends see. It doesn't have to be unique and nobody can find you " +
        "with it — only your friend code does that.",
      misAJour: "Display name updated.",
    },

    code: {
      titre: "Your friend code",
      sous:
        "Give it to people directly so they can add you. Renewing it makes the old one " +
        "stop working — handy if you've shared it too widely.",
      renouveler: "Renew",
      renouvele: "New friend code: the old one no longer works.",
    },

    presence: {
      titre: "What your friends see",
      sous: "While you're invisible, nothing about what you play leaves your PC.",
      detaille: "Game visible",
      detailleAide: "Your friends see what you're playing and for how long.",
      enLigne: "Online",
      enLigneAide: "They know you're online, not what you're playing.",
      invisible: "Invisible",
      invisibleAide: "Nobody sees anything. You still see your friends.",
    },

    absent: {
      titre: "Go “away” after",
      sous: "With no keyboard or mouse input.",
      minutes: "{n} min",
    },

    steam: {
      titre: "Visible to my Steam friends",
      sous:
        "Lets your Steam friends who already use Torii find you, and merges your profile " +
        "with your Steam profile. You both need to have it turned on.",
      sousSansSteam:
        "Connect your Steam account first in “Accounts & launchers”: without it, there's " +
        "nothing to match.",
      retrouver: "Find my Steam friends",
      retrouverAide:
        "Torii compares your Steam friends list with existing accounts. Only people who " +
        "also turned this on show up — that's what stops anyone from using it to find out " +
        "who uses Torii.",
      chercher: "Search my Steam friends",
      surSteam: "{nom} on Steam",
      aucun: "None of your Steam friends has a visible Torii account yet.",
    },

    bibliotheque: {
      titre: "My library",
      aide:
        "What you own, stored on the server so you get it back on your other devices — " +
        "and can show it to your friends, whatever the launcher. Hidden games and games " +
        "marked “don't share” are never included.",
      synchro: "Sync my library",
      synchroSous:
        "It's sent after a scan, and only if it changed since last time. Turning it off " +
        "deletes it from the server.",
      partage: "Visible to my Torii friends",
      partageSous:
        "Your friends see what you own, on Steam or not. When off, your library is only " +
        "for you and your own devices.",
      partageSansSynchro: "Turn on sync first: without it, there's nothing to show.",
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
        "Your friends see all your devices as a single library. Removing an old PC deletes " +
        "its library from the server.",
      ligneAppareil: "{n} game · {quand} | {n} games · {quand}",
    },

    notifAmi: {
      titre: "Notify me when a friend starts a game",
      sous:
        "A banner shows for a few seconds at the top right of the screen, without taking " +
        "focus. A game in exclusive fullscreen may hide it.",
    },

    silence: {
      titre: "Games never shared",
      aide:
        "These games never appear in your status, even while you play. Useful for apps " +
        "that run all the time.",
      rediffuser: "Share again",
      aucun: "No games here. Right-click a game to add it.",
    },

    appareils: {
      titre: "My devices",
      aide:
        "The machines where this Torii account is signed in. A session unused for six " +
        "months ends on its own, but if you don't recognize a device, sign it out right " +
        "away: it takes effect immediately.",
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
        "Your display name, friend code and all your connections are removed from the " +
        "server. Your friends won't see you in their list anymore. Your library and " +
        "settings stay on this computer: only the Torii account is deleted.",
      bouton: "Delete my Torii account",
      avant: "This is permanent: there's no trash, and the same friend code won't come back. Type",
      apres: "to confirm.",
      enCours: "Deleting…",
      definitif: "Delete permanently",
      fait: "Your Torii account has been deleted.",
    },
  },
};
export default en;
