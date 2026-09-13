/** Prix et comparateur — partagés entre la boutique, la fiche produit et la wishlist. */
export default {
  gratuit: "Gratuit",
  inconnu: "Prix indisponible",
  plusBas: "Plus bas : {prix}",
  // 🔑 Affiché à la place d'Instant Gaming hors zone euro : mieux vaut expliquer une
  // absence que laisser croire à une panne.
  revendeurIndisponible:
    "Les prix des revendeurs de clés ne sont pas disponibles dans cette région.",
} as const;
