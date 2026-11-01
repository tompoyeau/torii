import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { appliquerPrix, extras, garderVraiesPromos, home, lire, lireRayon, menuTags, parcourir, rayonValide, recents, valeurTri } from "../src/vitrine.js";
import { contexte, installerReseau, repartirDeZero, reponseJson } from "./outils.js";

const noms = new Map([[122, "RPG"], [1695, "Monde ouvert"], [6650, "Nudité"], [4182, "Solo"], [19, "Action"]]);

// Item de `Query` tel que relevé le 22/10/2026 (raccourci).
const cyberpunk = () => ({
  item_type: 0,
  id: 1091500,
  success: 1,
  visible: true,
  name: "Cyberpunk 2077",
  appid: 1091500,
  type: 0,
  content_descriptorids: [1, 2, 5],
  reviews: { summary_filtered: { review_count: 892730, percent_positive: 87, review_score: 8, review_score_label: "très positives" } },
  basic_info: { short_description: "Cyberpunk 2077 est un JDR d'action-aventure en monde ouvert." },
  release: { steam_release_date: 1607558400 },
  tags: [{ tagid: 1695 }, { tagid: 6650 }, { tagid: 122 }, { tagid: 99999 }, { tagid: 4182 }, { tagid: 19 }],
  assets: {
    asset_url_format: "steam/apps/1091500/${FILENAME}?t=1",
    header: "a1/header.jpg",
    library_capsule_2x: "b2/library_capsule_2x.jpg",
    library_hero: "c3/library_hero.jpg",
  },
});

beforeEach(() => {
  repartirDeZero();
});

