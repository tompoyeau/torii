/**
 * Steam, sans clé : transport, noms des tags, fiche `appdetails`.
 *
 * Mesuré le 01/11/2026 depuis Cloudflare : mêmes réponses que depuis un PC, et 30 `appdetails`
 * d'affilée sans un refus.
 */

import { DELAI_MS, chaine, entier, lireJson, memo, tableau } from "./lib.js";

export const QUERY = "https://api.steampowered.com/IStoreQueryService/Query/v1";
export const GET_ITEMS = "https://api.steampowered.com/IStoreBrowseService/GetItems/v1";
const TAGS = "https://store.steampowered.com/tagdata/populartags";
const IMAGES = "https://shared.akamai.steamstatic.com/store_item_assets/";

/** Pays du catalogue pour `appdetails` : aucun prix n'y est lu, et Steam masque des fiches selon le pays. */
const CATALOGUE = "us";

/** Tags jamais proposés comme catégories (contenus, appréciations, Accès anticipé). */
export const EXCLUS = new Set([4345, 4667, 6650, 12095, 9130, 1756, 4190, 5411, 3965, 1693, 10397, 493]);

/** Un JSON de Steam. `null` = panne, et ce qu'on en composera ne se mémorise pas. */
async function obtenir(ctx, url) {
  try {
    const reponse = await fetch(url, { signal: AbortSignal.timeout(DELAI_MS) });
    const donnees = reponse.ok ? lireJson(await reponse.text()) : null;
    if (donnees === null) ctx.partiel = true;
    return donnees;
  } catch (e) {
    console.error(`Steam injoignable : ${url.slice(0, 80)} —`, e);
    ctx.partiel = true;
    return null;
  }
}

/** Un appel à l'API de la boutique : la réponse entière (`response`), `null` si panne. */
export async function steamReponse(ctx, url, entree) {
  const racine = await obtenir(ctx, `${url}?input_json=${encodeURIComponent(JSON.stringify(entree))}`);
  return racine?.response ?? null;
}

/** Les jeux d'un appel à l'API de la boutique. */
export async function steamItems(ctx, url, entree) {
  const reponse = await steamReponse(ctx, url, entree);
  return reponse === null ? null : tableau(reponse.store_items);
}

export const contexteSteam = (ctx) => ({ language: ctx.steamLangue, country_code: ctx.region });

/**
 * La fiche `appdetails` d'un jeu : `data`, ou `null`.
 *
 * ⚠️ Steam peut répondre sous un AUTRE identifiant que celui demandé : quand il a regroupé
 * les éditions d'un jeu, `appids=2357570` (Overwatch) revient sous la clé `5026450`. Un seul
 * appid demandé = une seule entrée : on la prend, quelle que soit sa clé.
 */
export async function appdetails(ctx, appid) {
  const url = `https://store.steampowered.com/api/appdetails?appids=${appid}&l=${ctx.steamLangue}&cc=${CATALOGUE}`;
  const racine = await obtenir(ctx, url);
  if (racine === null || typeof racine !== "object") return null;
  const cles = Object.keys(racine);
  const entree = racine[String(appid)] ?? (cles.length === 1 ? racine[cles[0]] : null);
  return entree?.success === true ? (entree.data ?? null) : null;
}

// --- Noms des tags ----------------------------------------------------------------

/** Les noms changent rarement : une semaine de cache, et ce processus les garde en mémoire. */
const TTL_TAGS = 7 * 86_400;
const TAGS_EN_MEMOIRE = new Map();

/** Les tags de la langue courante : `noms` (id → nom) et `ordre` (les plus employés d'abord). */
export async function chargerTags(ctx) {
  const connus = TAGS_EN_MEMOIRE.get(ctx.steamLangue);
  if (connus) return connus;

  const texte = await memo(ctx, `steam/tags/${ctx.steamLangue}`, TTL_TAGS, async () => {
    const liste = await obtenir(ctx, `${TAGS}/${ctx.steamLangue}`);
    return Array.isArray(liste) && liste.length ? JSON.stringify(liste) : null;
  });
  const noms = new Map();
  const ordre = [];
  for (const t of tableau(texte === null ? null : lireJson(texte))) {
    const id = entier(t?.tagid);
    const nom = chaine(t?.name);
    if (id === null || nom === null) continue;
    noms.set(id, nom);
    ordre.push(id);
  }
  const tags = { noms, ordre };
  if (ordre.length) TAGS_EN_MEMOIRE.set(ctx.steamLangue, tags);
  return tags;
}

// --- Visuels et démos ---------------------------------------------------------------

/** Adresse complète du premier visuel présent parmi `cles`, dans `assets`. */
export function adresseAsset(assets, cles) {
  const format = chaine(assets?.asset_url_format);
  if (format === null) return null;
  const fichier = cles.map((c) => chaine(assets[c])).find((f) => f !== null);
  if (fichier === undefined) return null;
  return `${IMAGES}${format.replace("${FILENAME}", () => fichier)}`;
}

/** Les démos d'un jeu : Steam les donne d'office dans `related_items`, sans rien demander de plus. */
export const demos = (item) =>
  tableau(item?.related_items?.demo_appid)
    .map(entier)
    .filter((a) => a !== null);

/** Pour les tests : l'état de ce module survit d'un test à l'autre. */
export function oublierTags() {
  TAGS_EN_MEMOIRE.clear();
}
