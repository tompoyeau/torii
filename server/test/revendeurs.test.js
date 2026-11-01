import assert from "node:assert/strict";
import { test } from "node:test";
import { normaliserRevendeurs, publicAccount, setHiddenStores } from "../src/auth.js";

const session = { accountId: "compte-1", account: {} };

/** Une base qui retient ce qu'on lui demande d'écrire. */
function base() {
  const ecritures = [];
  return {
    ecritures,
    DB: {
      prepare: (sql) => ({
        bind: (...valeurs) => ({
          run: async () => {
            ecritures.push({ sql, valeurs });
          },
        }),
      }),
    },
  };
}

const requete = (corps) =>
  new Request("https://torii-api.topo-host.com/v1/me/hidden-stores", {
    method: "PUT",
    body: typeof corps === "string" ? corps : JSON.stringify(corps),
  });

test("la liste est bornée, sans doublon, et toujours dans le même ordre", () => {
  assert.deepEqual(normaliserRevendeurs([" G2A ", "Kinguin", "G2A", "", 12, null, "Eneba"]), ["Eneba", "G2A", "Kinguin"]);
  assert.equal(normaliserRevendeurs(Array.from({ length: 500 }, (_, i) => `R${i}`)).length, 200);
  assert.equal(normaliserRevendeurs(["x".repeat(300)])[0].length, 80);
});

test("la liste entière remplace la précédente, pour ce compte seulement", async () => {
  const env = base();
  const reponse = await setHiddenStores(requete({ stores: ["Kinguin", "G2A"] }), env, session);
  assert.equal(reponse.status, 200);
  assert.deepEqual(await reponse.json(), { stores: ["G2A", "Kinguin"] });
  assert.deepEqual(env.ecritures[0].valeurs, ['["G2A","Kinguin"]', "compte-1"]);
});

test("une liste vide est un choix valable : tout réafficher", async () => {
  const env = base();
  assert.equal((await setHiddenStores(requete({ stores: [] }), env, session)).status, 200);
  assert.deepEqual(env.ecritures[0].valeurs, ["[]", "compte-1"]);
});

test("un corps illisible ou sans liste ne touche à rien", async () => {
  for (const corps of ["pas du json", {}, { stores: "G2A" }]) {
    const env = base();
    assert.equal((await setHiddenStores(requete(corps), env, session)).status, 400);
    assert.equal(env.ecritures.length, 0);
  }
});

test("le compte rend sa liste, même si la colonne est vide ou abîmée", () => {
  const ligne = { id: "a", email: "a@b.c", display_name: "a", friend_code: "x" };
  assert.deepEqual(publicAccount({ ...ligne, hidden_stores: '["G2A"]' }).hiddenStores, ["G2A"]);
  assert.deepEqual(publicAccount({ ...ligne, hidden_stores: "abîmée" }).hiddenStores, []);
  assert.deepEqual(publicAccount(ligne).hiddenStores, []);
});
