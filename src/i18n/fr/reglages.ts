/** Paramètres (`SettingsView.vue`). */
export default {
  titre: "Paramètres",

  groupes: {
    application: "Application",
    bibliotheque: "Bibliothèque & Boutique",
    comptes: "Comptes",
  },

  categories: {
    general: "Paramètres généraux",
    raccourcis: "Raccourcis clavier",
    apropos: "À propos & maintenance",
    masques: "Jeux masqués",
    revendeurs: "Revendeurs masqués",
    comptes: "Comptes & launchers",
    torii: "Réseau Torii",
  },

  general: {
    autostart: {
      titre: "Lancer au démarrage de Windows",
      sous: "Torii s'ouvre tout seul quand tu allumes ton PC.",
    },
    minimise: {
      titre: "Démarrer minimisé",
      sous: "Torii démarre en arrière-plan, sans ouvrir sa fenêtre.",
    },
    tray: {
      titre: "Fermer réduit dans la zone de notification",
      sous:
        "La croix range Torii près de l'horloge au lieu de le quitter, pour qu'il continue de compter tes parties.",
    },
    retourJeu: {
      titre: "Revenir à la fermeture d'un jeu",
      sous:
        "Torii se fait discret pendant que tu joues, puis revient sur la fiche du jeu quand tu le quittes.",
    },
    theme: {
      titre: "Thème",
      sous: "Clair, sombre, ou comme Windows.",
      systeme: "Système",
      clair: "Clair",
      sombre: "Sombre",
    },
    vue: {
      titre: "Vue par défaut",
      sous: "Ce que la bibliothèque affiche à l'ouverture.",
      tous: "Tous",
      favoris: "Favoris",
      installes: "Installés",
      recent: "Récemment joué",
      alpha: "A → Z",
      tempsDeJeu: "Temps de jeu",
      grille: "Grille",
      liste: "Liste",
    },
    densite: {
      titre: "Densité de la bibliothèque",
      sous: "La taille des jaquettes.",
      compact: "Compact",
      normal: "Normal",
      grand: "Grand",
    },
    animations: {
      titre: "Réduire les animations",
      sous: "Moins de mouvements à l'écran.",
    },
    tuto: {
      titre: "Tuto de bienvenue",
      sous: "Refais la visite guidée du premier lancement.",
      bouton: "Revoir le tuto",
    },
    alertesPrix: {
      titre: "Alertes de prix (wishlist)",
      sous:
        "Une notification quand un jeu de ta wishlist passe en promo ou à son plus bas prix.",
    },
  },

  langue: {
    titre: "Langue",
    description:
      "La langue de Torii et des descriptions de jeux.",
    systeme: "Suivre Windows",
    // 🔑 Affiché seulement après un changement. L'interface a déjà basculé ; ce qui ne
    // suit pas, ce sont les fiches déjà chargées en mémoire. Le dire évite de croire que
    // la traduction a « à moitié marché ».
    appliquer:
      "Les fiches déjà ouvertes changeront de langue au prochain lancement.",
    redemarrer: "Redémarrer maintenant",
  },

  region: {
    titre: "Région",
    description:
      "Le pays de tes prix : il fixe la devise dans la Boutique et ta wishlist.",
    // 🔑 Le réglage le plus susceptible d'être mal compris : beaucoup s'attendent à ce que
    // changer la langue change aussi les prix. On l'écrit noir sur blanc.
    note: "Indépendante de la langue : Torii en anglais, prix en euros, c'est possible.",
    systeme: "Suivre Windows ({pays})",
    devise: "Devise : {devise}",
  },

  apropos: {
    version: "Version de Torii",
    maj: {
      verification: "Vérification…",
      disponible: "Mise à jour disponible : {version}",
      telechargement: "Téléchargement…",
      installee: "Installée — redémarrage…",
      erreur: "Erreur de vérification.",
      aJour: "Torii est à jour.",
    },
    verifier: "Vérifier les mises à jour",
    installer: "Installer maintenant",
    cache: {
      titre: "Vider le cache",
      sous:
        "Libère de la place, ou répare une jaquette ou un prix qui ne se met pas à jour. Tes comptes et tes favoris ne bougent pas.",
      bouton: "Vider le cache",
      enCours: "Nettoyage…",
      indisponible: "Indisponible hors de l'application.",
      vide:
        "Cache vidé.",
    },
    journal: {
      titre: "Journal de l'application",
      sous:
        "À joindre si tu signales un problème.",
      bouton: "Ouvrir le journal",
    },
    signaler: {
      titre: "Signaler un problème",
      sous:
        "Raconte ce qui ne va pas : ta version de Torii est ajoutée toute seule.",
      bouton: "Signaler",
      // Corps du rapport GitHub. Lu par le développeur, mais écrit par l'utilisateur :
      // les intitulés doivent être dans SA langue, sinon il ne sait pas quoi remplir.
      rapport: {
        cequisepasse: "**Ce qui se passe**",
        attendu: "**Ce que tu attendais**",
        reproduire: "**Comment le reproduire**",
        demo: "Démo web (pas d'installation)",
        journal1: "Pense à joindre le journal : Paramètres → À propos & maintenance →",
        journal2: "« Ouvrir le journal », puis copie son contenu ici.",
      },
    },
  },

  masques: {
    sous: "Les jeux que tu as retirés de ta bibliothèque.",
    reafficher: "Réafficher",
    aucun: "Aucun jeu masqué.",
  },

  revendeurs: {
    sous: "Les boutiques que tu ne veux plus voir dans les prix.",
    reafficher: "Réafficher",
    toutReafficher: "Tout réafficher",
    aucun: "Aucun revendeur masqué. Tu peux en masquer depuis la fiche d'un jeu dans la Boutique.",
  },

  torii: {
    intro:
      "Voir à quoi jouent tes amis, quel que soit leur launcher — et leur montrer ce que " +
      "tu joues, si tu le décides.",

    pseudo: {
      titre: "Ton pseudo",
      sous:
        "Le nom que voient tes amis.",
      misAJour: "Pseudo mis à jour.",
    },

    code: {
      titre: "Ton code d'ami",
      sous:
        "Donne-le à tes amis pour qu'ils t'ajoutent. Le renouveler désactive l'ancien.",
      renouveler: "Renouveler",
      renouvele: "Nouveau code d'ami : l'ancien ne fonctionne plus.",
    },

    presence: {
      titre: "Ce que tes amis voient",
      sous: "Invisible, tes amis ne voient rien de ce que tu fais.",
      detaille: "Jeu visible",
      detailleAide: "Tes amis voient à quoi tu joues et depuis quand.",
      enLigne: "En ligne",
      enLigneAide: "Ils te savent connecté, sans savoir à quoi tu joues.",
      invisible: "Invisible",
      invisibleAide: "Personne ne voit rien. Tu vois toujours tes amis.",
    },

    absent: {
      titre: "Passer « absent » après",
      sous: "Quand tu ne touches plus au clavier ni à la souris.",
      minutes: "{n} min",
    },

    steam: {
      titre: "Visible par mes amis Steam",
      sous:
        "Tes amis Steam qui utilisent Torii peuvent te retrouver, s'ils l'ont activé eux aussi.",
      sousSansSteam:
        "Connecte d'abord ton compte Steam dans « Comptes & launchers ».",
      retrouver: "Retrouver mes amis Steam",
      retrouverAide:
        "Seuls ceux qui ont aussi activé cette option apparaissent.",
      chercher: "Chercher parmi mes amis Steam",
      surSteam: "{nom} sur Steam",
      aucun: "Aucun de tes amis Steam n'a de compte Torii visible pour l'instant.",
    },

    bibliotheque: {
      titre: "Ma bibliothèque",
      aide:
        "Retrouve tes jeux sur ton téléphone et sur le web, et montre-les à tes amis si tu veux. Les jeux masqués n'y sont jamais.",
      synchro: "Synchroniser ma bibliothèque",
      synchroSous:
        "Elle se met à jour toute seule. La couper efface ta bibliothèque en ligne.",
      partage: "Visible par mes amis Torii",
      partageSous:
        "Tes amis voient tes jeux, de tous tes launchers.",
      tempsDeJeu: "Mon temps de jeu sur mes autres appareils",
      tempsDeJeuSous:
        "Ton temps de jeu et tes dernières parties, sur le web et ton téléphone. Tes amis ne les voient pas.",
      partageSansSynchro:
        "Active d'abord la synchronisation.",
      envoye: "{n} jeu envoyé depuis « {appareil} », {quand}. | {n} jeux envoyés depuis « {appareil} », {quand}.",
      rienEnvoye: "Rien n'a encore été envoyé depuis cet appareil.",
      maintenant: "Synchroniser maintenant",
      envoi: "Envoi…",
      activee: "Bibliothèque synchronisée.",
      coupee: "Synchronisation coupée, bibliothèque effacée du serveur.",
      envoyee: "Bibliothèque envoyée ({n} jeu). | Bibliothèque envoyée ({n} jeux).",
      rienAEnvoyer: "Rien à envoyer.",
      autres: "Mes autres appareils",
      autresAide:
        "Retirer un ancien PC efface sa bibliothèque en ligne.",
      ligneAppareil: "{n} jeu · {quand} | {n} jeux · {quand}",
    },

    notifAmi: {
      titre: "Me prévenir quand un ami lance un jeu",
      sous:
        "Un petit bandeau quelques secondes, en haut à droite de l'écran.",
    },

    silence: {
      titre: "Jeux jamais diffusés",
      aide:
        "Ces jeux n'apparaissent jamais dans ce que voient tes amis. Pratique pour les applis toujours ouvertes.",
      rediffuser: "Diffuser à nouveau",
      aucun: "Aucun jeu ici. Fais un clic droit sur un jeu pour l'ajouter.",
    },

    appareils: {
      titre: "Mes appareils",
      aide:
        "Les appareils connectés à ton compte. Si tu n'en reconnais pas un, déconnecte-le.",
      celuiCi: "cet appareil",
      connecte: "Connecté {quand}",
      actif: "actif {quand}",
      deconnecter: "Déconnecter",
      aucun: "Aucun appareil connecté à afficher.",
      deconnecterTous: "Déconnecter tous les autres appareils",
      confirmationTous:
        "L'autre appareil devra se reconnecter. | Les {n} autres appareils devront se reconnecter.",
      deconnecte: "« {nom} » a été déconnecté.",
      deconnectes: "L'autre appareil a été déconnecté. | {n} appareils déconnectés.",
      deconnecterCompte: "Déconnecter ce compte",
    },

    suppression: {
      titre: "Supprimer mon compte",
      aide:
        "Ton compte, ton pseudo et tes amis disparaissent. Tes jeux et tes réglages restent sur ce PC.",
      bouton: "Supprimer mon compte Torii",
      // ⚠️ Découpé autour du pseudo, qui est rendu en gras par le gabarit : on ne peut pas
      // mettre de balise dans une traduction (pas de `v-html`, volontairement).
      avant: "C'est définitif : il n'y a pas de corbeille, et le même code d'ami ne reviendra pas. Recopie",
      apres: "pour confirmer.",
      enCours: "Suppression…",
      definitif: "Supprimer définitivement",
      fait: "Ton compte Torii a été supprimé.",
    },
  },

  raccourcis: {
    sous:
      "Torii se pilote aussi au clavier.",
    // Infobulle d'un bouton : « Paramètres (Ctrl+,) ».
    infobulle: "{action} ({touches})",
    // Entre deux raccourcis équivalents : « F5 ou Ctrl + R ».
    ou: "ou",
    groupes: {
      navigation: "Se déplacer",
      actions: "Actions",
      medias: "Bandes-annonces et captures",
    },
    actions: {
      precedent: "Revenir à l'écran précédent",
      suivant: "Aller à l'écran suivant",
      fermer: "Fermer la fiche ou la fenêtre ouverte",
      bibliotheque: "Bibliothèque",
      boutique: "Boutique",
      wishlist: "Wishlist",
      amis: "Contacts",
      rechercher: "Aller dans la barre de recherche",
      ajouter: "Ajouter un jeu manuellement",
      resynchroniser: "Resynchroniser la bibliothèque",
      parametres: "Ouvrir les paramètres",
      raccourcis: "Afficher ces raccourcis",
      capture: "Capture précédente ou suivante",
      lecture: "Lecture / pause",
      avanceVideo: "Reculer ou avancer de 5 secondes",
      volume: "Monter ou baisser le volume",
      muet: "Couper le son",
      pleinEcran: "Vidéo en plein écran",
    },
    touches: {
      ctrl: "Ctrl",
      alt: "Alt",
      echap: "Échap",
      espace: "Espace",
      sourisRetour: "Bouton retour de la souris",
      sourisAvant: "Bouton avant de la souris",
    },
  },
} as const;
