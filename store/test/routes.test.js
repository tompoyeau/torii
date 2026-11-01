import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import worker, { oublierSessions } from "../src/index.js";
import { installerReseau, repartirDeZero, reponseJson } from "./outils.js";

const JETON = "Bearer jeton-valide";
const libre = { limit: async () => ({ success: true }) };
const plein = { limit: async () => ({ success: false }) };

/** Un environnement complet : la session `jeton-valide` existe, le reste est refusé. */
function environnement(autres = {}) {
  const verifications = [];
  return {
    verifications,
    RL_IP: libre,
    RL_COMPTE: libre,
    RL_ITAD: libre,
    ORIGINES: "https://app.torii-app.fr",
    AUTH: {
      fetch: async (url, init) => {
        verifications.push({ url, headers: init.headers });
        if (init.headers.authorization !== JETON) return reponseJson({ error: "non_connecte" }, 401);
        return reponseJson({ account: { id: "compte-1" } });
      },
    },
    ...autres,
  };
}

const exec = { waitUntil: (p) => p };

function appeler(env, chemin, { jeton = JETON, origine, methode = "GET", ip } = {}) {
  const headers = {};
  if (jeton) headers.authorization = jeton;
  if (origine) headers.origin = origine;
  if (ip) headers["cf-connecting-ip"] = ip;
  return worker.fetch(new Request(`https://torii-store.topo-host.com${chemin}`, { method: methode, headers }), env, exec);
}

const suggestions = () => reponseJson({ results: [{ id: "a", title: "Hades", type: "game" }] });

beforeEach(() => {
  repartirDeZero();
  oublierSessions();
});

test("sans session, rien", async () => {
  const { appels, proxy } = installerReseau(suggestions);
  const env = environnement({ PROXY: proxy });
  assert.equal((await appeler(env, "/v1/suggest?q=hades", { jeton: null })).status, 401);
  assert.equal((await appeler(env, "/v1/suggest?q=hades", { jeton: "Bearer faux" })).status, 401);
  assert.equal(appels.length, 0);
});

test("avec une session, la réponse arrive sous les noms de champs de l'application", async () => {
  const { proxy } = installerReseau(suggestions);
  const r = await appeler(environnement({ PROXY: proxy }), "/v1/suggest?q=hades");
  assert.equal(r.status, 200);
  assert.deepEqual(await r.json(), [{ gameId: "a", title: "Hades", coverUrl: null }]);
});

test("une session vérifiée ne relit pas la base à chaque requête", async () => {
  const { proxy } = installerReseau(suggestions);
  const env = environnement({ PROXY: proxy });
  await appeler(env, "/v1/suggest?q=ha");
  await appeler(env, "/v1/suggest?q=hb");
  assert.equal(env.verifications.length, 1);
});

test("torii-api en panne n'est pas un refus : personne n'est déconnecté", async () => {
  const { proxy } = installerReseau(suggestions);
  const env = environnement({
    PROXY: proxy,
    AUTH: {
      fetch: async () => {
        throw new Error("panne");
      },
    },
  });
  const r = await appeler(env, "/v1/suggest?q=hades");
  assert.equal(r.status, 503);
  assert.equal((await r.json()).error, "indisponible");
});

test("la vérification garde l'adresse du client, pour ses limites par IP", async () => {
  const { proxy } = installerReseau(suggestions);
  const env = environnement({ PROXY: proxy });
  await appeler(env, "/v1/suggest?q=hades", { ip: "203.0.113.9" });
  assert.equal(env.verifications[0].headers["cf-connecting-ip"], "203.0.113.9");
});

test("les limites répondent 429 avec Retry-After", async () => {
  installerReseau(suggestions);
  const parIp = await appeler(environnement({ RL_IP: plein }), "/v1/suggest?q=hades");
  assert.equal(parIp.status, 429);
  assert.equal(parIp.headers.get("retry-after"), "60");
  const parCompte = await appeler(environnement({ RL_COMPTE: plein }), "/v1/suggest?q=hades");
  assert.equal(parCompte.status, 429);
});

