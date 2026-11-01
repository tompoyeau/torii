/**
 * Le comparateur de prix (IsThereAnyDeal) : transport, pause après un refus, choix de la
 * meilleure offre, identifiants et prix groupés.
 *
 * 🔑 LA CLÉ ITAD EST UNE POUR TOUS LES UTILISATEURS DE TORII, APPLI PC COMPRISE. C'est la
 * ressource la plus fragile du service : une vingtaine d'appels en 4 s la saturent pour tout
 * le monde (mesuré le 01/11/2026). D'où les trois protections de ce fichier : un plafond
 * d'envoi (`RL_ITAD`), une pause après un refus, et le cache des appels en POST, que le proxy
 * ne met pas en cache.
 */

import { chaine, empreinte, lireJson, maintenant, memo, num, tableau, versProxy } from "./lib.js";

// --- Pause après un refus -----------------------------------------------------------

const PAUSE_DEFAUT = 60;

/** Fin de la pause connue de ce processus (secondes Unix). */
let pauseJusqua = 0;

const clePause = (ctx) => new Request(`${ctx.origin}/__cache/itad/pause`);

/**
 * Le comparateur est-il en pause ?
 *
 * ⚠️ Une variable ne suffit pas : Cloudflare fait tourner plusieurs processus, et celui qui
 * a reçu le refus n'est pas celui qui répondra à la requête suivante. La pause est donc
 * aussi posée dans le cache, partagé par tout le centre de données.
 */
async function enPause(ctx) {
  if (maintenant() < pauseJusqua) return true;
  const marque = await caches.default.match(clePause(ctx));
  if (!marque) return false;
  pauseJusqua = Number(await marque.text()) || 0;
  return maintenant() < pauseJusqua;
}

/** Retient un refus : plus d'appel avant `Retry-After` (60 s par défaut, de 10 s à 5 min). */
function noterRefus(ctx, reponse) {
  const annonce = Number.parseInt(reponse.headers.get("retry-after") ?? "", 10);
  const duree = Math.min(Math.max(Number.isNaN(annonce) ? PAUSE_DEFAUT : annonce, 10), 300);
  pauseJusqua = maintenant() + duree;
  console.warn(`comparateur saturé (429) : pause de ${duree} s`);
  const marque = new Response(String(pauseJusqua), {
    headers: { "cache-control": `public, max-age=${duree}` },
  });
  ctx.exec.waitUntil(caches.default.put(clePause(ctx), marque));
}

/** Pour les tests : l'état de ce module survit d'un test à l'autre. */
export function oublierEtat() {
  pauseJusqua = 0;
  IDS.clear();
}

// --- Transport ----------------------------------------------------------------------

/**
 * Un appel au comparateur. `null` = rien obtenu (pause, plafond, refus, panne) : l'appelant
 * fait sans, et `ctx.partiel` empêche de mémoriser ce qu'il composera.
 */
async function appel(ctx, chemin, init) {
  if (await enPause(ctx)) {
    ctx.partiel = true;
    return null;
  }
  if (ctx.env.RL_ITAD && !(await ctx.env.RL_ITAD.limit({ key: "itad" })).success) {
    ctx.partiel = true;
    return null;
  }
  try {
    const reponse = await versProxy(ctx, `itad/${chemin}`, init);
    if (reponse.status === 429) {
      noterRefus(ctx, reponse);
      ctx.partiel = true;
      return null;
    }
    const donnees = reponse.ok ? lireJson(await reponse.text()) : null;
    if (donnees === null) ctx.partiel = true;
    return donnees;
  } catch (e) {
    console.error(`comparateur injoignable : ${chemin} —`, e);
    ctx.partiel = true;
    return null;
  }
}

export const getJson = (ctx, chemin) => appel(ctx, chemin);

export const postJson = (ctx, chemin, corps) =>
  appel(ctx, chemin, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(corps),
  });

export const cheminPrix = (ctx) => `games/prices/v3?country=${ctx.region}`;

// --- Lire une offre -----------------------------------------------------------------

/** Nom de la boutique d'une offre. */
const boutique = (offre) => chaine(offre?.shop?.name) ?? "";

/**
 * Meilleure offre (prix mini) d'une entrée `games/prices/v3`, hors revendeurs masqués.
 *
 * 🔑 Le choix se fait ICI : la vitrine, la recherche et les rayons ne rendent qu'UN prix par
 * jeu, le front n'a donc aucun moyen de re-choisir. Sans ce filtre, un revendeur masqué
 * resterait meilleur prix partout sauf sur la fiche produit.
 */
