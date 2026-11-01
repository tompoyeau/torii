/**
 * Torii — briques communes de la boutique : réponses, lecture des valeurs JSON, cache,
 * contexte d'une requête.
 */

const JSON_UTF8 = "application/json; charset=utf-8";

export function json(data, status = 200, headers = {}) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": JSON_UTF8, ...headers },
  });
}

/** Erreur applicative : un code stable que le client peut traiter, plus un message lisible. */
export function fail(status, code, message, headers = {}) {
  return json({ error: code, message }, status, headers);
}

// --- Lecture des valeurs JSON -------------------------------------------------------

/** Un entier positif ou nul, sinon `null` (un montant ou un texte ne passent pas). */
export const entier = (v) => (Number.isInteger(v) && v >= 0 ? v : null);

/** Une chaîne, sinon `null`. */
export const chaine = (v) => (typeof v === "string" ? v : null);

/** Un tableau, sinon un tableau vide. */
export const tableau = (v) => (Array.isArray(v) ? v : []);

/** Un nombre, qu'il soit encodé en nombre ou en texte. */
export function num(v) {
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v !== "string" || v.trim() === "") return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

/** Lit un JSON sans présumer qu'il est valide (corps vide, page d'erreur). `null` sinon. */
export function lireJson(texte) {
  try {
    return JSON.parse(texte);
  } catch {
    return null;
  }
}

/**
 * Encodage d'un paramètre de requête, le même que l'application installée.
 *
 * ⚠️ Pas `encodeURIComponent` seul, qui laisse `!'()*` tels quels : l'adresse ne serait
 * plus celle des clients PC, et le cache du proxy ne se partagerait plus avec eux.
 */
export function urlencode(s) {
  return encodeURIComponent(s).replace(
    /[!'()*]/g,
    (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
  );
}

export const maintenant = () => Math.floor(Date.now() / 1000);

/** Empreinte SHA-256 en hexadécimal. */
export async function empreinte(texte) {
  const octets = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(texte));
  return [...new Uint8Array(octets)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

// --- Contexte d'une requête ---------------------------------------------------------

const REGION = /^[A-Z]{2}$/;
const MAX_EXCLUS = 40;
const MAX_NOM_BOUTIQUE = 40;

/** Pays où Instant Gaming a un sens : ses prix sont en euros, sans équivalent local. */
const ZONE_EURO = new Set([
  "AT", "BE", "BG", "CH", "CY", "CZ", "DE", "DK", "EE", "ES",
  "FI", "FR", "GR", "HR", "HU", "IE", "IT", "LT", "LU", "LV",
  "MT", "NL", "NO", "PT", "RO", "SE", "SI", "SK",
]);

export const zoneEuro = (region) => ZONE_EURO.has(region);

/**
 * Ce que chaque fonction reçoit en premier argument : région, langue et revendeurs masqués
 * (sans état global, contrairement à `locale.rs` : ici, deux requêtes de deux utilisateurs
 * différents tournent dans le même processus), plus les accès au reste du monde.
 *
 * `partiel` passe à vrai dès qu'une source n'a pas répondu : voir `memo`.
 */
export function lireContexte(request, env, exec) {
  const url = new URL(request.url);
  const q = url.searchParams;
  const region = (q.get("region") || "").toUpperCase();
  const langue = q.get("langue") === "fr" ? "fr" : "en";
  const exclus = [...new Set(q.getAll("exclus").map((n) => n.slice(0, MAX_NOM_BOUTIQUE)).filter(Boolean))]
    .sort()
    .slice(0, MAX_EXCLUS);
  return {
    env,
    exec,
    origin: url.origin,
    ip: request.headers.get("cf-connecting-ip") || null,
    region: REGION.test(region) ? region : "US",
    langue,
    steamLangue: langue === "fr" ? "french" : "english",
    exclus: new Set(exclus),
    exclusCle: exclus.join("|"),
    partiel: false,
  };
}

// --- Cache ------------------------------------------------------------------------

/**
 * Le texte mémorisé sous `cle`, ou ce que rend `charger`, mémorisé pour `ttl` secondes.
 *
 * 🔑 UNE RÉPONSE INCOMPLÈTE NE SE MÉMORISE JAMAIS. Si le comparateur est en pause au moment
 * où l'on compose l'accueil, celui-ci part sans ses prix, ce qui vaut mieux que rien pour
 * celui qui attend. Le mémoriser, ce serait servir des jeux sans prix à tout le monde
 * pendant dix minutes, longtemps après la fin de la panne.
 *
 * 🔑 La clé est bâtie sur l'`origin` de la requête entrante : le cache de Cloudflare est
 * cloisonné par zone, une clé pointant ailleurs ne serait jamais relue. Le chemin `/__cache/`
 * ne correspond à aucune route : appelé directement, il répond 404.
 */
export async function memo(ctx, cle, ttl, charger) {
  const requete = new Request(`${ctx.origin}/__cache/${cle}`);
  const garde = await caches.default.match(requete);
  if (garde) return garde.text();

  const valeur = await charger();
  if (valeur === null || ctx.partiel) return valeur;

  const copie = new Response(valeur, {
    headers: { "content-type": JSON_UTF8, "cache-control": `public, max-age=${ttl}` },
  });
  ctx.exec.waitUntil(caches.default.put(requete, copie));
  return valeur;
}

// --- Le proxy -----------------------------------------------------------------------

const PROXY = "https://torii-igdb-proxy.toriiapp.workers.dev";

export const DELAI_MS = 15_000;

/**
 * Un appel au proxy, qui détient les clés IGDB et ITAD.
 *
 * 🔑 LA VRAIE ADRESSE, MÊME À TRAVERS LE SERVICE BINDING. Le proxy range ses réponses sous
 * l'adresse reçue : avec celle des clients PC, ce que l'un a demandé sert aussi à l'autre, et
 * le quota partagé du comparateur en profite.
 *
 * ⚠️ `cf-connecting-ip` n'existe pas sur un appel interne : sans ce relais, tous les
 * utilisateurs du web partageraient un seul compteur dans les limites par IP du proxy.
 *
 * Sans binding (exécution hors de Cloudflare), l'appel part simplement sur le réseau.
 */
export function versProxy(ctx, chemin, init = {}) {
  const url = `${PROXY}/${chemin}`;
  const options = { ...init, signal: AbortSignal.timeout(DELAI_MS) };
  if (!ctx.env.PROXY) return fetch(url, options);
  const headers = { ...init.headers };
  if (ctx.ip) headers["cf-connecting-ip"] = ctx.ip;
  return ctx.env.PROXY.fetch(url, { ...options, headers });
}