test("un binding oublié se voit tout de suite", async () => {
  for (const manque of ["RL_IP", "RL_COMPTE", "RL_ITAD", "AUTH"]) {
    const r = await appeler(environnement({ [manque]: undefined }), "/v1/suggest?q=hades");
    assert.equal(r.status, 500, manque);
    assert.equal((await r.json()).error, "mal_configure");
  }
});

test("CORS : seulement les pages listées", async () => {
  installerReseau(suggestions);
  const env = environnement();
  const permise = await appeler(env, "/v1", { origine: "https://app.torii-app.fr" });
  assert.equal(permise.headers.get("access-control-allow-origin"), "https://app.torii-app.fr");
  const autre = await appeler(env, "/v1", { origine: "https://ailleurs.example" });
  assert.equal(autre.headers.get("access-control-allow-origin"), null);

  const avant = await appeler(env, "/v1/home", { origine: "https://app.torii-app.fr", methode: "OPTIONS", jeton: null });
  assert.equal(avant.status, 204);
  assert.equal(avant.headers.get("access-control-allow-headers"), "authorization");
});

test("les erreurs portent aussi les en-têtes CORS, sinon le front ne les lirait pas", async () => {
  installerReseau(suggestions);
  const r = await appeler(environnement(), "/v1/home", { origine: "https://app.torii-app.fr", jeton: null });
  assert.equal(r.status, 401);
  assert.equal(r.headers.get("access-control-allow-origin"), "https://app.torii-app.fr");
});

test("seul GET est accepté, et une route inconnue répond 404", async () => {
  const env = environnement();
  assert.equal((await appeler(env, "/v1/home", { methode: "POST" })).status, 405);
  assert.equal((await appeler(env, "/v1/rien")).status, 404);
  assert.equal((await appeler(env, "/__cache/itad/pause")).status, 404);
  assert.equal((await appeler(env, "/constructor")).status, 404);
});

test("les paramètres invalides sont refusés avant tout appel", async () => {
  const { appels, proxy } = installerReseau(suggestions);
  const env = environnement({ PROXY: proxy });
  assert.equal((await appeler(env, "/v1/shelf?kind=rien")).status, 400);
  assert.equal((await appeler(env, "/v1/shelf?kind=genre")).status, 400);
  assert.equal((await appeler(env, "/v1/browse?tags=1&sort=price")).status, 400);
  assert.equal((await appeler(env, "/v1/extras")).status, 400);
  assert.equal((await appeler(env, "/v1/game/pas..un..id")).status, 400);
  assert.equal(appels.length, 0);
});

test("un grand appid et un grand tag passent", async () => {
  const urls = [];
  const { proxy } = installerReseau((url) => {
    urls.push(url);
    if (url.includes("populartags")) return reponseJson([]);
    if (url.includes("GetItems") || url.includes("IStoreQueryService")) return reponseJson({ response: { store_items: [] } });
    return reponseJson({});
  });
  const env = environnement({ PROXY: proxy });
  assert.equal((await appeler(env, "/v1/shelf?kind=genre&tag=1100687")).status, 200);
  assert.equal((await appeler(env, "/v1/extras?appId=1091500&tags=1100687,122")).status, 200);
  assert.ok(urls.some((u) => decodeURIComponent(u).includes('"tagids":[1100687]')));
});

test("ce qu'un utilisateur a demandé sert aux autres", async () => {
  const { appels, proxy } = installerReseau(suggestions);
  const env = environnement({ PROXY: proxy });
  await appeler(env, "/v1/suggest?q=Hades");
  await appeler(env, "/v1/suggest?q=hades");
  assert.equal(appels.length, 1, "la casse ne compte pas");
});

test("une réponse incomplète ne se mémorise pas, et se signale", async () => {
  let reponse = () => new Response("", { status: 500 });
  const { appels, proxy } = installerReseau(() => reponse());
  const env = environnement({ PROXY: proxy });
  const panne = await appeler(env, "/v1/suggest?q=hades");
  assert.equal(panne.status, 200);
  assert.equal(panne.headers.get("x-torii-partiel"), "1");
  assert.deepEqual(await panne.json(), []);

  reponse = suggestions;
  const r = await appeler(env, "/v1/suggest?q=hades");
  assert.equal(r.headers.get("x-torii-partiel"), null);
  assert.equal((await r.json()).length, 1);
  assert.equal(appels.length, 2);
});

