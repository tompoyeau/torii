/** Mots partagés par plusieurs écrans. N'y mettre que ce qui sert vraiment à deux endroits. */
export default {
  annuler: "Annuler",
  fermer: "Fermer",
  enregistrer: "Enregistrer",
  confirmer: "Confirmer",
  retour: "Retour",
  chargement: "Chargement…",
  reessayer: "Réessayer",
  oui: "Oui",
  non: "Non",
  active: "Activé",
  desactive: "Désactivé",
  aLInstant: "à l'instant",
  recherche: "Recherche…",
  ajouter: "Ajouter",
  retirer: "Retirer",
  supprimer: "Supprimer",
  // ⚠️ Le « … » seul, pour un bouton dont le libellé disparaît pendant l'action : on
  // garde la clé plutôt qu'un littéral pour que la ponctuation reste celle de la langue.
  enCours: "…",
  // Partager un lien : fiches de jeu (bibliothèque, Boutique) et page web intégrée.
  partage: {
    partager: "Partager",
    aideJeu: "Copier un lien vers ce jeu, à envoyer à un ami",
    copierLien: "Copier le lien de la page",
    copie: "Lien copié : colle-le où tu veux.",
    impossible: "Copie impossible.",
    aucun: "Pas de page à partager pour ce jeu.",
  },
  // Le menu des catégories (tags Steam) : bibliothèque et Boutique.
  categories: {
    bouton: "Catégories",
    chercher: "Chercher une catégorie",
    aucune: "Aucune catégorie ne correspond.",
    effacer: "Tout effacer",
    toutes: "Les jeux doivent avoir toutes les catégories cochées.",
  },
} as const;
