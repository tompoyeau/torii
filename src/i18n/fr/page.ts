/** Page web intégrée (`PageWeb.vue`) : un site ouvert dans Torii plutôt que dans le navigateur. */
export default {
  titre: "Page web",
  precedente: "Page précédente",
  suivante: "Page suivante",
  recharger: "Recharger la page",
  navigateur: "Ouvrir dans le navigateur",
  fermer: "Fermer la page",
  // `lieu` = l'écran de Torii où l'on revient (« Boutique », un jeu…).
  fermerVers: "Fermer la page et revenir à : {lieu}",
  // Infobulle du cadenas : c'est Torii qui l'affiche, pas le site.
  securise: "Connexion chiffrée avec {domaine}",
  chargement: "Chargement de la page…",
  echec: "Torii n'a pas pu ouvrir cette page : {erreur}",
} as const;
