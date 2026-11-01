import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { lireOffres, lirePrix } from "../src/instant_gaming.js";
import { game, notes, offreFiche } from "../src/produit.js";
import { contexte, installerReseau, repartirDeZero, reponseJson } from "./outils.js";

beforeEach(() => {
  repartirDeZero();
});

// Fragment calqué sur la vraie structure du site (recherche « elden ring ») : un DLC PC, une
// version console, et le jeu de base PC, celui qu'on doit sélectionner.
const FRAGMENT = `
<article class="item">
  <a class="cover" href="https://www.instant-gaming.com/en/16007-buy-elden-ring-shadow-of-the-erdtree-edition-pc-steam/" title="buy ...">
  <div class="name"><span class="title" title="Elden Ring Shadow of the Erdtree Edition - PC (Steam)">x</span></div>
  <div class="discount">-21%</div><div class="price">31.49&nbsp;€</div>
</article>
<article class="item">
  <a class="cover" href="https://www.instant-gaming.com/en/19051-buy-elden-ring-tarnished-edition-switch-2/" title="buy ...">
  <div class="name"><span class="title" title="Elden Ring Tarnished Edition - Switch 2">x</span></div>
  <div class="discount">-26%</div><div class="price">58.99&nbsp;€</div>
</article>
<article class="item">
  <a class="cover" href="https://www.instant-gaming.com/en/4824-buy-elden-ring-pc-steam/" title="buy ...">
  <div class="name"><span class="title" title="Elden Ring - PC (Steam)">x</span></div>
  <div class="discount">-14%</div><div class="price">51.39&nbsp;€</div>
</article>`;

test("Instant Gaming : l'édition PC de base, ni le DLC ni la console", () => {
  const o = lireOffres(FRAGMENT, "eldenring");
  assert.equal(o.price, 51.39);
  assert.equal(o.savings, 14);
  assert.ok(o.url.endsWith("4824-buy-elden-ring-pc-steam/"));
});

test("Instant Gaming : rien de fiable, rien de rendu", () => {
  assert.equal(lireOffres(FRAGMENT, "haloinfinite"), null);
});

test("Instant Gaming : les formats de prix", () => {
  assert.equal(lirePrix("51.39&nbsp;€"), 51.39);
  assert.equal(lirePrix("9,59 €"), 9.59);
  assert.equal(lirePrix("€"), null);
});

test("une ligne du comparatif", () => {
  const o = offreFiche({
    shop: { name: "GOG" },
    price: { amount: "19.99", currency: "EUR" },
    regular: { amount: 39.99 },
    cut: 49.6,
    url: "https://gog",
    drm: [{ name: "Drm Free" }],
    expiry: "",
    voucher: "",
  });
  assert.deepEqual(o, {
    storeName: "GOG",
    price: 19.99,
    retailPrice: 39.99,
    currency: "EUR",
    savings: 50,
    buyUrl: "https://gog",
    available: true,
    drm: ["Drm Free"],
    expiry: null,
    voucher: null,
  });
  assert.equal(offreFiche({ shop: { name: "GOG" } }), null);
});

test("les notes retenues", () => {
  const n = notes([
    { source: "Steam", score: 87, count: 12, url: "https://s" },
    { source: "Metacritic User", score: 50 },
    { source: "OpenCritic", score: 0 },
    { source: "Metascore", score: "90", url: "http://m" },
  ]);
  assert.deepEqual(n, [
    { source: "Steam", score: 87, count: 12, url: "https://s" },
    { source: "Metascore", score: 90, count: 0, url: null },
  ]);
});

// --- La fiche de bout en bout, sans réseau ------------------------------------------

/** Le monde tel que le voit la fiche d'un jeu Steam. */
function monde({ ig = FRAGMENT, details = {}, prix = null } = {}) {
  const fiche = {
    type: "game",
    name: "Elden Ring",
    short_description: "Levez-vous, Sans-éclat",
    about_the_game: "<p>Un monde ouvert</p>",
    achievements: { total: 42 },
    ...details,
  };
  return installerReseau((url) => {
    if (url.includes("games/info/v2")) {
      return reponseJson({
        title: "Elden Ring",
        appid: 1245620,
        assets: { boxart: "https://x/box.jpg" },
        releaseDate: "2022-02-25",
        developers: [{ name: "FromSoftware" }],
        publishers: [{ name: "Bandai" }],
        reviews: [{ source: "Steam", score: 92, count: 900000 }],
        players: { day: 12000, peak: 0 },
        stats: { waitlisted: 340 },
      });
    }
    if (url.includes("games/prices/v3")) {
      return prix ?? reponseJson([{
        id: "e1",
        historyLow: { all: { amount: 20, currency: "EUR" }, y1: { amount: 25 }, m3: { amount: 30 } },
        deals: [{ shop: { name: "Steam" }, price: { amount: 59.99, currency: "EUR" }, regular: { amount: 59.99 }, cut: 0, url: "https://steam" }],
      }]);
    }
    if (url.endsWith("/external_games")) return reponseJson([{ game: 7, uid: "1245620" }]);
    if (url.endsWith("/games")) return reponseJson([{ id: 7, name: "Elden Ring", summary: "Rise, Tarnished", genres: [{ name: "Role-playing (RPG)" }] }]);
    if (url.includes("GetItems")) return reponseJson({ response: { store_items: [{ appid: 1245620, tags: [{ tagid: 122 }] }] } });
    if (url.includes("appdetails")) return reponseJson({ 1245620: { success: true, data: fiche } });
    if (url.includes("populartags")) return reponseJson([{ tagid: 122, name: "RPG" }]);
    if (url.includes("rechercher")) return new Response(ig);
    if (url.includes("4824-buy")) return new Response("<button addtocart>");
    return null;
  });
}

