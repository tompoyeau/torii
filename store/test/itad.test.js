import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { cheapest, getJson, idsItad, prixGroupes } from "../src/itad.js";
import { appliquerOffre, deals, itemDepuisDeal, lireCadeaux, search, suggest, triItad } from "../src/offres.js";
import { num, urlencode } from "../src/lib.js";
import { contexte, installerReseau, repartirDeZero, reponseJson } from "./outils.js";

beforeEach(() => {
  repartirDeZero();
});

// Objet deal calqué sur `/deals/v2` list[] (prix EUR natif, boutique + lien fournis).
test("lit un deal", () => {
  const s = itemDepuisDeal({
    id: "018d9-uuid",
    slug: "elden-ring",
    title: "Elden Ring",
    assets: { boxart: "https://x/box.jpg" },
    deal: { shop: { id: 61, name: "Steam" }, price: { amount: 31.99, currency: "EUR" }, regular: { amount: 59.99 }, cut: 47, url: "https://store.steampowered.com/app/1245620" },
  });
  assert.equal(s.gameId, "018d9-uuid");
  assert.equal(s.title, "Elden Ring");
  assert.equal(s.coverUrl, "https://x/box.jpg");
  assert.equal(s.price, 31.99);
  assert.equal(s.normalPrice, 59.99);
  assert.equal(s.savings, 47);
  assert.equal(s.storeName, "Steam");
  assert.ok(s.buyUrl.includes("steampowered"));
});

test("un nombre, qu'il soit encodé en nombre ou en texte", () => {
  assert.equal(num(9.99), 9.99);
  assert.equal(num("9.99"), 9.99);
  assert.equal(num(null), null);
  assert.equal(num(""), null);
});

/** Entrée `games/prices/v3` à trois offres, de la moins chère à la plus chère. */
const entreePrix = () => ({
  id: "g1",
  deals: [
    { shop: { name: "JoyBuggy" }, price: { amount: 9.99 }, regular: { amount: 39.99 }, cut: 75, url: "https://joy/1" },
    { shop: { name: "Fanatical" }, price: { amount: 12.49 }, regular: { amount: 39.99 }, cut: 69, url: "https://fana/1" },
    { shop: { name: "Steam" }, price: { amount: 19.99 }, regular: { amount: 39.99 }, cut: 50, url: "https://steam/1" },
  ],
});

// Le revendeur masqué ne doit JAMAIS être retenu : la vitrine, la recherche et les rayons ne
// rendent qu'un prix par jeu, il n'y a rien à re-filtrer côté front.
test("la meilleure offre écarte les revendeurs masqués", () => {
  const entree = entreePrix();
  assert.equal(cheapest(entree, new Set()).shop.name, "JoyBuggy");
  assert.equal(cheapest(entree, new Set(["JoyBuggy"])).shop.name, "Fanatical");
  assert.equal(cheapest(entree, new Set(["JoyBuggy", "Fanatical"])).price.amount, 19.99);
  // Tout masqué : aucune offre retenue (le jeu sort de la grille).
  assert.equal(cheapest(entree, new Set(["JoyBuggy", "Fanatical", "Steam"])), null);
});

test("remplacer une offre réécrit toute la carte", () => {
  const item = { storeName: "JoyBuggy", price: 9.99, normalPrice: 39.99, savings: 75, buyUrl: "https://joy/1" };
  assert.equal(appliquerOffre(item, cheapest(entreePrix(), new Set(["JoyBuggy"]))), true);
  assert.equal(item.storeName, "Fanatical");
  assert.equal(item.price, 12.49);
  assert.equal(item.savings, 69);
  assert.equal(item.buyUrl, "https://fana/1");
});

// Tête réelle de `deals/v2?sort=price` (22/10/2026, raccourcie).
test("cadeaux : seulement les jeux à zéro", () => {
  const liste = [
    { id: "a", title: "Sker Ritual", type: "game", assets: { boxart: "https://x/a.jpg", banner600: "https://x/a6.jpg" }, deal: { shop: { name: "Epic Game Store" }, price: { amount: 0, currency: "EUR" }, regular: { amount: 19.99 }, cut: 100, flag: "N", expiry: "2026-10-29T16:00:00+01:00", url: "https://epic/a" } },
    { id: "b", title: "Stellar End Demo Version", type: null, deal: { shop: { name: "Humble Store" }, price: { amount: 0 }, regular: { amount: 4.99 }, cut: 100, url: "https://h/b" } },
    { id: "c", title: "Squad Weapon Skins", type: "dlc", deal: { shop: { name: "Steam" }, price: { amount: 0 }, regular: { amount: 2.99 }, cut: 100, url: "https://s/c" } },
    { id: "d", title: "Pongo", type: "game", deal: { shop: { name: "Dreamgame" }, price: { amount: 0.06 }, regular: { amount: 0.79 }, cut: 92, url: "https://d/d" } },
    { id: "e", title: "Terraspace", type: "game", deal: { shop: { name: "GOG" }, price: { amount: 0 }, regular: { amount: 14.99 }, cut: 100, url: "https://gog/e" } },
  ];
  const tous = lireCadeaux(liste, new Set());
  assert.deepEqual(tous.map((i) => i.title), ["Sker Ritual", "Terraspace"]);
  assert.equal(tous[0].bannerUrl, "https://x/a6.jpg");
  assert.equal(tous[0].expiry, "2026-10-29T16:00:00+01:00");
  assert.equal(tous[0].record, true);
  assert.equal(lireCadeaux(liste, new Set(["GOG"])).length, 1);
});

