/**
 * Accueil de la Boutique : les rayons « façon Steam » (vedette, meilleures ventes, nouveautés,
 * jeux attendus, genres, free-to-play) et le parcours par catégories.
 *
 * 🔑 DEUX SOURCES, DEUX RÔLES : STEAM CHOISIT LES JEUX, ITAD DONNE LE PRIX. Mêmes règles que
 * `metadata/vitrine.rs` dans l'application installée, dont les relevés valent ici :
 *
 * ⚠️ `sort` est une énumération que Steam ne documente pas. 10 = ventes du moment dans le pays,
 * 13 = popularité (classe mieux les jeux à venir), 21 = appréciation, 20 = sorties récentes
 * qui marchent. `GetWeeklyTopSellers`, lui, est mondial et figé pour la semaine.
 */

import { cheapest, devise, estRecord, finOffre, idsItad, prixGroupes } from "./itad.js";
import { chaine, entier, maintenant, num, tableau } from "./lib.js";
import { deals, giveaways } from "./offres.js";
import {
  EXCLUS, GET_ITEMS, QUERY, adresseAsset, chargerTags, contexteSteam, demos, steamItems, steamReponse,
} from "./steam.js";

const PAR_RAYON = 12;
export const PAR_PAGE = 24;
/** Une « nouveauté » est sortie depuis moins de 75 jours. */
const FENETRE_NOUVEAUTES = 75 * 86_400;
/** Seuil d'avis positifs pour la vedette : on ne met pas en avant un jeu mal reçu. */
const AVIS_VEDETTE = 75;
/** Remise minimale de la meilleure offre pour figurer en vedette. */
const REMISE_VEDETTE = 20;

/** Descripteurs Steam « contenu sexuel » et « nudité » : masqués par défaut, comme sur Steam. */
const ADULTE = [3, 4];
/** Tags jamais affichés sur une carte : ils décrivent un contenu, ou répètent « Gratuit ». */
const TAGS_MASQUES = new Set([6650, 12095, 4345, 4667, 113]);
/** Tags de logiciels : la recherche est limitée aux jeux, ils ne ramèneraient presque rien. */
const TAGS_LOGICIELS = new Set([84, 87, 784, 809, 872, 1027, 1445, 5407, 7948, 8013, 13906]);

const DONNEES_COMPLETES = {
  include_assets: true,
  include_release: true,
  include_reviews: true,
  include_basic_info: true,
  include_tag_count: 6,
};

const RAYONS = new Set(["featured", "topSellers", "newReleases", "upcoming", "genre", "freeToPlay"]);
const TRIS = { topSellers: 10, rating: 21, trending: 20 };

/** Un nom de rayon est-il valable (avec son tag pour un genre) ? */
export const rayonValide = (nom, tag) => RAYONS.has(nom) && (nom !== "genre" || entier(tag) !== null);

/** La valeur Steam d'un tri du parcours, `null` s'il est inconnu. */
export const valeurTri = (tri) => (Object.hasOwn(TRIS, tri) ? TRIS[tri] : null);

// --- Steam -----------------------------------------------------------------------

/** `Query` limité aux jeux, contenu adulte exclu : `{ items, total }`, `null` si panne. */
async function requetePage(ctx, tri, debut, nombre, filtres, donnees) {
  const reponse = await steamReponse(ctx, QUERY, {
    query: {
      start: debut,
      count: nombre,
      sort: tri,
      filters: { ...filtres, type_filters: { include_games: true }, content_descriptors_excluded: ADULTE },
    },
    context: contexteSteam(ctx),
    data_request: donnees,
  });
  if (reponse === null) return null;
  return { items: tableau(reponse.store_items), total: entier(reponse.metadata?.total_matching_records) ?? 0 };
}

async function requete(ctx, tri, nombre, filtres, donnees = DONNEES_COMPLETES) {
  const page = await requetePage(ctx, tri, 0, nombre, filtres, donnees);
  return page === null ? null : page.items;
}

