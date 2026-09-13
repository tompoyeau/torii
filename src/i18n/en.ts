/**
 * English catalogue — translation of `fr.ts`, which is the source of truth.
 *
 * ⚠️ KEYS ARE NEVER INVENTED HERE. Each zone file is typed `Forme<typeof fr zone>`:
 * a missing translation or a key that doesn't exist in French is a compile error. Add
 * the key to the French zone first, then translate it here.
 *
 * 🔑 THE TONE IS THE SAME, NOT THE WORDS. The French copy says « tes jeux », not « vos
 * jeux » — Torii talks to one person, plainly. English has no such choice to make, so
 * the equivalent is short sentences and no marketing register: "Your games", never
 * "Your gaming library experience".
 */
import amis from "./en/amis";
import bibliotheque from "./en/bibliotheque";
import boutique from "./en/boutique";
import commun from "./en/commun";
import demo from "./en/demo";
import comptes from "./en/comptes";
import fiche from "./en/fiche";
import prix from "./en/prix";
import reglages from "./en/reglages";
import systeme from "./en/systeme";

import type { Forme } from "./index";
import type fr from "./fr";

/** Typé contre le français : une zone entière oubliée ici ne compile pas. */
const en: Forme<typeof fr> = {
  amis,
  bibliotheque,
  boutique,
  commun,
  comptes,
  demo,
  fiche,
  prix,
  reglages,
  systeme,
};
export default en;