export function cheapest(entree, exclus) {
  let meilleure = null;
  let prix = Infinity;
  for (const offre of tableau(entree?.deals)) {
    if (exclus.has(boutique(offre))) continue;
    const p = num(offre?.price?.amount) ?? Number.MAX_VALUE;
    if (meilleure === null || p < prix) {
      meilleure = offre;
      prix = p;
    }
  }
  return meilleure;
}

/** La devise d'un montant (`{ amount, amountInt, currency }`), vide si absente. */
export const devise = (montant) => chaine(montant?.currency) ?? "";

/**
 * Drapeau d'une offre : `H` = au plus bas historique, `N` = nouveau plus bas.
 *
 * ⚠️ Seulement s'il y a une remise. Un jeu qui vient de sortir est « au plus bas historique »
 * à plein tarif, et ITAD le marque `H` : mesuré sur Call of Duty à 69,99 €, -0 %. L'afficher
 * serait vrai et parfaitement trompeur. `S` (plus bas d'UNE boutique) n'est jamais un record.
 */
export const estRecord = (offre) =>
  (offre?.flag === "H" || offre?.flag === "N") && (num(offre?.cut) ?? 0) > 0;

/** Fin annoncée d'une offre, si le vendeur la donne. */
export const finOffre = (offre) => chaine(offre?.expiry) || null;

/** Les `name` d'une liste d'objets (`developers`, `publishers`, `drm`). */
export const noms = (liste) =>
  tableau(liste)
    .map((x) => chaine(x?.name))
    .filter((n) => n !== null);

// --- Identifiants et prix groupés ---------------------------------------------------

const ITAD_STEAM = 61;

/** Une correspondance appid → id ne bouge pas : un jour de cache, et les trouvés en mémoire. */
const TTL_IDS = 86_400;
const MAX_IDS_EN_MEMOIRE = 5_000;
const IDS = new Map();

/** Dix minutes : le front garde lui-même ses rayons 30 min, ce n'est pas ce qui les périmera. */
const TTL_PRIX = 600;
const TRANCHE_PRIX = 200;

/**
 * appid Steam → identifiant ITAD, en UN appel pour toute la liste.
 *
 * Un appid absent de la réponse n'est pas tranché et n'est pas rendu ; la réponse d'ITAD dit
 * `null` pour un jeu qu'il ne connaît pas.
 */
export async function idsItad(ctx, appids) {
  const resultat = new Map();
  const aDemander = [];
  for (const appid of new Set(appids)) {
    if (IDS.has(appid)) resultat.set(appid, IDS.get(appid));
    else aDemander.push(appid);
  }
  if (!aDemander.length) return resultat;

  aDemander.sort((a, b) => a - b);
  const corps = aDemander.map((a) => `app/${a}`);
  const cle = `itad/ids/${await empreinte(corps.join())}`;
  const texte = await memo(ctx, cle, TTL_IDS, async () => {
    const reponse = await postJson(ctx, `lookup/id/shop/${ITAD_STEAM}/v1`, corps);
    return reponse === null ? null : JSON.stringify(reponse);
  });
  const reponse = texte === null ? null : lireJson(texte);
  if (reponse === null) return resultat;

  if (IDS.size > MAX_IDS_EN_MEMOIRE) IDS.clear();
  for (const appid of aDemander) {
    const id = chaine(reponse[`app/${appid}`]);
    if (id === null) continue;
    IDS.set(appid, id);
    resultat.set(appid, id);
  }
  return resultat;
}

/** Les entrées `games/prices/v3` de ces jeux, par tranches de 200 (la limite d'ITAD). */
export async function prixGroupes(ctx, ids) {
  const entrees = [];
  for (let i = 0; i < ids.length; i += TRANCHE_PRIX) {
    const tranche = ids.slice(i, i + TRANCHE_PRIX);
    const cle = `itad/prix/${ctx.region}/${await empreinte([...tranche].sort().join())}`;
    const texte = await memo(ctx, cle, TTL_PRIX, async () => {
      const reponse = await postJson(ctx, cheminPrix(ctx), tranche);
      return reponse === null ? null : JSON.stringify(reponse);
    });
    if (texte !== null) entrees.push(...tableau(lireJson(texte)));
  }
  return entrees;
}