/** Fiches complètes d'une liste d'appids, dans l'ordre demandé. */
async function details(ctx, appids) {
  const items = await steamItems(ctx, GET_ITEMS, {
    ids: appids.map((appid) => ({ appid })),
    context: contexteSteam(ctx),
    data_request: DONNEES_COMPLETES,
  });
  if (items === null) return null;
  const parId = new Map(items.map((i) => [i?.appid, i]));
  return appids.map((a) => parId.get(a)).filter(Boolean);
}

/** Appids des jeux sortis dans la fenêtre des nouveautés, dans l'ordre des ventes. */
export function recents(items, instant, nombre) {
  return items
    .filter((i) => {
      const sortie = i?.release?.steam_release_date;
      return Number.isInteger(sortie) && sortie > instant - FENETRE_NOUVEAUTES && sortie <= instant;
    })
    .map((i) => entier(i?.appid))
    .filter((a) => a !== null)
    .slice(0, nombre);
}

/** Les jeux bruts d'un rayon, tels que Steam les renvoie. `null` si panne. */
async function brut(ctx, nom, tag) {
  const payants = { exclude_free_items: true };
  switch (nom) {
    // Populaires, en promo d'au moins 20 %, et bien reçus. On en demande plus qu'on n'en
    // montre : l'interface retire ceux déjà possédés.
    case "featured":
      return requete(ctx, 10, 30, {
        released_only: true,
        price_filters: { exclude_free_items: true, min_discount_percent: 20 },
        review_score_filter: { min_score: 6 },
      });
    case "topSellers":
      return requete(ctx, 10, 20, { price_filters: payants });
    // ⚠️ `release_date_filter` existe, mais Steam l'ignore sans un `release_date_type` qu'on ne
    // connaît pas. On prend donc les meilleures ventes, avec leur date seule, on garde les
    // sorties récentes, et on ne demande la fiche complète que pour celles-là.
    case "newReleases": {
      const liste = await requete(
        ctx,
        10,
        150,
        { released_only: true, price_filters: payants },
        { include_release: true },
      );
      if (liste === null) return null;
      const appids = recents(liste, maintenant(), PAR_RAYON + 2);
      return appids.length ? details(ctx, appids) : [];
    }
    case "upcoming":
      return requete(ctx, 13, 20, { coming_soon_only: true });
    case "genre":
      return requete(ctx, 10, 20, {
        released_only: true,
        tagids_must_match: [{ tagids: [tag] }],
        price_filters: payants,
      });
    case "freeToPlay":
      return requete(ctx, 10, 16, { price_filters: { only_free_items: true } });
    default:
      return null;
  }
}

// --- Lire un jeu ------------------------------------------------------------------

/**
 * Un item de l'API de la boutique. `null` pour ce qui n'est pas à montrer : réponse en échec,
 * autre chose qu'un jeu (ou un contenu additionnel si `dlc`), contenu adulte.
 */
export function lire(item, noms, dlc = false) {
  if (!item) return null;
  const succes = entier(item.success);
  if ((succes !== null && succes !== 1) || item.visible === false) return null;
  const type = entier(item.type);
  if (type !== null && type !== 0 && !(dlc && type === 4)) return null;
  if (tableau(item.content_descriptorids).some((c) => ADULTE.includes(c))) return null;

  const appId = entier(item.appid) ?? entier(item.id);
  const titre = chaine(item.name)?.trim();
  if (appId === null || !titre) return null;

  const assets = item.assets ?? {};
  const avis = item.reviews?.summary_filtered;
  const sortie = item.release;
  const nbAvis = entier(avis?.review_count) ?? 0;
  return {
    appId,
    gameId: "",
    title: titre,
    capsuleUrl: adresseAsset(assets, ["library_capsule_2x", "library_capsule"]),
    headerUrl: adresseAsset(assets, ["header", "main_capsule"]),
    heroUrl: adresseAsset(assets, ["library_hero", "library_hero_2x"]),
    description: chaine(item.basic_info?.short_description)?.trim() || null,
    tags: tableau(item.tags)
      .map((t) => entier(t?.tagid))
      .filter((id) => id !== null && !TAGS_MASQUES.has(id))
      .map((id) => noms.get(id))
      .filter(Boolean)
      .slice(0, 3),
    // Pas d'avis du tout : Steam renvoie un score à 0 % qu'il ne faut pas afficher.
    reviewScore: nbAvis > 0 ? (entier(avis?.review_score) ?? 0) : 0,
    reviewPercent: nbAvis > 0 ? entier(avis?.percent_positive) : null,
    reviewCount: nbAvis,
    releaseDate: Number.isInteger(sortie?.steam_release_date) ? sortie.steam_release_date : null,
    releaseDisplay: chaine(sortie?.coming_soon_display) ?? "date_full",
    releaseText: chaine(sortie?.custom_release_date_message),
    comingSoon: sortie?.is_coming_soon === true,
    isFree: item.is_free === true,
    hasDemo: demos(item).length > 0,
    price: null,
    normalPrice: null,
    currency: "",
    savings: 0,
    storeName: "",
    buyUrl: "",
    historyLow: null,
    record: false,
    expiry: null,
  };
}