test("les revendeurs masqués font partie de la clé de cache", async () => {
  const { appels, proxy } = installerReseau((url) => {
    if (url.includes("games/search")) return reponseJson([{ id: "a", title: "Hades", type: "game" }]);
    return reponseJson([{ id: "a", deals: [
      { shop: { name: "JoyBuggy" }, price: { amount: 1 } },
      { shop: { name: "GOG" }, price: { amount: 2 } },
    ] }]);
  });
  const env = environnement({ PROXY: proxy });
  const tous = await (await appeler(env, "/v1/search?q=hades&region=FR")).json();
  const sans = await (await appeler(env, "/v1/search?q=hades&region=FR&exclus=JoyBuggy")).json();
  assert.equal(tous[0].storeName, "JoyBuggy");
  assert.equal(sans[0].storeName, "GOG");
  // Le comparateur n'a pas été redemandé pour autant : ses prix sont déjà en cache.
  assert.equal(appels.filter((a) => a.url.includes("games/prices")).length, 1);
});

test("une fiche sans le comparateur répond 503, pour que le front propose de réessayer", async () => {
  const { proxy } = installerReseau(() => new Response("", { status: 429 }));
  const r = await appeler(environnement({ PROXY: proxy }), "/v1/game/018d937f-2997-7131-b8b9-7c8af4825fa8");
  assert.equal(r.status, 503);
  assert.equal((await r.json()).error, "indisponible");
});

test("région et langue : une valeur inattendue retombe sur les défauts de l'application", async () => {
  const urls = [];
  const { proxy } = installerReseau((url) => {
    urls.push(url);
    return reponseJson({ list: [] });
  });
  await appeler(environnement({ PROXY: proxy }), "/v1/deals?region=../x&langue=klingon");
  assert.ok(urls[0].includes("country=US"));
});

test("fiche d'un jeu possédé : il faut un jeu, et l'absence de lien est une réponse, pas une panne", async () => {
  const { proxy } = installerReseau(() => reponseJson([]));
  const env = environnement({ PROXY: proxy });
  assert.equal((await appeler(env, "/v1/owned")).status, 400);
  const r = await appeler(env, "/v1/owned?titre=VALORANT");
  assert.equal(r.status, 200);
  assert.deepEqual(await r.json(), { fiche: null });
});

test("un appid connu passe directement, sans interroger IGDB", async () => {
  const { appels } = installerReseau((url) =>
    url.includes("GetNewsForApp") ? reponseJson({ appnews: { newsitems: [] } }) : reponseJson({ response: { store_items: [] } }),
  );
  const r = await appeler(environnement(), "/v1/owned?appId=620");
  assert.equal(r.status, 200);
  assert.equal((await r.json()).fiche.appId, 620);
  assert.ok(!appels.some((a) => a.url.includes("igdb") || a.corps));
});

test("descriptif et joueurs en ce moment passent eux aussi par la session", async () => {
  const { proxy } = installerReseau((url) =>
    url.includes("GetNumberOfCurrentPlayers")
      ? reponseJson({ response: { result: 1, player_count: 12 } })
      : reponseJson([{ id: 1, name: "VALORANT", summary: "Un jeu de tir." }]),
  );
  const env = environnement({ PROXY: proxy });
  assert.equal((await appeler(env, "/v1/meta")).status, 400);
  assert.equal((await appeler(env, "/v1/players/1", { jeton: null })).status, 401);
  assert.equal((await (await appeler(env, "/v1/meta?titre=VALORANT")).json()).meta.description, "Un jeu de tir.");
  assert.equal((await (await appeler(env, "/v1/players/1")).json()).joueurs, 12);
});

test("une erreur interne ne décrit rien au client", async () => {
  const env = environnement({
    RL_COMPTE: {
      limit: async () => {
        throw new Error("détail interne");
      },
    },
  });
  const r = await appeler(env, "/v1/suggest?q=hades");
  assert.equal(r.status, 500);
  assert.ok(!(await r.text()).includes("détail interne"));
});
