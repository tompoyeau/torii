import assert from "node:assert/strict";
import { test } from "node:test";
import { memeListe, reconcilier } from "../src/lib/revendeurs.ts";

const inconnu = { compte: null, aEnvoyer: false };
const connu = { compte: "moi", aEnvoyer: false };

test("première rencontre : rien de ce qu'on avait masqué ne se perd, de part et d'autre", () => {
  const r = reconcilier(["G2A", "Eneba"], ["Kinguin", "G2A"], "moi", inconnu);
  assert.deepEqual(r.liste, ["Eneba", "G2A", "Kinguin"]);
  assert.equal(r.envoyer, true);
});

test("première rencontre sur un appareil vierge : il reprend le compte, sans rien renvoyer", () => {
  const r = reconcilier([], ["Kinguin", "G2A"], "moi", inconnu);
  assert.deepEqual(r.liste, ["G2A", "Kinguin"]);
  assert.equal(r.envoyer, false);
});

test("ensuite le compte a raison : un revendeur réaffiché ailleurs ne revient pas masqué", () => {
  const r = reconcilier(["G2A", "Kinguin"], ["Kinguin"], "moi", connu);
  assert.deepEqual(r.liste, ["Kinguin"]);
  assert.equal(r.envoyer, false);
});

test("un revendeur masqué ailleurs arrive ici", () => {
  assert.deepEqual(reconcilier(["G2A"], ["Eneba", "G2A"], "moi", connu).liste, ["Eneba", "G2A"]);
});

test("tout réafficher ailleurs vide la liste ici", () => {
  assert.deepEqual(reconcilier(["G2A"], [], "moi", connu).liste, []);
});

test("un changement resté ici faute de réseau passe devant le compte, et repart", () => {
  const r = reconcilier(["G2A"], ["G2A", "Kinguin"], "moi", { compte: "moi", aEnvoyer: true });
  assert.deepEqual(r.liste, ["G2A"]);
  assert.equal(r.envoyer, true);
});

test("changer de compte refait une fusion, même avec un envoi en attente pour l'ancien", () => {
  const r = reconcilier(["G2A"], ["Eneba"], "autre", { compte: "moi", aEnvoyer: true });
  assert.deepEqual(r.liste, ["Eneba", "G2A"]);
  assert.equal(r.envoyer, true);
});

test("deux synchronisations de suite ne changent plus rien", () => {
  const premiere = reconcilier(["G2A"], ["Eneba"], "moi", inconnu);
  const seconde = reconcilier(premiere.liste, premiere.liste, "moi", connu);
  assert.deepEqual(seconde.liste, premiere.liste);
  assert.equal(seconde.envoyer, false);
});

test("l'ordre ne compte pas pour dire que deux listes sont les mêmes", () => {
  assert.equal(memeListe(["a", "b"], ["b", "a", "a"]), true);
  assert.equal(memeListe(["a"], ["a", "b"]), false);
  // Un nom ne doit pas pouvoir se faire passer pour deux.
  assert.equal(memeListe(["a,b"], ["a", "b"]), false);
});
