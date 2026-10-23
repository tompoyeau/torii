import { reactive } from "vue";
import { langueCourante } from "../i18n";
import { regionCourante } from "../i18n/regions";
import { ownedGameSheet } from "../lib/tauri";
import type { Game, OwnedGameSheet } from "../types";
import type { Etat } from "./useStoreHome";

/**
 * Ce que Steam ajoute à la fiche d'un jeu de la bibliothèque (médias, notes, annonces…),
 * gardé pour la session : rouvrir une fiche ne redemande rien. Indexé par région et
 * langue, comme la fiche de la Boutique dont il reprend les données. Les prix des
 * contenus additionnels, eux, passent par `useContenusAdditionnels`, commun aux deux fiches.
 */
interface Entree {
  etat: Etat;
  fiche: OwnedGameSheet | null;
}
const fiches = reactive<Record<string, Entree>>({});

const cle = (id: string) => `${regionCourante.value}|${langueCourante.value}|${id}`;

async function charger(game: Game, steamAppid: string | null) {
  const k = cle(game.id);
  const actuel = fiches[k];
  if (actuel?.etat === "pret" || actuel?.etat === "chargement") return;
  fiches[k] = { etat: "chargement", fiche: actuel?.fiche ?? null };
  const fiche = await ownedGameSheet(game, steamAppid);
  // Rien du tout : pas de lien sûr, ou Steam n'a pas répondu. On ne le retient pas, pour
  // retenter à la prochaine ouverture plutôt que de laisser la fiche courte toute la session.
  const vide = !fiche || (!fiche.steam && !fiche.actualites.length);
  fiches[k] = { etat: vide ? "echec" : "pret", fiche: vide ? null : fiche };
}

function entree(id: string | null | undefined): Entree {
  return (id && fiches[cle(id)]) || { etat: "vide", fiche: null };
}

export function useFichePossede() {
  return { charger, entree };
}
