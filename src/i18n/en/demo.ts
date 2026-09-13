import type { Forme } from "../index";
import type fr from "../fr/demo";

const en: Forme<typeof fr> = {
  description:
    "A striking, critically acclaimed experience. Explore a hand-crafted world, refine your playstyle over the hours and let the soundtrack carry you. Every session brings you closer to the end — or to a fresh start.",
  descriptionBoutique:
    "A critically acclaimed title. Explore a hand-crafted world, refine your style over the hours and let its art direction carry you.",
  studioFictif: "Made-up Studio",
  sims4: "The Sims 4",

  genres: {
    action: "Action",
    actionAventure: "Action-adventure",
    actionRpg: "Action RPG",
    aventure: "Adventure",
    bacASable: "Sandbox",
    cooperatif: "Co-op",
    deckBuilding: "Deck-building",
    gestion: "Management",
    horreur: "Horror",
    jeuDeRole: "Role-playing",
    metroidvania: "Metroidvania",
    plateforme: "Platformer",
    rogueLite: "Roguelite",
    reflexion: "Puzzle",
    simulation: "Simulation",
    tir: "Shooter",
  },

  succes: {
    nom1: "Escape From Avernus",
    desc1: "Take control of the nautiloid and escape the Hells.",
    date1: "Unlocked Aug 30, 2023 @ 10:28am",
    nom2: "Out of the Frying Pan",
    desc2: "Leave Act One for somewhere far darker.",
    date2: "Unlocked Nov 19, 2023 @ 8:24am",
    nom3: "The City Awaits",
    desc3: "Leave Act Two and head for Baldur's Gate.",
    date3: "Unlocked Jan 26, 2024 @ 2:31pm",
    nom4: "All's Well That Ends Well",
    desc4: "Finish the game.",
    nom5: "The Call of Blood",
    desc5: "Drink the blood of a defeated enemy.",
  },
};
export default en;
