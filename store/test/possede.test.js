import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { appidUnique, lienSteam, norm } from "../src/igdb.js";
import { extrait, fichePossede, imageAnnonce, joueursEnCeMoment, lireActualite } from "../src/possede.js";
import { contexte, installerReseau, repartirDeZero, reponseJson } from "./outils.js";

beforeEach(() => repartirDeZero());

// Les mêmes relevés que les tests de `metadata/possede.rs`.

test("une annonce réelle de Hunt: Showdown 1896", () => {
  const a = lireActualite({
    title: "Twitch Drops Dates | Blood Testament",
    url: "https://steamstore-a.akamaihd.net/news/externalpost/steam_community_announcements/1846651706696131",
    date: 1_792_500_000,
    contents:
      "[img]{STEAM_CLAN_IMAGE}/30897473/c98e8aaa00dde106cce0fa1770d02c4a9ccbe12b.jpg[/img] 🎁 [b]Twitch Drops are back![/b]  From Oct 26th to Nov 9th, you&#39;ll have another chance. [url=https://twitch.tv/x]Link[/url]",
  });
  assert.equal(a.titre, "Twitch Drops Dates | Blood Testament");
  assert.equal(a.image, "https://clan.akamai.steamstatic.com/images/30897473/c98e8aaa00dde106cce0fa1770d02c4a9ccbe12b.jpg");
  assert.equal(a.extrait, "🎁 Twitch Drops are back! From Oct 26th to Nov 9th, you'll have another chance. Link");
});

test("une annonce sans image ni adresse sûre", () => {
  const a = lireActualite({
    title: "CONTROL Resonant Is Out Now!",
    url: "https://store.steampowered.com/news/1",
    date: 1,
    contents: '[p]Manhattan is on the brink.[/p][previewyoutube="WhQm-ExRz60;full"][/previewyoutube] <img src="https://ailleurs.example/x.jpg">',
  });
  assert.equal(a.image, null, "une image hors des serveurs de Steam est écartée");
  assert.equal(a.extrait, "Manhattan is on the brink.");
  assert.equal(lireActualite({ title: "x", url: "http://non-chiffre" }), null);
  assert.equal(imageAnnonce("https://ailleurs.example/x.jpg"), null);
});

test("un extrait long est coupé sur un mot", () => {
  const e = extrait("mot ".repeat(80));
  assert.ok(e.endsWith("mot…") && [...e].length <= 181);
});

// Le lien avec Steam : relevés de l'application (Hades, Control).

const jeu = (name, uids, autres = {}) => ({
  name,
  external_games: uids.map((uid) => ({ external_game_source: 1, uid })),
  ...autres,
});

test("deux entrées du même nom, une seule sur Steam : c'est celle-là", () => {
  assert.equal(appidUnique([jeu("Hades", []), jeu("Hades", ["1145360"])], norm("Hades")), 1145360);
});

test("deux jeux du même nom tous deux sur Steam : aucun lien plutôt qu'un mauvais", () => {
  assert.equal(appidUnique([jeu("Control", ["1"]), jeu("Control", ["2"])], norm("Control")), null);
});

test("un autre nom, un DLC ou une édition ne font pas un lien", () => {
  assert.equal(appidUnique([jeu("CONTROL Resonant", ["9"])], norm("Control")), null);
  assert.equal(appidUnique([jeu("Control", ["9"], { parent_game: 4 })], norm("Control")), null);
  assert.equal(appidUnique([jeu("Control", ["9"], { version_parent: 4 })], norm("Control")), null);
});

test("un autre magasin ne compte pas, et un alias si", () => {
  const ailleurs = { name: "Control", external_games: [{ external_game_source: 26, uid: "abc" }] };
  assert.equal(appidUnique([ailleurs], norm("Control")), null);
  const alias = jeu("Overwatch", ["2357570"], { alternative_names: [{ name: "Overwatch 2" }] });
  assert.equal(appidUnique([alias], norm("Overwatch 2")), 2357570);
});

test("le lien interroge IGDB sur le nom exact, sans repli sur une recherche floue", async () => {
  const { appels, proxy } = installerReseau(() => reponseJson([jeu("Hades", ["1145360"])]));
  const appid = await lienSteam(contexte({ env: { PROXY: proxy } }), 'Hades™');
  assert.equal(appid, 1145360);
  assert.equal(appels.length, 1);
  assert.match(appels[0].corps, /where name ~ "Hades" \| alternative_names\.name ~ "Hades"/);
  assert.doesNotMatch(appels[0].corps, /search/);
});

// La fiche.

test("la fiche réunit Steam et les annonces, sans rien qui suppose le compte Steam", async () => {
  installerReseau((url) => {
    if (url.includes("GetNewsForApp")) {
      assert.match(url, /appid=594650&count=4&maxlength=0&feeds=steam_community_announcements/);
      return reponseJson({ appnews: { newsitems: [{ title: "Patch", url: "https://steam/news", date: 5, contents: "Notes" }] } });
    }
    if (url.includes("appdetails")) return reponseJson({ 594650: { success: true, data: { name: "Hunt", dlc: [1, 2] } } });
    if (url.includes("GetItems")) return reponseJson({ response: { store_items: [{ appid: 594650, name: "Hunt" }] } });
    return reponseJson([]);
  });
  const f = await fichePossede(contexte(), 594650);
  assert.equal(f.appId, 594650);
  assert.equal(f.actualites[0].titre, "Patch");
  assert.ok(f.steam);
  assert.deepEqual(f.dlc, []);
  assert.deepEqual(f.dlcPossedes, []);
});

test("Steam muet : la fiche reste vide et se signale comme incomplète", async () => {
  installerReseau(() => new Response("panne", { status: 500 }));
  const ctx = contexte();
  const f = await fichePossede(ctx, 594650);
  assert.equal(f.steam, null);
  assert.deepEqual(f.actualites, []);
  assert.equal(ctx.partiel, true);
});

test("joueurs en ce moment : seulement un chiffre valable", async () => {
  installerReseau(() => reponseJson({ response: { result: 1, player_count: 40356 } }));
  assert.equal(await joueursEnCeMoment(contexte(), 1), 40356);
  installerReseau(() => reponseJson({ response: { result: 42 } }));
  assert.equal(await joueursEnCeMoment(contexte(), 1), null);
  installerReseau(() => new Response("panne", { status: 503 }));
  assert.equal(await joueursEnCeMoment(contexte(), 1), null);
});
