/** Bandeaux transverses (démo, mise à jour) et messages techniques montrés à l'écran. */
export default {
  // Commandes de la fenêtre, dessinées par Torii (`ControlesFenetre`).
  fenetre: {
    titre: "Fenêtre",
    reduire: "Réduire",
    agrandir: "Agrandir",
    restaurer: "Restaurer",
    fermer: "Fermer",
  },
  // Ton état, à droite de la bande du haut (`EtatJoueur`).
  etat: {
    enJeu: "En jeu",
    enLigne: "En ligne",
    invisible: "Invisible",
    synchro: "Synchronisation…",
    minutes: "{m} min",
    heures: "{h} h {m}",
    ouvrirFiche: "Ouvrir la fiche de {titre}",
    changer: "{etat} — changer ce que tes amis voient",
  },
  demo: {
    pastille: "Démo",
    texte: "Tu essaies Torii dans ton navigateur, sur une bibliothèque inventée.",
    limite: "Lancer un jeu ou connecter un compte demande l'application.",
    telecharger: "Télécharger Torii",
    masquer: "Masquer ce bandeau",
  },

  maj: {
    disponible: "Mise à jour disponible",
    pret: "Torii {version} est prêt à être installé.",
    installer: "Installer et redémarrer",
    plusTard: "Plus tard",
    telechargement: "Téléchargement de Torii {version}…",
    enCours: "En cours…",
    installee: "Mise à jour installée",
    redemarrage: "Redémarrage de Torii…",
    echec: "Échec de la mise à jour",
  },

  titrePage: "Torii — bibliothèque de jeux",
  commandeHorsApplication: "{commande} : indisponible hors de l'application Torii.",
  socialHorsApplication: "Le service Torii n'est disponible que dans l'application.",
} as const;