test("lit un jeu de la boutique", () => {
  const j = lire(cyberpunk(), noms);
  assert.equal(j.appId, 1091500);
  assert.equal(j.title, "Cyberpunk 2077");
  assert.equal(j.capsuleUrl, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1091500/b2/library_capsule_2x.jpg?t=1");
  assert.ok(j.headerUrl.endsWith("/a1/header.jpg?t=1"));
  assert.ok(j.heroUrl.endsWith("/c3/library_hero.jpg?t=1"));
  // La nudité est écartée, un tag sans nom aussi, et on s'arrête à trois.
  assert.deepEqual(j.tags, ["Monde ouvert", "RPG", "Solo"]);
  assert.deepEqual([j.reviewScore, j.reviewPercent, j.reviewCount], [8, 87, 892730]);
  assert.equal(j.releaseDate, 1607558400);
  assert.equal(j.comingSoon, false);
  assert.equal(j.isFree, false);
  assert.ok(j.description.startsWith("Cyberpunk 2077 est"));
});

test("écarte le contenu adulte et ce qui n'est pas un jeu", () => {
  assert.equal(lire({ ...cyberpunk(), content_descriptorids: [1, 3] }, noms), null);
  assert.equal(lire({ ...cyberpunk(), type: 6 }, noms), null);
  assert.equal(lire({ ...cyberpunk(), success: 2 }, noms), null);
});

test("un jeu à venir sans avis : pas de faux 0 %, et le texte de l'éditeur", () => {
  const j = lire({
    appid: 5,
    type: 0,
    name: "Sheep Tag 2",
    reviews: { summary_filtered: { review_count: 0, percent_positive: 0, review_score: 0 } },
    release: { is_coming_soon: true, custom_release_date_message: "Prochainement", coming_soon_display: "text_comingsoon" },
  }, noms);
  assert.deepEqual([j.reviewScore, j.reviewPercent], [0, null]);
  assert.equal(j.comingSoon, true);
  assert.equal(j.releaseDisplay, "text_comingsoon");
  assert.equal(j.releaseText, "Prochainement");
});

test("la vedette veut des avis et une bannière", () => {
  const tiede = cyberpunk();
  tiede.appid = 2;
  tiede.reviews.summary_filtered.percent_positive = 61;
  const sansBanniere = cyberpunk();
  sansBanniere.appid = 3;
  sansBanniere.assets = { asset_url_format: "steam/apps/3/${FILENAME}", header: "h.jpg" };
  const v = lireRayon("featured", [tiede, cyberpunk(), sansBanniere], noms);
  assert.deepEqual(v.map((j) => j.appId), [1091500]);
});

test("nouveautés dans la fenêtre", () => {
  const t = 2_000_000_000;
  const items = [
    { appid: 1, release: { steam_release_date: t - 10 * 86_400 } },
    { appid: 2, release: { steam_release_date: t - 400 * 86_400 } },
    { appid: 3, release: { steam_release_date: t + 86_400 } }, // précommande
    { appid: 4 },
    { appid: 5, release: { steam_release_date: t - 74 * 86_400 } },
  ];
  assert.deepEqual(recents(items, t, 10), [1, 5]);
  assert.deepEqual(recents(items, t, 1), [1]);
});

test("applique la meilleure offre autorisée", () => {
  const entree = {
    id: "g1",
    historyLow: { all: { amount: 12.18, currency: "EUR" } },
    deals: [
      { shop: { name: "JoyBuggy" }, price: { amount: 9.99, currency: "EUR" }, regular: { amount: 39.99 }, cut: 75, url: "https://joy", flag: "N" },
      { shop: { name: "Steam" }, price: { amount: 13.99, currency: "EUR" }, regular: { amount: 39.99 }, cut: 65, url: "https://steam", flag: "S", expiry: "2026-11-03T00:59:00+01:00" },
    ],
  };
  const j = {};
  appliquerPrix(j, entree, new Set());
  assert.deepEqual([j.price, j.storeName, j.record], [9.99, "JoyBuggy", true]);

  const k = {};
  appliquerPrix(k, entree, new Set(["JoyBuggy"]));
  assert.equal(k.price, 13.99);
  assert.equal(k.normalPrice, 39.99);
  assert.equal(k.savings, 65);
  assert.equal(k.currency, "EUR");
  assert.equal(k.historyLow, 12.18);
  // `S` = plus bas de cette boutique seulement : pas un record.
  assert.equal(k.record, false);
  assert.equal(k.expiry, "2026-11-03T00:59:00+01:00");
});

test("un plein tarif n'est pas un record", () => {
  // Relevé réel : un jeu qui sort est « H » à 69,99 €, -0 %.
  const j = {};
  appliquerPrix(j, {
    id: "cod",
    historyLow: { all: { amount: 69.99, currency: "EUR" } },
    deals: [{ shop: { name: "Steam" }, price: { amount: 69.99, currency: "EUR" }, regular: { amount: 69.99 }, cut: 0, url: "https://steam", flag: "H" }],
  }, new Set());
  assert.equal(j.price, 69.99);
  assert.equal(j.record, false);
});

test("la vedette garde les vraies promos", () => {
  const gardes = garderVraiesPromos([
    { appId: 1, price: 17.99, savings: 70 },
    { appId: 2, price: 38.49, savings: 4 },
    { appId: 3, price: null, savings: 0 },
  ]);
  assert.deepEqual(gardes.map((j) => j.appId), [1]);
});

test("un DLC n'est accepté que si on le demande", () => {
  const extension = { ...cyberpunk(), type: 4 };
  assert.equal(lire(extension, noms), null);
  assert.equal(lire(extension, noms, true).appId, 1091500);
  // Une bande-son (autre type) reste refusée, même parmi les contenus additionnels.
  assert.equal(lire({ ...cyberpunk(), type: 6 }, noms, true), null);
});

test("noms des rayons et des tris", () => {
  assert.equal(rayonValide("topSellers", null), true);
  assert.equal(rayonValide("genre", 122), true);
  assert.equal(rayonValide("genre", null), false);
  assert.equal(rayonValide("autre", null), false);
  assert.equal(valeurTri("topSellers"), 10);
  assert.equal(valeurTri("rating"), 21);
  assert.equal(valeurTri("trending"), 20);
  assert.equal(valeurTri("price"), null);
  assert.equal(valeurTri("constructor"), null);
});

test("le menu des catégories garde l'ordre de Steam", () => {
  // Indépendant, Gore (exclu), Survie, Utilitaires (logiciel), Coop.
  const menu = menuTags({
    ordre: [492, 4345, 1662, 87, 1685],
    noms: new Map([[492, "Indépendant"], [4345, "Gore"], [1662, "Survie"], [87, "Utilitaires"], [1685, "Coop"]]),
  });
  assert.deepEqual(menu.map((t) => t.id), [492, 1662, 1685]);
});

// --- De bout en bout, sans réseau ---------------------------------------------------

/** Steam et le comparateur tels que les voit l'accueil. */
function reseauAccueil({ prix = () => reponseJson([{ id: "itad-1", historyLow: { all: { amount: 5, currency: "EUR" } }, deals: [{ shop: { name: "GOG" }, price: { amount: 9, currency: "EUR" }, regular: { amount: 30 }, cut: 70, url: "https://gog/1" }] }]) } = {}) {
  return installerReseau((url) => {
    if (url.includes("IStoreQueryService")) return reponseJson({ response: { store_items: [cyberpunk()] } });
    if (url.includes("populartags")) return reponseJson([{ tagid: 122, name: "RPG" }]);
    if (url.includes("lookup/id/shop/61")) return reponseJson({ "app/1091500": "itad-1" });
    if (url.includes("games/prices/v3")) return prix();
    if (url.includes("sort=price")) {
      return reponseJson({ list: [{ id: "c1", title: "Offert", type: "game", deal: { shop: { name: "Epic" }, price: { amount: 0 }, regular: { amount: 10 }, cut: 100, url: "https://e" } }] });
    }
    if (url.includes("deals/v2")) {
      return reponseJson({ list: [{ id: "d1", title: "En promo", type: "game", assets: { boxart: "https://x/b.jpg" }, deal: { shop: { name: "Steam" }, price: { amount: 3, currency: "EUR" }, regular: { amount: 6 }, cut: 50, url: "https://s" } }] });
    }
    return null;
  });
}

test("l'accueil complet : Steam choisit, le comparateur tarife", async () => {
  const { proxy } = reseauAccueil();
  const ctx = contexte({ env: { PROXY: proxy } });
  const a = await home(ctx);
  assert.equal(ctx.partiel, false);
  assert.deepEqual(a.featured.map((j) => [j.gameId, j.price, j.savings, j.storeName]), [["itad-1", 9, 70, "GOG"]]);
  assert.equal(a.topSellers.length, 1);
  assert.equal(a.deals[0].title, "En promo");
  assert.deepEqual(a.giveaways.map((g) => g.title), ["Offert"]);
});

test("sans le comparateur, l'accueil part sans ses prix et le dit", async () => {
  const { proxy } = reseauAccueil({ prix: () => new Response("", { status: 429 }) });
  const ctx = contexte({ env: { PROXY: proxy } });
  const a = await home(ctx);
  assert.equal(ctx.partiel, true);
  assert.equal(a.topSellers[0].price, null);
  assert.deepEqual(a.featured, [], "pas de prix, pas de vraie promo à montrer");
});

test("un parcours rend le total de Steam, et null si Steam est en panne", async () => {
  const { proxy } = installerReseau((url) => {
    if (url.includes("IStoreQueryService")) return reponseJson({ response: { store_items: [cyberpunk()], metadata: { total_matching_records: 726 } } });
    if (url.includes("populartags")) return reponseJson([]);
    if (url.includes("lookup/id")) return reponseJson({});
    return null;
  });
  const p = await parcourir(contexte({ env: { PROXY: proxy } }), [1662, 1685], 10, 0);
  assert.equal(p.total, 726);
  assert.equal(p.jeux[0].gameId, "");

  assert.deepEqual(await parcourir(contexte(), [], 10, 0), { total: 0, jeux: [] });

  installerReseau(() => new Response("", { status: 500 }));
  assert.equal(await parcourir(contexte(), [1662], 10, 0), null);
});

test("le bas de fiche demande les similaires par paires de tags, sans se répéter", async () => {
  const corps = [];
  const { proxy } = installerReseau((url) => {
    if (url.includes("IStoreQueryService")) {
      corps.push(JSON.parse(new URL(url).searchParams.get("input_json")).query.filters.tagids_must_match);
      return reponseJson({ response: { store_items: [{ ...cyberpunk(), appid: 7 }, { ...cyberpunk(), appid: 1 }] } });
    }
    if (url.includes("populartags")) return reponseJson([{ tagid: 122, name: "RPG" }, { tagid: 19, name: "Action" }]);
    if (url.includes("GetItems")) return reponseJson({ response: { store_items: [{ ...cyberpunk(), appid: 9, type: 4 }] } });
    if (url.includes("lookup/id")) return reponseJson({});
    return null;
  });
  const x = await extras(contexte({ env: { PROXY: proxy } }), 1, [9], [122, 19, 4]);
  assert.deepEqual(corps, [
    [{ tagids: [122] }, { tagids: [19] }],
    [{ tagids: [19] }, { tagids: [4] }],
  ]);
  assert.deepEqual(x.dlc.map((j) => j.appId), [9]);
  // Le jeu lui-même (1) n'est pas son propre voisin, et 7 ne revient pas deux fois.
  assert.deepEqual(x.similar.map((j) => j.appId), [7]);
  assert.deepEqual(x.similarTags, ["RPG", "Action"]);
});
