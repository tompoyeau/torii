/**
 * Les listes du comparateur : vitrine, jeux offerts, recherche, autocomplétion.
 * Mêmes règles que `metadata/store.rs` dans l'application installée.
 */

import { cheapest, devise, estRecord, finOffre, getJson, prixGroupes } from "./itad.js";
import { chaine, num, tableau, urlencode } from "./lib.js";

const PAGE = 48;
const MAX_RESULTATS = 24;
const MAX_SUGGESTIONS = 8;
const MAX_OFFERTS = 8;

/** Traduit un critère de tri de la vitrine en `sort` ITAD. */
export function triItad(tri) {
  switch (tri) {
    case "savings":
      return "-cut";
    case "price":
      return "price";
    case "recent":
      return "-time";
    case "rating":
      return "-rating";
    default:
      return "-trending";
  }
}

/** Une carte de grille depuis un objet deal (`/deals/v2` `list[]`), `null` s'il est inexploitable. */
export function itemDepuisDeal(it) {
  // Le prix et la boutique sont sous `deal` ; les visuels au niveau du jeu.
  const offre = it?.deal ?? it;
  const prix = num(offre?.price?.amount);
  const gameId = chaine(it?.id);
  const title = chaine(it?.title);
  if (prix === null || gameId === null || title === null) return null;
  return {
    gameId,
    title,
    coverUrl: chaine(it.assets?.boxart),
    price: prix,
    normalPrice: num(offre.regular?.amount) ?? prix,
    currency: devise(offre.price),
    savings: Math.round(num(offre.cut) ?? 0),
    storeName: chaine(offre.shop?.name) ?? "",
    buyUrl: chaine(offre.url) ?? "",
    bannerUrl: chaine(it.assets?.banner600),
    record: estRecord(offre),
    expiry: finOffre(offre),
  };
}

/** Remplace l'offre d'une carte. `false` si l'offre n'a pas de prix lisible. */
export function appliquerOffre(item, offre) {
  const prix = num(offre?.price?.amount);
  if (prix === null) return false;
  item.price = prix;
  item.normalPrice = num(offre.regular?.amount) ?? prix;
  item.currency = devise(offre.price);
  item.savings = Math.round(num(offre.cut) ?? 0);
  item.storeName = chaine(offre.shop?.name) ?? "";
  item.buyUrl = chaine(offre.url) ?? "";
  item.record = estRecord(offre);
  item.expiry = finOffre(offre);
  return true;
}

/**
 * La vitrine impose UNE offre par jeu : si elle vient d'un revendeur masqué, on redemande
 * toutes les offres de ces jeux (un seul appel) et on retient la meilleure autorisée. Un jeu
 * qui n'a plus aucune offre visible sort de la liste.
 */
async function retarifer(ctx, items) {
  if (!ctx.exclus.size) return items;
  const ids = items.filter((i) => ctx.exclus.has(i.storeName)).map((i) => i.gameId);
  if (!ids.length) return items;
  const entrees = new Map((await prixGroupes(ctx, ids)).map((e) => [e?.id, e]));
  return items.filter((item) => {
    if (!ctx.exclus.has(item.storeName)) return true;
    const offre = cheapest(entrees.get(item.gameId), ctx.exclus);
    return offre !== null && appliquerOffre(item, offre);
  });
}

/** Vitrine : une page de jeux, selon le tri choisi. */
export async function deals(ctx, page, tri) {
  const chemin = `deals/v2?country=${ctx.region}&offset=${page * PAGE}&limit=${PAGE}&sort=${triItad(tri)}`;
  const racine = await getJson(ctx, chemin);
  if (racine === null) return [];
  const liste = Array.isArray(racine) ? racine : tableau(racine.list);
  const items = liste
    .filter((it) => it?.type !== "dlc")
    .map(itemDepuisDeal)
    .filter((i) => i !== null);
  return retarifer(ctx, items);
}

/**
 * Jeux offerts : seuls les JEUX à 0 € comptent. La même tête de liste contient des DLC, des
 * bandes-son et des démos à 0 € (`type` `dlc` ou nul), qui ne sont pas des cadeaux au sens où
 * l'entend un joueur.
 */
export function lireCadeaux(liste, exclus) {
  return tableau(liste)
    .filter((it) => it?.type === "game")
    .map(itemDepuisDeal)
    .filter((i) => i !== null && i.price === 0 && i.normalPrice > 0 && !exclus.has(i.storeName))
    .slice(0, MAX_OFFERTS);
}

export async function giveaways(ctx) {
  const racine = await getJson(ctx, `deals/v2?country=${ctx.region}&offset=0&limit=60&sort=price`);
  return lireCadeaux(racine?.list, ctx.exclus);
}

const resultats = (reponse) => (Array.isArray(reponse) ? reponse : tableau(reponse?.results));

/** Autocomplétion : titre et jaquette, sans prix, donc un seul appel léger. */
export async function suggest(ctx, requete) {
  const q = requete.trim();
  if (q.length < 2) return [];
  const trouves = await getJson(ctx, `games/search/v1?title=${urlencode(q)}`);
  return resultats(trouves)
    .filter((g) => g?.type !== "dlc" && chaine(g?.id) !== null && chaine(g?.title) !== null)
    .slice(0, MAX_SUGGESTIONS)
    .map((g) => ({ gameId: g.id, title: g.title, coverUrl: chaine(g.assets?.boxart) }));
}

/** Recherche par titre : les jeux, puis la meilleure offre de chacun en un seul appel. */
export async function search(ctx, requete) {
  const q = requete.trim();
  if (!q) return [];
  const trouves = await getJson(ctx, `games/search/v1?title=${urlencode(q)}`);
  const gardes = resultats(trouves)
    .filter((g) => g?.type !== "dlc")
    .slice(0, MAX_RESULTATS);
  const ids = gardes.map((g) => chaine(g?.id)).filter((i) => i !== null);
  if (!ids.length) return [];
  const meta = new Map(gardes.map((g) => [g?.id, g]));

  const items = [];
  for (const entree of await prixGroupes(ctx, ids)) {
    const id = chaine(entree?.id);
    const offre = cheapest(entree, ctx.exclus);
    const prix = num(offre?.price?.amount);
    if (id === null || prix === null) continue;
    const jeu = meta.get(id);
    items.push({
      gameId: id,
      title: chaine(jeu?.title) ?? "",
      coverUrl: chaine(jeu?.assets?.boxart),
      price: prix,
      normalPrice: num(offre.regular?.amount) ?? prix,
      currency: devise(offre.price),
      savings: Math.round(num(offre.cut) ?? 0),
      storeName: chaine(offre.shop?.name) ?? "",
      buyUrl: chaine(offre.url) ?? "",
      bannerUrl: null,
      record: estRecord(offre),
      expiry: finOffre(offre),
    });
  }
  return items;
}