test("tri de la vitrine", () => {
  assert.equal(triItad("savings"), "-cut");
  assert.equal(triItad("price"), "price");
  assert.equal(triItad("autre"), "-trending");
});

test("encode comme l'application installée", () => {
  assert.equal(urlencode("Hades (2020) & co!"), "Hades%20%282020%29%20%26%20co%21");
});

// --- La protection du quota ---------------------------------------------------------

test("un 429 met le comparateur en pause : plus aucun appel ne part", async () => {
  const { appels, proxy } = installerReseau(() => new Response("", { status: 429 }));
  const ctx = contexte({ env: { PROXY: proxy } });
  assert.equal(await getJson(ctx, "deals/v2"), null);
  assert.equal(ctx.partiel, true);
  assert.equal(appels.length, 1);

  assert.equal(await getJson(contexte({ env: { PROXY: proxy } }), "deals/v2"), null);
  assert.equal(appels.length, 1, "aucun appel pendant la pause");
});

test("une autre erreur n'en déclenche pas : une panne n'est pas un refus", async () => {
  let reponse = () => new Response("", { status: 500 });
  const { appels, proxy } = installerReseau(() => reponse());
  assert.equal(await getJson(contexte({ env: { PROXY: proxy } }), "deals/v2"), null);
  reponse = () => reponseJson({ list: [] });
  assert.deepEqual(await getJson(contexte({ env: { PROXY: proxy } }), "deals/v2"), { list: [] });
  assert.equal(appels.length, 2);
});

test("la pause se voit depuis un autre processus, par le cache", async () => {
  const { proxy } = installerReseau(() => new Response("", { status: 429 }));
  await getJson(contexte({ env: { PROXY: proxy } }), "deals/v2");

  // Un autre processus : il n'a pas la variable, seulement le cache du centre de données.
  const cache = globalThis.caches;
  const { oublierEtat } = await import("../src/itad.js");
  oublierEtat();
  globalThis.caches = cache;
  const autre = installerReseau(() => reponseJson({ list: [] }));
  assert.equal(await getJson(contexte({ env: { PROXY: autre.proxy } }), "deals/v2"), null);
  assert.equal(autre.appels.length, 0);
});

test("Retry-After donne la durée de la pause, bornée", async () => {
  const { proxy } = installerReseau(() => new Response("", { status: 429, headers: { "retry-after": "1" } }));
  await getJson(contexte({ env: { PROXY: proxy } }), "deals/v2");
  // Bornée à 10 s au moins : l'appel suivant n'est toujours pas parti.
  const { appels } = installerReseau(() => reponseJson({}));
  await getJson(contexte({ env: { PROXY: { fetch: globalThis.fetch } } }), "deals/v2");
  assert.equal(appels.length, 0);
});

test("le plafond d'envoi coupe avant le réseau, sans mettre en pause", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson({ list: [] }));
  const plein = { limit: async () => ({ success: false }) };
  const ctx = contexte({ env: { PROXY: proxy, RL_ITAD: plein } });
  assert.equal(await getJson(ctx, "deals/v2"), null);
  assert.equal(ctx.partiel, true);
  assert.equal(appels.length, 0);

  const libre = { limit: async () => ({ success: true }) };
  assert.deepEqual(await getJson(contexte({ env: { PROXY: proxy, RL_ITAD: libre } }), "deals/v2"), { list: [] });
});

test("les appels passent par la vraie adresse du proxy, pour partager son cache", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson({}));
  await getJson(contexte({ env: { PROXY: proxy } }), "deals/v2?country=FR");
  assert.equal(appels[0].url, "https://torii-igdb-proxy.toriiapp.workers.dev/itad/deals/v2?country=FR");
});

test("les prix groupés se mémorisent, quel que soit l'ordre demandé", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson([{ id: "a" }, { id: "b" }]));
  const ctx = () => contexte({ env: { PROXY: proxy } });
  assert.equal((await prixGroupes(ctx(), ["a", "b"])).length, 2);
  assert.equal((await prixGroupes(ctx(), ["b", "a"])).length, 2);
  assert.equal(appels.length, 1);
  // Un autre pays n'a pas les mêmes prix.
  await prixGroupes(contexte({ env: { PROXY: proxy }, region: "US" }), ["a", "b"]);
  assert.equal(appels.length, 2);
});

