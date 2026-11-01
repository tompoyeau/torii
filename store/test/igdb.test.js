import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { genreCourt, lienPartage, lireMeta, lirePages, metaSteam, metaTitre, nomCorrespond, norm, pageEpic, traduireGenre } from "../src/igdb.js";
import { contexte, installerReseau, repartirDeZero, reponseJson } from "./outils.js";

beforeEach(() => {
  repartirDeZero();
});

test("tous les genres IGDB sont traduits", () => {
  // Les trois derniers sont identiques en français, c'est voulu.
  const identiques = ["Arcade", "MOBA", "Sport"];
  for (const brut of [
    "Adventure", "Arcade", "Card & Board Game", "Fighting", "Hack and slash/Beat 'em up", "Indie", "MOBA",
    "Music", "Pinball", "Platform", "Point-and-click", "Puzzle", "Quiz/Trivia", "Racing",
    "Real Time Strategy (RTS)", "Role-playing (RPG)", "Shooter", "Simulator", "Sport", "Strategy", "Tactical",
    "Turn-based strategy (TBS)", "Visual Novel",
  ]) {
    const court = genreCourt(brut);
    if (!identiques.includes(brut)) assert.notEqual(traduireGenre(court, "fr"), court, `genre non traduit : ${brut}`);
  }
});

test("un genre inconnu traverse sans dommage", () => {
  assert.equal(traduireGenre("Roguelike", "fr"), "Roguelike");
  assert.equal(traduireGenre("constructor", "fr"), "constructor");
});

test("les sigles se déplient, dans les deux langues", () => {
  assert.equal(traduireGenre(genreCourt("Role-playing (RPG)"), "fr"), "Jeu de rôle");
  assert.equal(traduireGenre(genreCourt("Real Time Strategy (RTS)"), "en"), "Real-time strategy");
});

// « Overwatch 2 » n'existe que comme alias de « Overwatch ». Sans ça, la recherche ne ramène que
// des DLC, tous écartés, et le jeu reste sans jaquette.
test("un nom alternatif vaut le nom principal, et rien d'autre", () => {
  const jeu = { name: "Overwatch", alternative_names: [{ name: "Overwatch II" }, { name: "Overwatch 2" }] };
  assert.ok(nomCorrespond(jeu, norm("Overwatch 2")));
  assert.ok(nomCorrespond(jeu, norm("Overwatch")));
  assert.ok(!nomCorrespond({ name: "Overwatch 2: Invasion Bundle" }, norm("Overwatch 2")));
  const nu = { name: "Hadès" };
  assert.ok(nomCorrespond(nu, norm("HADÈS")));
  assert.ok(!nomCorrespond(nu, norm("Hadès II")));
});

test("les accents ne font pas échouer la correspondance", () => {
  assert.equal(norm("Hadès"), norm("Hades"));
  assert.equal(norm("Pokémon Écarlate"), "pokemonecarlate");
  assert.equal(norm("CŒUR & âme"), "coeurame");
  assert.equal(norm("Straße"), "strasse");
  assert.equal(norm("PUBG: Battlegrounds"), "pubgbattlegrounds");
  assert.notEqual(norm("Hadès"), norm("Hadès II"));
});

test("lit une fiche IGDB", () => {
  const meta = lireMeta({
    id: 1942,
    name: "The Witcher 3: Wild Hunt",
    genres: [{ id: 12, name: "Role-playing (RPG)" }, { id: 31, name: "Adventure" }],
    summary: "Un sorceleur",
    cover: { image_id: "co1" },
    screenshots: [{ image_id: "s1" }, { image_id: "s2" }],
    involved_companies: [{ developer: false, company: { name: "Bandai" } }, { developer: true, company: { name: "CDPR" } }],
    first_release_date: 1431993600,
  }, "fr");
  assert.equal(meta.id, 1942);
  assert.equal(meta.genre, "Jeu de rôle");
  assert.equal(meta.coverUrl, "https://images.igdb.com/igdb/image/upload/t_cover_big_2x/co1.jpg");
  // Pas d'artwork : le hero est la première capture.
  assert.equal(meta.heroUrl, "https://images.igdb.com/igdb/image/upload/t_1080p/s1.jpg");
  assert.equal(meta.developer, "CDPR");
  assert.equal(meta.year, 2015);
  assert.equal(meta.screenshots.length, 2);
});

