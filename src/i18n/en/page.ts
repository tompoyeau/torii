import type { Forme } from "../index";
import type fr from "../fr/page";

const en: Forme<typeof fr> = {
  titre: "Web page",
  precedente: "Previous page",
  suivante: "Next page",
  recharger: "Reload the page",
  navigateur: "Open in browser",
  fermer: "Close the page",
  fermerVers: "Close the page and go back to {lieu}",
  securise: "Encrypted connection to {domaine}",
  chargement: "Loading the page…",
  echec: "Torii couldn't open this page: {erreur}",
};
export default en;