test("les prix partent par tranches de 200", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson([]));
  await prixGroupes(contexte({ env: { PROXY: proxy } }), Array.from({ length: 450 }, (_, i) => `g${i}`));
  assert.deepEqual(appels.map((a) => JSON.parse(a.corps).length), [200, 200, 50]);
});

test("une panne de prix ne se mémorise pas", async () => {
  let reponse = () => new Response("", { status: 500 });
  const { appels, proxy } = installerReseau(() => reponse());
  assert.deepEqual(await prixGroupes(contexte({ env: { PROXY: proxy } }), ["a"]), []);
  reponse = () => reponseJson([{ id: "a" }]);
  assert.equal((await prixGroupes(contexte({ env: { PROXY: proxy } }), ["a"])).length, 1);
  assert.equal(appels.length, 2);
});

test("identifiants : un seul appel, et ce qui est trouvé reste en mémoire", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson({ "app/220": "id-220", "app/137730": null }));
  const ctx = () => contexte({ env: { PROXY: proxy } });
  const r = await idsItad(ctx(), [220, 137730, 42, 220]);
  // 137730 est inconnu d'ITAD, 42 n'est pas dans la réponse.
  assert.deepEqual([...r], [[220, "id-220"]]);
  assert.deepEqual(JSON.parse(appels[0].corps), ["app/42", "app/220", "app/137730"]);

  await idsItad(ctx(), [220]);
  assert.equal(appels.length, 1);
});

// --- Listes -------------------------------------------------------------------------

test("une vitrine dont l'offre vient d'un revendeur masqué est re-tarifée", async () => {
  const carte = (id, boutique, prix) => ({ id, title: id, type: "game", deal: { shop: { name: boutique }, price: { amount: prix }, regular: { amount: 40 }, cut: 50, url: `https://${boutique}` } });
  const { appels, proxy } = installerReseau((url) => {
    if (url.includes("deals/v2")) return reponseJson({ list: [carte("garde", "Steam", 5), carte("repris", "JoyBuggy", 2), carte("perdu", "JoyBuggy", 1), { ...carte("dlc", "Steam", 1), type: "dlc" }] });
    return reponseJson([
      { id: "repris", deals: [{ shop: { name: "JoyBuggy" }, price: { amount: 2 } }, { shop: { name: "GOG" }, price: { amount: 3 }, regular: { amount: 40 }, cut: 25, url: "https://gog" }] },
      { id: "perdu", deals: [{ shop: { name: "JoyBuggy" }, price: { amount: 1 } }] },
    ]);
  });
  const items = await deals(contexte({ env: { PROXY: proxy }, exclus: new Set(["JoyBuggy"]) }), 0, "featured");
  assert.deepEqual(items.map((i) => [i.gameId, i.storeName, i.price]), [["garde", "Steam", 5], ["repris", "GOG", 3]]);
  assert.deepEqual(JSON.parse(appels[1].corps), ["repris", "perdu"]);
});

test("sans revendeur masqué, la vitrine ne fait qu'un appel", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson({ list: [] }));
  await deals(contexte({ env: { PROXY: proxy } }), 2, "savings");
  assert.equal(appels.length, 1);
  assert.ok(appels[0].url.endsWith("deals/v2?country=FR&offset=96&limit=48&sort=-cut"));
});

test("la recherche écarte les DLC et ne rend que les jeux en vente", async () => {
  const { proxy } = installerReseau((url) => {
    if (url.includes("games/search")) {
      return reponseJson([
        { id: "a", title: "Hollow Knight", type: "game", assets: { boxart: "https://x/a.jpg" } },
        { id: "b", title: "Hollow Knight OST", type: "dlc" },
        { id: "c", title: "Hollow Knight: Silksong", type: "game" },
      ]);
    }
    return reponseJson([
      { id: "a", deals: [{ shop: { name: "GOG" }, price: { amount: 7, currency: "EUR" }, regular: { amount: 15 }, cut: 53, url: "https://gog" }] },
      { id: "c", deals: [] },
    ]);
  });
  const r = await search(contexte({ env: { PROXY: proxy } }), "  hollow knight ");
  assert.equal(r.length, 1);
  assert.deepEqual([r[0].gameId, r[0].title, r[0].coverUrl, r[0].price, r[0].currency], ["a", "Hollow Knight", "https://x/a.jpg", 7, "EUR"]);
});

test("l'autocomplétion ne demande aucun prix, et rien sous deux caractères", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson({ results: [{ id: "a", title: "Hades", type: "game" }, { id: "b", title: "Hades OST", type: "dlc" }] }));
  assert.deepEqual(await suggest(contexte({ env: { PROXY: proxy } }), "h"), []);
  assert.equal(appels.length, 0);
  assert.deepEqual(await suggest(contexte({ env: { PROXY: proxy } }), "hades"), [{ gameId: "a", title: "Hades", coverUrl: null }]);
  assert.equal(appels.length, 1);
});
