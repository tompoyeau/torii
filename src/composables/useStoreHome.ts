import { reactive, ref } from "vue";
import { accueilDeDemo, rayonDeDemo } from "../data/storeMock";
import { langueCourante } from "../i18n";
import { regionCourante } from "../i18n/regions";
import { hasTauriRuntime, storeHome, storeShelf } from "../lib/tauri";
import type { StoreHome, StoreShelfItem, StoreShelfKind } from "../types";

/**
 * Accueil de la Boutique : vedette, rayons et classements (singleton).
 *
 * 🔑 GARDÉ EN MÉMOIRE POUR LA SESSION. Revenir à la Boutique ne relance rien pendant
 * 30 minutes : les classements de Steam et les promos ne bougent pas à la minute, et
 * chaque ouverture coûterait sinon plusieurs appels au comparateur, dont le quota est
 * partagé par tous les utilisateurs de Torii (clé unique, relayée par le proxy).
 *
 * ⚠️ La mémoire est indexée par région et langue : les rayons dépendent du pays (Steam
 * classe pays par pays) et de la langue (tags, descriptions). Changer l'un ou l'autre
 * recharge au prochain affichage.
 */

export type Etat = "vide" | "chargement" | "pret" | "echec";

const DUREE = 30 * 60_000;

const home = ref<StoreHome | null>(null);
const homeEtat = ref<Etat>("vide");
let homeCle = "";
let homeAt = 0;

interface Rayon {
  items: StoreShelfItem[];
  etat: Etat;
  at: number;
}
const rayons = reactive<Record<string, Rayon>>({});

const contexte = () => `${regionCourante.value}|${langueCourante.value}`;
const cleRayon = (kind: StoreShelfKind, tag?: number) => `${contexte()}|${kind}|${tag ?? ""}`;

/** Charge l'accueil, sauf s'il est encore frais. `force` : après un changement de revendeurs masqués. */
async function chargerAccueil(force = false) {
  const cle = contexte();
  const frais = homeCle === cle && Date.now() - homeAt < DUREE && homeEtat.value === "pret";
  if ((frais && !force) || homeEtat.value === "chargement") return;
  homeEtat.value = "chargement";
  // Hors Tauri (démo web) : relevé réel figé. Dans l'application, une panne s'affiche
  // comme telle — jamais de données de démo à la place de vrais prix.
  const res = hasTauriRuntime() ? await storeHome() : accueilDeDemo();
  if (contexte() !== cle) return; // la région a changé pendant l'appel
  if (res) {
    home.value = res;
    homeCle = cle;
    homeAt = Date.now();
    homeEtat.value = "pret";
  } else {
    homeEtat.value = "echec";
  }
}

/** Charge un rayon (onglet de classement, genre, free-to-play), sauf s'il est frais. */
async function chargerRayon(kind: StoreShelfKind, tag?: number, force = false) {
  const cle = cleRayon(kind, tag);
  const actuel = rayons[cle];
  if (actuel?.etat === "chargement") return;
  if (!force && actuel?.etat === "pret" && Date.now() - actuel.at < DUREE) return;
  rayons[cle] = { items: actuel?.items ?? [], etat: "chargement", at: actuel?.at ?? 0 };
  const res = hasTauriRuntime() ? await storeShelf(kind, tag) : rayonDeDemo(kind, tag);
  rayons[cle] = res
    ? { items: res, etat: "pret", at: Date.now() }
    : { items: actuel?.items ?? [], etat: "echec", at: actuel?.at ?? 0 };
}

/** L'état d'un rayon (vide tant qu'il n'a pas été demandé). */
function rayon(kind: StoreShelfKind, tag?: number): Rayon {
  return rayons[cleRayon(kind, tag)] ?? { items: [], etat: "vide", at: 0 };
}

/**
 * Les prix des rayons sont choisis côté Rust en tenant compte des revendeurs masqués :
 * ils deviennent faux quand la liste change. On recharge l'accueil et on périme les
 * rayons (rechargés à leur prochain affichage).
 */
function invalider() {
  homeAt = 0;
  for (const cle of Object.keys(rayons)) rayons[cle].at = 0;
  if (homeEtat.value !== "vide") void chargerAccueil(true);
}

export function useStoreHome() {
  return { home, homeEtat, chargerAccueil, chargerRayon, rayon, invalider };
}