/** Lit les jeux d'un rayon et applique ses règles propres. */
export function lireRayon(nom, items, noms) {
  let jeux = items.map((i) => lire(i, noms)).filter((j) => j !== null);
  if (nom === "featured") {
    jeux = jeux.filter((j) => (j.reviewPercent ?? 0) >= AVIS_VEDETTE && j.heroUrl !== null);
  }
  return jeux.slice(0, PAR_RAYON);
}

/**
 * La vedette promet une promo : la remise de Steam ne suffit pas, il faut que la meilleure
 * offre toutes boutiques en soit une. Mesuré : Disney Dreamlight Valley, en promo sur Steam,
 * ressortait à -4 % chez un autre vendeur.
 */
export const garderVraiesPromos = (jeux) =>
  jeux.filter((j) => j.price !== null && j.savings >= REMISE_VEDETTE);

// --- Prix -------------------------------------------------------------------------

/** Applique une entrée `games/prices/v3` à un jeu. */
export function appliquerPrix(jeu, entree, exclus) {
  jeu.historyLow = num(entree?.historyLow?.all?.amount);
  jeu.currency = devise(entree?.historyLow?.all);
  const offre = cheapest(entree, exclus);
  const prix = num(offre?.price?.amount);
  if (prix === null) return;
  jeu.price = prix;
  jeu.normalPrice = num(offre.regular?.amount) ?? prix;
  jeu.currency = devise(offre.price);
  jeu.savings = Math.round(num(offre.cut) ?? 0);
  jeu.storeName = chaine(offre.shop?.name) ?? "";
  jeu.buyUrl = chaine(offre.url) ?? "";
  jeu.record = estRecord(offre);
  jeu.expiry = finOffre(offre);
}

/** Donne à chaque jeu son identifiant ITAD et sa meilleure offre. */
async function completer(ctx, jeux) {
  if (!jeux.length) return;
  const ids = await idsItad(ctx, jeux.map((j) => j.appId));
  for (const j of jeux) {
    if (ids.has(j.appId)) j.gameId = ids.get(j.appId);
  }
  // Un jeu gratuit n'a pas d'offre à comparer : inutile de le faire tarifer.
  const aTarifer = [...new Set(jeux.filter((j) => !j.isFree && j.gameId).map((j) => j.gameId))].sort();
  const entrees = new Map((await prixGroupes(ctx, aTarifer)).map((e) => [e?.id, e]));
  for (const j of jeux) {
    if (entrees.has(j.gameId)) appliquerPrix(j, entrees.get(j.gameId), ctx.exclus);
  }
}

// --- Points d'entrée --------------------------------------------------------------

