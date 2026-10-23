import { reactive } from "vue";
import { langueCourante } from "../i18n";
import { regionCourante } from "../i18n/regions";
import { hasTauriRuntime, storeGameExtras } from "../lib/tauri";
import type { StoreExtras, StoreGame } from "../types";
import type { Etat } from "./useStoreHome";

/**
 * Jeux similaires du bas de la fiche produit, gardés pour la session : revenir sur une
 * fiche déjà parcourue ne redemande rien au comparateur. Indexé par région et langue,
 * comme l'accueil. Les contenus additionnels ont leur propre chargement, page par page
 * (`useContenusAdditionnels`).
 */
interface Entree {
  etat: Etat;
  extras: StoreExtras | null;
}
const cache = reactive<Record<string, Entree>>({});

const cle = (gameId: string) => `${regionCourante.value}|${langueCourante.value}|${gameId}`;

/** Demande le bas de fiche d'un jeu, s'il n'est pas déjà là (ou en route). */
async function charger(jeu: StoreGame, force = false) {
  const k = cle(jeu.gameId);
  const actuel = cache[k];
  if (!force && (actuel?.etat === "pret" || actuel?.etat === "chargement")) return;
  const steam = jeu.steam;
  // Sans fiche Steam (jeu absent de Steam, ou démo web) : pas de tags à chercher.
  if (!steam || !jeu.appId || !hasTauriRuntime()) {
    cache[k] = { etat: "pret", extras: null };
    return;
  }
  cache[k] = { etat: "chargement", extras: actuel?.extras ?? null };
  // Aucun contenu additionnel ici : la recherche de Steam ne ramène que des jeux, et les
  // contenus sont demandés à part, page par page.
  const res = await storeGameExtras(jeu.appId, [], steam.tagIds);
  cache[k] = res ? { etat: "pret", extras: res } : { etat: "echec", extras: null };
}

function entree(gameId: string): Entree {
  return cache[cle(gameId)] ?? { etat: "vide", extras: null };
}

export function useFicheExtras() {
  return { charger, entree };
}
