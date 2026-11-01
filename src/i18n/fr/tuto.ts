/** Le tuto du premier lancement : l'assistant, puis la visite en bulles. */
export default {
  passer: "Passer le tuto",
  retour: "Retour",
  suivant: "Suivant",
  etape: "Étape {n} sur {total}",
  accueil: {
    titre: "Bienvenue sur Torii",
    texte: "Tous tes jeux au même endroit, quel que soit le launcher, et le meilleur prix pour les autres. Trois petites étapes et c'est parti.",
  },
  launchers: {
    titre: "Connecte tes launchers",
    texte: "Connecte tes comptes pour voir aussi les jeux que tu n'as pas installés. Aucun mot de passe ne passe par Torii : tu te connectes dans la fenêtre officielle de chaque launcher. C'est facultatif, et tu pourras le faire plus tard dans les Paramètres.",
    recherche: "Torii cherche les jeux installés sur ce PC…",
    trouves: "Torii a déjà trouvé un jeu. | Torii a déjà trouvé {n} jeux.",
    aucun: "Torii n'a pas encore trouvé de jeu sur ce PC.",
  },
  compte: {
    titre: "Emporte Torii partout avec toi",
    texte: "Un compte Torii est facultatif, mais c'est lui qui fait sortir Torii de ton PC.",
    avantages: {
      partout: {
        titre: "Ta bibliothèque et la Boutique sur ton téléphone",
        texte: "Retrouve tes jeux et compare les prix depuis n'importe quel navigateur.",
      },
      amis: {
        titre: "Tes amis, quel que soit le launcher",
        texte: "Vois qui joue à quoi en direct, et ce que vous avez en commun, sur Steam ou ailleurs.",
      },
      alertes: {
        titre: "Une alerte quand un ami lance un jeu",
        texte: "Un bandeau discret apparaît dès qu'il commence à jouer.",
      },
      suivi: {
        titre: "Tes réglages te suivent",
        texte: "Tes revendeurs masqués sont les mêmes sur tous tes appareils, et ton temps de jeu aussi si tu le souhaites.",
      },
    },
    rassurer: "Gratuit et sans mot de passe : un code par e-mail suffit. Rien n'est partagé tant que tu ne l'as pas activé.",
    creer: "Créer mon compte",
    plusTard: "Plus tard",
    connecte: "C'est fait, {nom} !",
    connecteTexte: "Ton compte est prêt. Tu trouveras tous ses réglages dans Paramètres → Réseau Torii.",
    decouvrir: "Découvrir Torii",
  },
  visite: {
    bulle: "{n} sur {total}",
    terminer: "Terminer",
    bibliotheque: {
      titre: "Ta bibliothèque",
      texte: "Tous tes jeux réunis, quel que soit le launcher. Filtre par plateforme et lance n'importe quel jeu d'ici.",
    },
    boutique: {
      titre: "La Boutique",
      texte: "Compare le prix d'un jeu sur toutes les boutiques PC, et repère les vraies promos.",
    },
    wishlist: {
      titre: "Ta wishlist",
      texte: "Garde sous la main les jeux qui te tentent, où qu'ils soient vendus. Torii peut te prévenir quand leur prix baisse.",
    },
    amis: {
      titre: "Tes amis",
      texte: "Vois à quoi jouent tes amis et ce que vous avez en commun. Pour revoir ce tuto : Paramètres → Général.",
    },
  },
} as const;