test("la fiche complète", async () => {
  const { appels, proxy } = monde();
  const ctx = contexte({ env: { PROXY: proxy } });
  const g = await game(ctx, "e1");
  assert.equal(ctx.partiel, false);
  assert.equal(g.title, "Elden Ring");
  assert.equal(g.appId, 1245620);
  assert.equal(g.releaseDate, "2022-02-25");
  assert.deepEqual(g.developers, ["FromSoftware"]);
  assert.equal(g.playersDay, 12000);
  assert.equal(g.playersPeak, null, "un compteur à zéro n'est pas une information");
  assert.equal(g.waitlisted, 340);
  assert.deepEqual([g.low3m, g.low1y, g.cheapestEver, g.currency], [30, 25, 20, "EUR"]);
  // Triées par prix : Instant Gaming passe devant Steam, sur le site français.
  assert.deepEqual(g.prices.map((p) => [p.storeName, p.price]), [["Instant Gaming", 51.39], ["Steam", 59.99]]);
  assert.equal(g.prices[0].buyUrl, "https://www.instant-gaming.com/fr/4824-buy-elden-ring-pc-steam/");
  assert.equal(g.prices[0].retailPrice, 59.76);
  assert.equal(g.prices[0].available, true);
  // La description de Steam, traduite, plutôt que celle d'IGDB ; le genre reste celui d'IGDB.
  assert.equal(g.description, "Levez-vous, Sans-éclat");
  assert.equal(g.genre, "Jeu de rôle");
  assert.equal(g.steam.succes, 42);
  assert.deepEqual(g.steam.tags, ["RPG"]);
  assert.equal(g.coverUrl, "https://x/box.jpg");
  assert.equal(g.heroUrl, "https://cdn.cloudflare.steamstatic.com/steam/apps/1245620/library_hero.jpg");
  // `appdetails` sert trois fois, mais ne part qu'une.
  assert.equal(appels.filter((a) => a.url.includes("appdetails")).length, 1);
});

test("hors zone euro, pas d'Instant Gaming", async () => {
  const { appels, proxy } = monde();
  const g = await game(contexte({ env: { PROXY: proxy }, region: "US" }), "e1");
  assert.deepEqual(g.prices.map((p) => p.storeName), ["Steam"]);
  assert.ok(!appels.some((a) => a.url.includes("instant-gaming")));
});

test("la fiche d'une démo garde la description d'IGDB et donne son jeu complet", async () => {
  const { proxy } = monde({ details: { type: "demo", fullgame: { appid: "1245620", name: "Elden Ring" } } });
  const g = await game(contexte({ env: { PROXY: proxy } }), "e1");
  assert.equal(g.description, "Rise, Tarnished");
  assert.deepEqual(g.steam.jeuComplet, { appId: 1245620, titre: "Elden Ring" });
});

test("un Instant Gaming en panne ne prive pas de la fiche, et ne la dit pas incomplète", async () => {
  const { proxy } = monde({ ig: "<html>site en travaux</html>" });
  const ctx = contexte({ env: { PROXY: proxy } });
  const g = await game(ctx, "e1");
  assert.deepEqual(g.prices.map((p) => p.storeName), ["Steam"]);
  assert.equal(ctx.partiel, false);
});

test("sans les prix, la fiche reste lisible mais se dit incomplète", async () => {
  const { proxy } = monde({ prix: new Response("", { status: 429 }) });
  const ctx = contexte({ env: { PROXY: proxy }, region: "US" });
  const g = await game(ctx, "e1");
  assert.deepEqual(g.prices, []);
  assert.equal(g.cheapestEver, null);
  assert.equal(ctx.partiel, true);
});

test("sans le titre du comparateur, pas de fiche", async () => {
  const { proxy } = installerReseau(() => new Response("", { status: 500 }));
  assert.equal(await game(contexte({ env: { PROXY: proxy } }), "e1"), null);
});

test("un jeu hors Steam : recherche par titre, sans appel à Steam", async () => {
  const { appels, proxy } = installerReseau((url) => {
    if (url.includes("games/info/v2")) return reponseJson({ title: "Valorant", assets: {} });
    if (url.includes("games/prices/v3")) return reponseJson([]);
    if (url.endsWith("/games")) return reponseJson([{ id: 1, name: "VALORANT", cover: { image_id: "co" }, summary: "Tir" }]);
    if (url.includes("rechercher") || url.includes("/en/search/")) return new Response("");
    return null;
  });
  const g = await game(contexte({ env: { PROXY: proxy } }), "v1");
  assert.equal(g.appId, null);
  assert.equal(g.steam, null);
  assert.equal(g.description, "Tir");
  assert.equal(g.coverUrl, "https://images.igdb.com/igdb/image/upload/t_cover_big_2x/co.jpg");
  assert.ok(!appels.some((a) => a.url.includes("steampowered")));
});
