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
    apropos: "À propos & maintenance",
    masques: "Jeux masqués",
    revendeurs: "Revendeurs masqués",
    comptes: "Comptes & launchers",
    torii: "Réseau Torii",
  },

  general: {
    autostart: {
      titre: "Lancer au démarrage de Windows",
      sous: "Torii s'ouvrira automatiquement à l'ouverture de ta session.",
    },
    minimise: {
      titre: "Démarrer minimisé",
      sous: "Se lance réduit dans la zone de notification (à côté de l'horloge).",
    },
    tray: {
      titre: "Fermer réduit dans la zone de notification",
      sous:
        "Activé par défaut : Torii continue de repérer tes parties une fois la fenêtre " +
        "fermée. Décoche pour que la croix quitte vraiment l'application — « Quitter » " +
        "reste disponible par clic droit sur l'icône près de l'horloge.",
    },
    retourJeu: {
      titre: "Revenir à la fermeture d'un jeu",
      sous:
        "Torii se minimise au lancement d'un jeu, puis revient au premier plan sur sa " +
        "fiche quand tu le fermes. (Jeux installés lancés depuis Torii.)",
    },
    theme: {
      titre: "Thème",
      sous: "Apparence de l'application.",
      systeme: "Système",
      clair: "Clair",
      sombre: "Sombre",
    },
    vue: {
      titre: "Vue par défaut",
      sous: "Filtre, tri et affichage au démarrage de la bibliothèque.",
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
      sous: "Taille des jaquettes dans la grille.",
      compact: "Compact",
      normal: "Normal",
      grand: "Grand",
    },
    animations: {
      titre: "Réduire les animations",
      sous: "Désactive les transitions et effets (accessibilité, machines modestes).",
    },
    alertesPrix: {
      titre: "Alertes de prix (wishlist)",
      sous:
        "Une notification quand un jeu de ta wishlist Steam passe en promo ou atteint son " +
        "plus bas prix historique. (Torii doit tourner, même réduit dans le tray.)",
    },
  },

  langue: {
    titre: "Langue",
    description:
      "La langue de l'interface, et celle dans laquelle Torii demande les descriptions et " +
      "les genres aux boutiques.",
    systeme: "Suivre Windows",
    // 🔑 Affiché seulement après un changement. L'interface a déjà basculé ; ce qui ne
    // suit pas, ce sont les fiches déjà chargées en mémoire. Le dire évite de croire que
    // la traduction a « à moitié marché ».
    appliquer:
      "Les descriptions et les genres des jeux déjà affichés changeront de langue au " +
      "prochain démarrage.",
    redemarrer: "Redémarrer maintenant",
  },

  region: {
    titre: "Région",
    description:
      "Le pays utilisé pour les prix : il détermine la devise et les montants affichés " +
      "dans la Boutique et la wishlist.",
    // 🔑 Le réglage le plus susceptible d'être mal compris : beaucoup s'attendent à ce que
    // changer la langue change aussi les prix. On l'écrit noir sur blanc.
    note: "Indépendante de la langue : on peut lire Torii en anglais avec des prix en euros.",
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
        "Supprime les métadonnées, jaquettes et prix mis en cache (re-téléchargés au " +
        "besoin). N'affecte ni tes comptes ni tes favoris.",
      bouton: "Vider le cache",
      enCours: "Nettoyage…",
      indisponible: "Indisponible hors de l'application.",
      vide:
        "Cache vidé ({n} fichier). Les données seront re-téléchargées au besoin. | " +
        "Cache vidé ({n} fichiers). Les données seront re-téléchargées au besoin.",
    },
    journal: {
      titre: "Journal de l'application",
      sous:
        "Démarrages, erreurs et incidents. À joindre si tu signales un problème : c'est ce " +
        "qui permet de comprendre ce qui s'est passé sur ta machine.",
      bouton: "Ouvrir le journal",
    },
    signaler: {
      titre: "Signaler un problème",
      sous:
        "Ouvre un rapport pré-rempli avec ta version de Torii. C'est le seul moyen qu'un " +
        "bug arrive jusqu'à quelqu'un qui peut le corriger.",
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
    sous: "Les jeux masqués sont retirés de la bibliothèque. Réaffiche-les ici.",
    reafficher: "Réafficher",
    aucun: "Aucun jeu masqué.",
  },

  revendeurs: {
    sous: "Boutiques masquées dans le comparatif de prix de la Boutique.",
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
        "Le nom que voient tes amis. Il n'a pas besoin d'être unique et ne permet à " +
        "personne de te retrouver — seul ton code d'ami le permet.",
      misAJour: "Pseudo mis à jour.",
    },

    code: {
      titre: "Ton code d'ami",
      sous:
        "À donner de la main à la main pour qu'on t'ajoute. Le renouveler rend l'ancien " +
        "inutilisable — pratique si tu l'as diffusé trop largement.",
      renouveler: "Renouveler",
      renouvele: "Nouveau code d'ami : l'ancien ne fonctionne plus.",
    },

    presence: {
      titre: "Ce que tes amis voient",
      sous: "Tant que tu es invisible, rien de ce que tu joues ne quitte ton PC.",
      detaille: "Jeu visible",
      detailleAide: "Tes amis voient à quoi tu joues et depuis quand.",
      enLigne: "En ligne",
      enLigneAide: "Ils te savent connecté, sans savoir à quoi tu joues.",
      invisible: "Invisible",
      invisibleAide: "Personne ne voit rien. Tu vois toujours tes amis.",
    },

    absent: {
      titre: "Passer « absent » après",
      sous: "Sans action au clavier ni à la souris.",
      minutes: "{n} min",
    },

    steam: {
      titre: "Visible par mes amis Steam",
      sous:
        "Permet à tes amis Steam déjà sur Torii de te retrouver, et de fusionner ta fiche " +
        "avec ton profil Steam. Il faut que vous l'ayez activé tous les deux.",
      sousSansSteam:
        "Connecte d'abord ton compte Steam dans « Comptes & launchers » : sans lui, il n'y " +
        "a rien à rapprocher.",
      retrouver: "Retrouver mes amis Steam",
      retrouverAide:
        "Torii compare ta liste d'amis Steam aux comptes existants. Seuls ceux qui ont eux " +
        "aussi activé cette option apparaissent — c'est ce qui empêche de s'en servir pour " +
        "savoir qui utilise Torii.",
      chercher: "Chercher parmi mes amis Steam",
      surSteam: "{nom} sur Steam",
      aucun: "Aucun de tes amis Steam n'a de compte Torii visible pour l'instant.",
    },

    bibliotheque: {
      titre: "Ma bibliothèque",
      aide:
        "Ce que tu possèdes, déposé sur le serveur pour le retrouver sur tes autres " +
        "appareils — et le montrer à tes amis, quel que soit le launcher. Les jeux masqués " +
        "et ceux marqués « ne pas diffuser » n'en font jamais partie.",
      synchro: "Synchroniser ma bibliothèque",
      synchroSous:
        "Elle part après un scan, et seulement si elle a changé depuis la dernière fois. " +
        "La couper l'efface du serveur.",
      partage: "Visible par mes amis Torii",
      partageSous:
        "Tes amis voient ce que tu possèdes, Steam ou non. Éteint, ta bibliothèque ne sert " +
        "qu'à toi et à tes propres appareils.",
      partageSansSynchro:
        "Active d'abord la synchronisation : sans elle, il n'y a rien à montrer.",
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
        "Tes amis voient l'ensemble de tes appareils comme une seule bibliothèque. Retirer " +
        "un vieux PC efface la sienne du serveur.",
      ligneAppareil: "{n} jeu · {quand} | {n} jeux · {quand}",
    },

    notifAmi: {
      titre: "Me prévenir quand un ami lance un jeu",
      sous:
        "Un bandeau s'affiche quelques secondes en haut à droite de l'écran, sans prendre " +
        "le focus. Un jeu en plein écran exclusif peut le masquer.",
    },

    silence: {
      titre: "Jeux jamais diffusés",
      aide:
        "Ces jeux n'apparaissent jamais dans ta présence, même en cours de partie. Utile " +
        "pour les applications qui tournent en permanence.",
      rediffuser: "Diffuser à nouveau",
      aucun: "Aucun jeu masqué. Fais un clic droit sur un jeu pour l'ajouter.",
    },

    appareils: {
      titre: "Mes appareils",
      aide:
        "Les machines où ce compte Torii est connecté. Une session inutilisée pendant six " +
        "mois tombe d'elle-même, mais si tu ne reconnais pas un appareil, déconnecte-le " +
        "tout de suite : c'est immédiat et sans appel.",
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
        "Ton pseudo, ton code d'ami et toutes tes relations disparaissent du serveur. Tes " +
        "amis ne te verront plus dans leur liste. Ta bibliothèque et tes réglages restent " +
        "sur cet ordinateur : seul le compte Torii est supprimé.",
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
} as const;
