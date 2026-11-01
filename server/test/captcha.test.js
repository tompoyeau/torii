import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { captchaValide, depuisUnePage } from "../src/captcha.js";

const reel = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = reel;
});

const env = { TURNSTILE_SECRET: "secret", TURNSTILE_HOSTNAMES: "app.torii-app.fr, autre.example" };
const bonneReponse = { success: true, action: "connexion", hostname: "app.torii-app.fr" };

/** Cloudflare répond ceci ; `appels` retient ce qu'on lui a demandé. */
function cloudflare(reponse) {
  const appels = [];
  globalThis.fetch = async (url, init) => {
    appels.push({ url, corps: new URLSearchParams(init.body) });
    if (reponse instanceof Error) throw reponse;
    return reponse instanceof Response ? reponse : Response.json(reponse);
  };
  return appels;
}

test("un jeton valable pour notre page et notre action passe", async () => {
  const appels = cloudflare(bonneReponse);
  assert.equal(await captchaValide(env, "jeton", "203.0.113.9"), true);
  assert.equal(appels[0].url, "https://challenges.cloudflare.com/turnstile/v0/siteverify");
  assert.equal(appels[0].corps.get("secret"), "secret");
  assert.equal(appels[0].corps.get("response"), "jeton");
  assert.equal(appels[0].corps.get("remoteip"), "203.0.113.9");
});

test("les pages listées sont toutes acceptées, espaces compris", async () => {
  cloudflare({ ...bonneReponse, hostname: "autre.example" });
  assert.equal(await captchaValide(env, "jeton", null), true);
});

test("refusé : Cloudflare dit non, autre page, autre action", async () => {
  for (const reponse of [
    { ...bonneReponse, success: false },
    { ...bonneReponse, hostname: "localhost" },
    { ...bonneReponse, hostname: "ailleurs.example" },
    { ...bonneReponse, action: "autre" },
    { success: true, hostname: "app.torii-app.fr" },
  ]) {
    cloudflare(reponse);
    assert.equal(await captchaValide(env, "jeton", null), false, JSON.stringify(reponse));
  }
});

test("une clé d'essai ne passe jamais pour une de nos pages", async () => {
  const essai = { success: true, hostname: "example.com", metadata: { result_with_testing_key: true } };
  cloudflare(essai);
  assert.equal(await captchaValide(env, "jeton", null), false);
  // Sauf là où on l'a demandé, sur une pile locale.
  cloudflare(essai);
  assert.equal(await captchaValide({ ...env, TURNSTILE_HOSTNAMES: "example.com" }, "jeton", null), true);
});

test("le doute se solde par un refus : panne, réponse illisible, erreur", async () => {
  cloudflare(new Error("réseau"));
  assert.equal(await captchaValide(env, "jeton", null), false);
  cloudflare(new Response("pas du json"));
  assert.equal(await captchaValide(env, "jeton", null), false);
  cloudflare(new Response("{}", { status: 500 }));
  assert.equal(await captchaValide(env, "jeton", null), false);
});

test("sans jeton ou sans configuration, Cloudflare n'est même pas appelé", async () => {
  const appels = cloudflare(bonneReponse);
  for (const jeton of [undefined, null, "", 42, {}, "x".repeat(2049)]) {
    assert.equal(await captchaValide(env, jeton, null), false);
  }
  assert.equal(await captchaValide({ TURNSTILE_HOSTNAMES: "a.example" }, "jeton", null), false);
  assert.equal(await captchaValide({ TURNSTILE_SECRET: "s" }, "jeton", null), false);
  assert.equal(await captchaValide({ TURNSTILE_SECRET: "s", TURNSTILE_HOSTNAMES: " , " }, "jeton", null), false);
  assert.equal(appels.length, 0);
});

test("une page web se reconnaît à son en-tête Origin", () => {
  const appel = (headers) => new Request("https://torii-api.topo-host.com/v1/auth/signup", { method: "POST", headers });
  assert.equal(depuisUnePage(appel({ origin: "https://app.torii-app.fr" })), true);
  assert.equal(depuisUnePage(appel({ origin: "null" })), true);
  assert.equal(depuisUnePage(appel({})), false);
});
