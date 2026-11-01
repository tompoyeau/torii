import assert from "node:assert/strict";
import { test } from "node:test";
import {
  forgetAll, forgetLibrary, forgetStats, readLibrary, readStats, uploadLibrary, uploadStats,
} from "../src/library.js";

const moi = { accountId: "moi", account: {} };
const ami = { accountId: "ami", account: {} };
const PC = "pc-1";

/** Un stockage et une base en mémoire : ceux d'un compte `moi`, avec l'appareil `pc-1`. */
function environnement() {
  const objets = new Map();
  const ecrituresSql = [];
  return {
    objets,
    ecrituresSql,
    LIBS: {
      put: async (cle, valeur) => {
        objets.set(cle, typeof valeur === "string" ? valeur : new TextDecoder().decode(valeur));
      },
      get: async (cle) => {
        const v = objets.get(cle);
        return v === undefined ? null : { body: v, json: async () => JSON.parse(v) };
      },
      delete: async (cles) => {
        for (const c of [cles].flat()) objets.delete(c);
      },
    },
    DB: {
      prepare: (sql) => ({
        bind: (...valeurs) => ({
          first: async () => {
            if (sql.includes("share_library")) return { share_library: 1 }; // Ami accepté, partage ouvert.
            if (sql.includes("SELECT digest")) return { digest: "abc", updated_at: 1 };
            return valeurs[0] === "moi" && valeurs[1] === PC ? { oui: 1 } : null;
          },
          all: async () => ({ results: valeurs[0] === "moi" ? [{ device_id: PC }] : [] }),
          run: async () => {
            ecrituresSql.push(sql);
          },
        }),
      }),
    },
  };
}

const requete = (corps) => new Request("https://api.example/v1/x", { method: "PUT", body: JSON.stringify(corps) });
const lecture = (session, env) => readStats(new Request("https://api.example/v1/x"), env, session, PC);

test("le temps de jeu s'écrit à part, sans toucher à l'index que voient les amis", async () => {
  const env = environnement();
  const r = await uploadStats(requete({ deviceId: PC, games: [{ key: "hades", minutes: 600, lastPlayed: 1_700_000_000 }] }), env, moi);
  assert.equal(r.status, 200);
  assert.deepEqual([...env.objets.keys()], ["stats/moi/pc-1.json"]);
  assert.deepEqual(env.ecrituresSql, [], "ni date ni empreinte de la bibliothèque ne bougent");
});

test("seuls les champs connus, bornés, sont retenus", async () => {
  const env = environnement();
  await uploadStats(
    requete({
      deviceId: PC,
      games: [
        { key: "ok", minutes: 90, lastPlayed: 1_700_000_000, secret: "non" },
        { key: "negatif", minutes: -5 },
        { key: "chaine", minutes: "90" },
        { key: "futur", lastPlayed: 99_999_999_999 },
        { minutes: 10 },
        { key: "rien" },
      ],
    }),
    env,
    moi,
  );
  const { games } = JSON.parse(env.objets.get("stats/moi/pc-1.json"));
  assert.deepEqual(games, [{ key: "ok", minutes: 90, lastPlayed: 1_700_000_000 }]);
});

test("un nouvel envoi remplace le précédent : aucun historique", async () => {
  const env = environnement();
  await uploadStats(requete({ deviceId: PC, games: [{ key: "a", minutes: 10 }] }), env, moi);
  await uploadStats(requete({ deviceId: PC, games: [{ key: "b", minutes: 20 }] }), env, moi);
  const { games } = await (await lecture(moi, env)).json();
  assert.deepEqual(games, [{ key: "b", minutes: 20 }]);
});

test("un appareil inconnu ne peut rien déposer", async () => {
  const env = environnement();
  const r = await uploadStats(requete({ deviceId: "autre-pc", games: [{ key: "a", minutes: 1 }] }), env, moi);
  assert.equal(r.status, 409);
  assert.equal(env.objets.size, 0);
});

test("on ne lit que le sien : la session d'un ami ne mène pas au temps de jeu d'un autre", async () => {
  const env = environnement();
  await uploadStats(requete({ deviceId: PC, games: [{ key: "a", minutes: 10 }] }), env, moi);
  assert.deepEqual((await (await lecture(moi, env)).json()).games, [{ key: "a", minutes: 10 }]);
  assert.deepEqual((await (await lecture(ami, env)).json()).games, []);
});

test("un ami qui lit la bibliothèque n'y trouve aucun temps de jeu, même envoyé par erreur", async () => {
  const env = environnement();
  await uploadLibrary(
    requete({
      deviceId: PC,
      deviceName: "PC",
      digest: "abc",
      games: [{ key: "a", title: "A", platforms: ["steam"], steamAppId: 620, minutes: 500, lastPlayed: 1_700_000_000 }],
    }),
    env,
    moi,
  );
  const reponse = await readLibrary(new Request("https://api.example/v1/x"), env, ami, "moi", PC);
  assert.equal(reponse.status, 200);
  const { games } = JSON.parse(await reponse.text());
  assert.deepEqual(games, [{ key: "a", title: "A", platforms: ["steam"], steamAppId: 620 }]);
});

test("un identifiant Steam farfelu ne s'écrit pas", async () => {
  const env = environnement();
  await uploadLibrary(
    requete({
      deviceId: PC,
      games: [
        { key: "a", title: "A", platforms: [], steamAppId: "620" },
        { key: "b", title: "B", platforms: [], steamAppId: -1 },
      ],
    }),
    env,
    moi,
  );
  const { games } = JSON.parse(env.objets.get("lib/moi/pc-1.json"));
  assert.ok(games.every((j) => !("steamAppId" in j)));
});

test("éteindre l'option efface le temps de jeu, pas la bibliothèque", async () => {
  const env = environnement();
  env.objets.set("lib/moi/pc-1.json", "{}");
  env.objets.set("stats/moi/pc-1.json", "{}");
  await forgetStats(new Request("https://api.example/v1/x"), env, moi);
  assert.deepEqual([...env.objets.keys()], ["lib/moi/pc-1.json"]);
});

test("oublier un appareil, ou tout, emporte aussi le temps de jeu", async () => {
  let env = environnement();
  env.objets.set("lib/moi/pc-1.json", "{}");
  env.objets.set("stats/moi/pc-1.json", "{}");
  await forgetLibrary(new Request("https://api.example/v1/x"), env, moi, PC);
  assert.equal(env.objets.size, 0);

  env = environnement();
  env.objets.set("lib/moi/pc-1.json", "{}");
  env.objets.set("stats/moi/pc-1.json", "{}");
  await forgetAll(new Request("https://api.example/v1/x"), env, moi);
  assert.equal(env.objets.size, 0);
});
