/**
 * Fiche produit : comparatif multi-boutiques + plus bas historiques + IGDB + Instant Gaming +
 * fiche Steam. Mêmes règles que `store::game` dans l'application installée.
 *
 * 🔑 Une fois le titre et l'appid connus (`games/info/v2`), tout le reste part EN PARALLÈLE : la
 * fiche n'attend que le plus lent, elle n'additionne pas les allers-retours.
 */

import { ficheSteam } from "./fiche.js";
import { metaSteam, metaTitre } from "./igdb.js";
import { offreInstantGaming } from "./instant_gaming.js";
import { cheminPrix, devise, finOffre, getJson, noms, postJson } from "./itad.js";
import { chaine, entier, num, tableau, urlencode, zoneEuro } from "./lib.js";
import { appdetails } from "./steam.js";

const CDN = "https://cdn.cloudflare.steamstatic.com/steam/apps";

/** Types Steam dont la description ne doit pas passer pour celle d'un jeu. */
const PAS_UN_JEU = new Set(["dlc", "music", "video", "demo", "hardware", "mod", "episode"]);

/** Une ligne du comparatif, depuis une offre `games/prices/v3`. */
export function offreFiche(d) {
  const prix = num(d?.price?.amount);
  if (prix === null) return null;
  return {
    storeName: chaine(d.shop?.name) ?? "",
    price: prix,
    retailPrice: num(d.regular?.amount) ?? prix,
    currency: devise(d.price),
    savings: Math.round(num(d.cut) ?? 0),
    buyUrl: chaine(d.url) ?? "",
    available: true, // offres du comparateur = offres en cours
    drm: noms(d.drm),
    expiry: finOffre(d),
    voucher: chaine(d.voucher) || null,
  };
}

/**
 * Notes retenues : Steam, Metacritic (critiques) et OpenCritic. La note des joueurs de Metacritic
 * est écartée : elle double celle de Steam, en moins fiable.
 */
export function notes(reviews) {
  return tableau(reviews)
    .filter((r) => ["Steam", "Metascore", "OpenCritic"].includes(r?.source))
    .map((r) => {
      const url = chaine(r.url);
      return {
        source: r.source,
        score: Math.round(num(r.score) ?? 0),
        count: entier(r.count) ?? 0,
        url: url?.startsWith("https://") ? url : null,
      };
    })
    .filter((r) => r.score > 0);
}

/** Une ligne Instant Gaming du comparatif : euros seulement, et zone euro seulement. */
function ligneInstantGaming(ig) {
  return {
    storeName: "Instant Gaming",
    price: ig.price,
    retailPrice: ig.savings > 0 ? Math.round((ig.price / (1 - ig.savings / 100)) * 100) / 100 : ig.price,
    currency: "EUR",
    savings: ig.savings,
    buyUrl: ig.url,
    available: ig.available,
    drm: [],
    expiry: null,
    voucher: null,
  };
}

const positif = (v) => {
  const n = entier(v);
  return n ? n : null;
};

/** La fiche d'un jeu. `null` si le comparateur ne répond pas (rien à montrer sans son titre). */
export async function game(ctx, gameId) {
  const info = await getJson(ctx, `games/info/v2?id=${urlencode(gameId)}`);
  const titre = chaine(info?.title);
  if (titre === null) return null;
  const appId = entier(info.appid);

  // `appdetails` sert à la fiche Steam, à la description traduite et au repli d'Instant Gaming.
  const details = appId === null ? Promise.resolve(null) : appdetails(ctx, appId);

  const [prix, ig, igdb, steam, d] = await Promise.all([
    postJson(ctx, cheminPrix(ctx), [gameId]),
    // ⚠️ Zone euro seulement : au milieu de prix en livres ou en dollars, sa ligne ne se comparerait à rien.
    zoneEuro(ctx.region)
      ? offreInstantGaming(ctx, titre, async () => chaine((await details)?.name)?.trim() ?? null)
      : null,
    appId === null ? metaTitre(ctx, titre) : metaSteam(ctx, appId),
    appId === null ? null : ficheSteam(ctx, appId, details),
    details,
  ]);

  const entree = tableau(prix)[0] ?? null;
  const prices = tableau(entree?.deals)
    .map(offreFiche)
    .filter((o) => o !== null);
  if (ig !== null) prices.push(ligneInstantGaming(ig));
  prices.sort((a, b) => a.price - b.price);

  // 🔑 IGDB ne localise rien : sa description est en anglais, toujours. Celle de Steam est dans la
  // langue demandée, et par appid, aucun doute sur le jeu visé. Le genre, lui, reste celui d'IGDB.
  const traduite = d !== null && !PAS_UN_JEU.has(d.type) ? chaine(d.short_description) || null : null;

  let coverUrl = chaine(info.assets?.boxart);
  let heroUrl = igdb?.heroUrl ?? null;
  coverUrl ??= igdb?.coverUrl ?? null;
  if (appId !== null) {
    heroUrl ??= `${CDN}/${appId}/library_hero.jpg`;
    coverUrl ??= `${CDN}/${appId}/library_600x900.jpg`;
  }

  return {
    gameId,
    title: titre,
    coverUrl,
    heroUrl,
    appId,
    releaseDate: chaine(info.releaseDate),
    developers: noms(info.developers),
    publishers: noms(info.publishers),
    reviews: notes(info.reviews),
    playersDay: positif(info.players?.day),
    playersPeak: positif(info.players?.peak),
    waitlisted: positif(info.stats?.waitlisted),
    low3m: num(entree?.historyLow?.m3?.amount),
    low1y: num(entree?.historyLow?.y1?.amount),
    steam,
    cheapestEver: num(entree?.historyLow?.all?.amount),
    currency: devise(entree?.historyLow?.all),
    prices,
    description: traduite ?? igdb?.description ?? null,
    genre: igdb?.genre ?? null,
    developer: igdb?.developer ?? null,
    year: igdb?.year ?? null,
    screenshots: igdb?.screenshots ?? [],
  };
}
