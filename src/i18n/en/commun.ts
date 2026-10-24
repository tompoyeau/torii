import type { Forme } from "../index";
import type fr from "../fr/commun";

const en: Forme<typeof fr> = {
  annuler: "Cancel",
  fermer: "Close",
  enregistrer: "Save",
  confirmer: "Confirm",
  retour: "Back",
  chargement: "Loading…",
  reessayer: "Try again",
  oui: "Yes",
  non: "No",
  active: "On",
  desactive: "Off",
  aLInstant: "just now",
  recherche: "Searching…",
  ajouter: "Add",
  retirer: "Remove",
  supprimer: "Delete",
  enCours: "…",
  partage: {
    partager: "Share",
    aideJeu: "Copy a link to this game to send to a friend",
    copierLien: "Copy page link",
    copie: "Link copied — paste it anywhere.",
    impossible: "Couldn't copy.",
    aucun: "No page to share for this game.",
  },
  categories: {
    bouton: "Categories",
    chercher: "Search categories",
    aucune: "No matching category.",
    effacer: "Clear all",
    toutes: "Games must have every checked category.",
  },
};
export default en;
