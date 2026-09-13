import type { Forme } from "../index";
import type fr from "../fr/boutique";

const en: Forme<typeof fr> = {
  titre: "Store",
  sousTitre: "Discover games and compare prices across every PC store.",
  rechercher: "Search for a game to buy…",
  effacer: "Clear",
  lancerRecherche: "Search",
  resultatsPour: "Results for “{requete}”",
  hasard: "Random pick",
  duMoment: "Trending now",
  auHasard: "Surprise me",
  auHasardAide: "Open a random game",
  pioche: "Picking…",
  tris: {
    featured: "Featured",
    savings: "Biggest discounts",
    price: "Lowest price",
    recent: "Recent",
    rating: "Top rated",
  },
  chargement: "Loading the store…",
  aucun: "No games found.",
  autreTitre: "Try another title.",
  ajouterWishlist: "Add to wishlist",
  retirerWishlist: "Remove from wishlist",

  produit: {
    fiche: "Product page",
    chargement: "Loading…",
    chargementFiche: "Loading product page…",
    meilleurPrix: "Best price",
    auLieuDe: "instead of {prix}",
    acheterChez: "Buy at {boutique}",
    dansWishlist: "On your wishlist",
    plusBasHistorique: "All-time low:",
    comparer: "Compare ({n} store) | Compare ({n} stores)",
    rupture: "Out of stock",
    masquerVendeur: "Hide this seller",
    reafficherVendeur: "Show this seller again",
    voirBoutique: "View in store",
    acheter: "Buy",
    vendeursMasques: "{n} hidden seller | {n} hidden sellers",
    avertissement:
      "Prices are indicative ({devise}). Use the eye icon to hide sellers you don't " +
      "want to see. Purchases happen on the seller's store.",
  },

  wishlist: {
    titre: "Wishlist",
    enPromo: "· {n} on sale",
    actualisation: "Refreshing…",
    rechercher: "Search…",
    rechercherAide: "Search your wishlist",
    actualiser: "Refresh",
    connecterSteam: "Connect your Steam account to track prices on your wishlist.",
    ouvrirReglages: "Open settings",
    recuperation: "Fetching prices for your wishlist…",
    premiereFois: "Takes a few seconds the first time.",
    vide: "Your Steam wishlist is empty.",
    aucunResultat: "Nothing on your wishlist matches “{requete}”.",
    jeuSteam: "Steam game",
    pasDOffre: "No offers yet",
    auPlusBas: "★ All-time low",
    auPlusBasAide: "The current price matches its all-time low",
    retire: "“{titre}” removed from your wishlist",
    ajouteSteam: "“{titre}” added to your wishlist, Steam included",
    ajoute: "“{titre}” added to your wishlist",
  },

  notifications: {
    baisse: "💸 Price drop",
    baisses: "💸 Price drops",
    plusieurs: "{n} games on your wishlist dropped in price (sale or all-time low).",
  },
};
export default en;