test("un jeu Steam se trouve par son appid", async () => {
  const { appels, proxy } = installerReseau((url) => {
    if (url.endsWith("/external_games")) return reponseJson([{ game: 1942, uid: "292030" }]);
    return reponseJson([{ id: 1942, name: "The Witcher 3" }]);
  });
  const meta = await metaSteam(contexte({ env: { PROXY: proxy } }), 292030);
  assert.equal(meta.id, 1942);
  assert.ok(appels[0].corps.includes('uid = ("292030")'));
  assert.ok(appels[1].corps.includes("where id = (1942)"));
});

test("un jeu hors Steam : l'égalité d'abord, puis une recherche sans jamais prendre le premier", async () => {
  const { appels, proxy } = installerReseau((_, init) => {
    if (init.body.includes("search ")) return reponseJson([{ id: 1, name: "Control Resonant" }, { id: 2, name: "CONTROL" }]);
    return reponseJson([]);
  });
  const meta = await metaTitre(contexte({ env: { PROXY: proxy } }), 'Con"trol™');
  assert.equal(meta.id, 2);
  assert.equal(appels.length, 2);
  assert.ok(appels[0].corps.includes('name ~ "Control"'), "guillemets et symboles retirés");
});

test("sans réponse au premier appel, on n'en tente pas un second", async () => {
  const { appels, proxy } = installerReseau(() => new Response("", { status: 502 }));
  const ctx = contexte({ env: { PROXY: proxy } });
  assert.equal(await metaTitre(ctx, "Control"), null);
  assert.equal(appels.length, 1);
  assert.equal(ctx.partiel, true);
});

// --- Le lien à partager -------------------------------------------------------------

test("la page Epic perd sa langue", () => {
  for (const url of [
    "https://store.epicgames.com/en-US/p/alan-wake-2",
    "https://www.epicgames.com/store/p/alan-wake-2",
    "https://www.epicgames.com/store/en-US/product/alan-wake-2/home",
  ]) {
    assert.equal(pageEpic(url), "https://store.epicgames.com/p/alan-wake-2", url);
  }
  assert.equal(pageEpic("https://www.epicgames.com/fortnite"), null);
});

// Relevé IGDB de VALORANT (24/10/2026) : pas de Steam, une page Epic, un site officiel.
test("les pages d'un jeu hors Steam", () => {
  const p = lirePages({
    websites: [
      { type: 1, url: "https://playvalorant.com/" },
      { type: 16, url: "https://www.epicgames.com/store/en-US/p/valorant" },
    ],
    external_games: [
      { external_game_source: 3, url: "https://www.giantbomb.com/games/3030-77445/" },
      { external_game_source: 26, url: "https://store.epicgames.com/en-US/p/valorant" },
    ],
  });
  assert.equal(p.steam, null);
  assert.equal(p.epic, "https://store.epicgames.com/p/valorant");
  assert.equal(p.site, "https://playvalorant.com/");
});

test("la page Steam vient de l'appid", () => {
  const p = lirePages({ external_games: [{ external_game_source: 1, uid: "2357570", url: "https://store.steampowered.com/app/2357570" }] });
  assert.equal(p.steam, "https://store.steampowered.com/app/2357570/");
});

test("le lien partagé : Steam, sinon Epic, GOG, ou le site officiel", async () => {
  const { proxy } = installerReseau((_, init) => {
    if (init.body.includes("fields websites.url")) {
      return reponseJson([{ websites: [{ type: 1, url: "https://playvalorant.com/" }], external_games: [] }]);
    }
    return reponseJson([{ id: 9, name: "VALORANT" }]);
  });
  assert.equal(await lienPartage(contexte({ env: { PROXY: proxy } }), "VALORANT"), "https://playvalorant.com/");
});
