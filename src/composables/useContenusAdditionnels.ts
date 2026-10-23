import { reactive } from "vue";
import { langueCourante } from "../i18n";
import { regionCourante } from "../i18n/regions";
import { storeGameExtras } from "../lib/tauri";
import type { StoreShelfItem } from "../types";
import type { Etat } from "./useStoreHome";

/**
 * Contenus additionnels d'un jeu, avec leur meilleur prix, demandés PAR LOTS : la page
 * affichée seulement (5 contenus), jamais deux fois le même. Hunt: Showdown en compte plus
 * de 80 ; le comparateur, dont le quota est partagé par tous les utilisateurs de Torii,
 * n'en voit passer que ce qu'on feuillette.
 *
 * Commun à la fiche de la Boutique et à celle d'un jeu possédé : passer de l'une à l'autre
 * ne redemande rien. Gardé pour la session, par région et langue (les prix et les noms en
 * dépendent).
 */
interface Entree {
  etat: Etat;
  jeux: StoreShelfItem[];
  /** Appids déjà demandés (reçus ou en route). */
  demandes: number[];
  /** Demandés, mais que Steam ne décrit plus (contenu retiré de la vente…) : on les saute. */
  absents: number[];
}
const cache = reactive<Record<string, Entree>>({});

const cle = (appId: number) => `${regionCourante.value}|${langueCourante.value}|${appId}`;
const vide = (): Entree => ({ etat: "vide", jeux: [], demandes: [], absents: [] });

/** Demande les prix des contenus `ids` du jeu `appId` qui ne l'ont pas encore été. */
async function charger(appId: number, ids: number[]) {
  const k = cle(appId);
  const actuel = cache[k] ?? vide();
  const manquants = ids.filter((id) => !actuel.demandes.includes(id));
  if (!manquants.length) return;
  cache[k] = { ...actuel, etat: "chargement", demandes: [...actuel.demandes, ...manquants] };
  const res = await storeGameExtras(appId, manquants, []);
  const cur = cache[k];
  if (!res) {
    // Échec : on les rend à nouveau demandables, pour le bouton « Réessayer ».
    cache[k] = { ...cur, etat: "echec", demandes: cur.demandes.filter((id) => !manquants.includes(id)) };
    return;
  }
  const recus = new Set(res.dlc.map((j) => j.appId));
  cache[k] = {
    ...cur,
    etat: "pret",
    jeux: [...cur.jeux, ...res.dlc],
    absents: [...cur.absents, ...manquants.filter((id) => !recus.has(id))],
  };
}

function entree(appId: number): Entree {
  return cache[cle(appId)] ?? vide();
}

export function useContenusAdditionnels() {
  return { charger, entree };
}
