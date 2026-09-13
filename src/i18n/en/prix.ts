import type { Forme } from "../index";
import type fr from "../fr/prix";

const en: Forme<typeof fr> = {
  gratuit: "Free",
  inconnu: "Price unavailable",
  plusBas: "Lowest: {prix}",
  revendeurIndisponible: "Key reseller prices are not available in this region.",
};
export default en;
