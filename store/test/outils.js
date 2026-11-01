/** Ce qu'il faut pour faire tourner le Worker hors de Cloudflare, sans réseau. */

import { oublierEtat } from "../src/itad.js";
import { oublierTags } from "../src/steam.js";

/** Un cache en mémoire, à la façon de `caches.default`. */
export function installerCache() {
  const entrees = new Map();
  globalThis.caches = {
    default: {
      match: async (requete) => entrees.get(requete.url)?.clone(),
      put: async (requete, reponse) => {
        entrees.set(requete.url, reponse.clone());
      },
    },
  };
  return entrees;
}

/** Remet à zéro tout ce qui survit d'un test à l'autre. */
export function repartirDeZero() {
  oublierEtat();
  oublierTags();
  return installerCache();
}

/** Le contexte d'une requête, en français depuis la France sauf demande contraire. */
export function contexte(autres = {}) {
  return {
    env: {},
    exec: { waitUntil: (p) => p },
    origin: "https://test.example",
    ip: null,
    region: "FR",
    langue: "fr",
    steamLangue: "french",
    exclus: new Set(),
    exclusCle: "",
    partiel: false,
    ...autres,
  };
}

export const reponseJson = (donnees, status = 200, headers = {}) =>
  new Response(JSON.stringify(donnees), { status, headers });

/**
 * Remplace `fetch` et le binding du proxy : `routes` reçoit chaque adresse appelée et rend une
 * `Response`. Renvoie la liste des appels, pour compter ce qui est vraiment parti.
 */
export function installerReseau(routes) {
  const appels = [];
  const repondre = async (url, init) => {
    appels.push({ url: String(url), methode: init?.method ?? "GET", corps: init?.body ?? null });
    return (await routes(String(url), init)) ?? new Response("inconnu", { status: 404 });
  };
  globalThis.fetch = repondre;
  return { appels, proxy: { fetch: repondre } };
}