/** L'accueil : vedette, meilleures ventes, promos et jeux offerts, demandés en parallèle. */
export async function home(ctx) {
  const [vedette, ventes, promos, offerts, { noms }] = await Promise.all([
    brut(ctx, "featured"),
    brut(ctx, "topSellers"),
    deals(ctx, 0, "featured"),
    giveaways(ctx),
    chargerTags(ctx),
  ]);
  const enVedette = lireRayon("featured", vedette ?? [], noms);
  const tous = [...enVedette, ...lireRayon("topSellers", ventes ?? [], noms)];
  await completer(ctx, tous);
  return {
    featured: garderVraiesPromos(tous.slice(0, enVedette.length)),
    topSellers: tous.slice(enVedette.length),
    deals: promos,
    giveaways: offerts,
  };
}

/** Un rayon seul (onglet, genre, free-to-play), prix compris. */
export async function shelf(ctx, nom, tag) {
  const [items, { noms }] = await Promise.all([brut(ctx, nom, tag), chargerTags(ctx)]);
  const jeux = lireRayon(nom, items ?? [], noms);
  await completer(ctx, jeux);
  return jeux;
}

/** Les tags du menu « Catégories », dans l'ordre de Steam (les plus employés d'abord). */
export function menuTags({ noms, ordre }) {
  return ordre
    .filter((id) => !EXCLUS.has(id) && !TAGS_LOGICIELS.has(id))
    .map((id) => ({ id, nom: noms.get(id) }));
}

export async function tagsBoutique(ctx) {
  return menuTags(await chargerTags(ctx));
}

/** Les jeux qui portent TOUS ces tags, une page à la fois, prix compris. `null` si panne. */
export async function parcourir(ctx, tags, tri, page) {
  if (!tags.length) return { total: 0, jeux: [] };
  const filtres = {
    released_only: true,
    tagids_must_match: tags.map((t) => ({ tagids: [t] })),
  };
  const [resultat, { noms }] = await Promise.all([
    requetePage(ctx, tri, page * PAR_PAGE, PAR_PAGE, filtres, DONNEES_COMPLETES),
    chargerTags(ctx),
  ]);
  if (resultat === null) return null;
  const jeux = resultat.items.map((i) => lire(i, noms)).filter((j) => j !== null);
  await completer(ctx, jeux);
  return { total: resultat.total, jeux };
}

/**
 * Jeux qui partagent les deux tags principaux du jeu (les plus vendus d'abord). S'ils sont trop
 * peu — « Cyberpunk » et « Monde ouvert » ensemble, c'est une poignée —, on complète avec les
 * tags suivants.
 */
async function similaires(ctx, appId, dlc, tags) {
  const vus = new Set([...dlc, appId]);
  const out = [];
  for (let i = 0; i + 1 < tags.length && i < 2; i++) {
    const items = await requete(ctx, 10, 16, {
      released_only: true,
      price_filters: { exclude_free_items: true },
      tagids_must_match: [{ tagids: [tags[i]] }, { tagids: [tags[i + 1]] }],
    });
    for (const item of items ?? []) {
      const appid = entier(item?.appid);
      if (appid === null || vus.has(appid)) continue;
      vus.add(appid);
      out.push(item);
    }
    if (out.length >= 6) break;
  }
  return out;
}

/** Contenus additionnels et jeux similaires d'une fiche, prix compris (un appel pour les deux). */
export async function extras(ctx, appId, dlc, tags) {
  const [dlcBruts, proches, { noms }] = await Promise.all([
    dlc.length ? details(ctx, dlc) : [],
    similaires(ctx, appId, dlc, tags),
    chargerTags(ctx),
  ]);
  const contenus = (dlcBruts ?? []).map((i) => lire(i, noms, true)).filter((j) => j !== null);
  const voisins = proches.map((i) => lire(i, noms)).filter((j) => j !== null).slice(0, PAR_RAYON);
  await completer(ctx, [...contenus, ...voisins]);
  return {
    dlc: contenus,
    similar: voisins,
    similarTags: tags.slice(0, 2).map((t) => noms.get(t)).filter(Boolean),
  };
}

/** L'identifiant ITAD d'un jeu Steam, pour ouvrir SA fiche : jamais par le titre. */
export async function idPourSteam(ctx, appid) {
  return (await idsItad(ctx, [appid])).get(appid) ?? null;
}
