/**
 * Torii — mini-proxy IGDB + IsThereAnyDeal (archi Playnite).
 *
 * Deux services dont la clé ne peut pas être embarquée dans l'app distribuée :
 *   - IGDB : auth Twitch (Client-ID + token d'app). POST /<endpoint> (Apicalypse).
 *   - IsThereAnyDeal (ITAD) : clé d'API. GET/POST /itad/<endpoint>, la clé injectée ici.
 *
 * L'app Torii appelle ce Worker, jamais IGDB/ITAD directement.
 *
 * ## Ce Worker est public, et c'est ça qu'il faut avoir en tête
 *
 * Son URL part en clair dans chaque version installée : elle s'extrait du binaire en
 * quelques secondes. Aucun secret partagé ne peut donc distinguer Torii d'un script —
 * `PROXY_TOKEN` compris, puisqu'il faudrait l'embarquer lui aussi. Il ne reste que deux
 * défenses, et ce sont exactement les deux mises en place ici :
 *
 *   1. **le cache** — une réponse servie depuis le cache ne coûte ni appel IGDB, ni appel
 *      ITAD, ni token Twitch. C'est la meilleure des deux, parce qu'elle sert aussi les
 *      joueurs : tout le monde possède Fortnite, Minecraft ou Valorant, et la première
 *      personne qui les demande paie pour toutes les suivantes ;
 *   2. **une limite par IP** — elle borne ce qu'une seule source peut consommer, sans
 *      jamais gêner un lancement normal (l'app se throttle déjà à 300 ms par appel).
 *
 * ⚠️ L'enjeu n'est pas la facture — tout tient sur l'offre gratuite. C'est que Twitch
 * révoque l'application IGDB en cas d'abus : Torii perdrait d'un coup TOUTE sa métadonnée
 * descriptive, pour tout le monde, sans recours rapide. Et accessoirement que les 100 000
 * requêtes/jour de l'offre gratuite Workers, partagées avec `torii-api`, soient brûlées
 * par quelqu'un d'autre avant midi.
 *
 * ## Pas d'en-têtes CORS, et c'est volontaire
 *
 * Aucun client de Torii n'est un navigateur : tout part de Rust (`ureq`), et demain d'un
 * client mobile — ni l'un ni l'autre n'applique la politique d'origine. Annoncer
 * `Access-Control-Allow-Origin: *` n'aurait donc servi personne, sauf une page web tierce
 * qui aurait pu faire marteler ce proxy par le navigateur de ses visiteurs : autant
 * d'adresses IP différentes, donc autant de limites par IP contournées. Si un vrai client
 * web voit le jour un jour, ce sera une ligne à remettre — en connaissance de cause.
 *
 * Secrets Cloudflare (voir README) :
 *   - TWITCH_CLIENT_ID       : Client ID de l'app Twitch (IGDB)
 *   - TWITCH_CLIENT_SECRET   : Client Secret de l'app Twitch (IGDB)
 *   - ITAD_API_KEY           : clé d'API IsThereAnyDeal
 *   - PROXY_TOKEN (option)   : interrupteur d'urgence, cf. plus bas
 *
 * Bindings (wrangler.toml) : RL_TOTAL et RL_AMONT, les deux compteurs par IP.
 */

// Cache du token d'app Twitch, par isolate (les tokens durent ~60 jours).
let cachedToken = null; // { value, expiresAt }

async function getAppToken(env) {
  const now = Date.now();
  if (cachedToken && cachedToken.expiresAt > now + 60_000) return cachedToken.value;
  const url =
    "https://id.twitch.tv/oauth2/token" +
    `?client_id=${encodeURIComponent(env.TWITCH_CLIENT_ID)}` +
    `&client_secret=${encodeURIComponent(env.TWITCH_CLIENT_SECRET)}` +
    "&grant_type=client_credentials";
  const r = await fetch(url, { method: "POST" });
  if (!r.ok) throw new Error(`twitch token ${r.status}: ${await r.text()}`);
  const j = await r.json();
  cachedToken = { value: j.access_token, expiresAt: now + j.expires_in * 1000 };
  return cachedToken.value;
}

// --- Ce qu'on accepte de relayer -------------------------------------------

/** Endpoints IGDB autorisés. Tout le reste est refusé, y compris ce qui existe chez IGDB. */
const IGDB_ALLOWED = new Set(["games", "external_games", "genres", "covers", "multiquery"]);

/**
 * Endpoints ITAD autorisés — liste relevée sur TOUTES les versions de `store.rs`, y
 * compris celles déjà installées chez les joueurs (`git log -p` sur le fichier).
 *
 * 🔑 Sans cette liste, `/itad/<n'importe quoi>` relayait la clé ITAD vers n'importe quel
 * endpoint de l'API, y compris ceux qui écrivent. Un allowlist ne coûte rien ici : le
 * client n'en appellera jamais d'autre sans qu'on touche à ce fichier.
 */
const ITAD_ALLOWED = new Set([
  "deals/v2",
  "games/search/v1",
  "games/info/v2",
  "games/overview/v2",
  "games/prices/v3",
  "games/lookup/v1",
]);

/**
 * Une requête Apicalypse tient en quelques centaines d'octets ; le plus gros lot Steam
 * (400 appids) fait ~4 Ko. 16 Ko laisse de la marge et ferme la porte à l'idée d'utiliser
 * ce Worker comme tuyau.
 */
const MAX_CORPS = 16 * 1024;
const MAX_CHEMIN = 200;
/** La query string est recopiée telle quelle vers ITAD : elle se borne aussi. */
const MAX_QUERY = 512;

// --- Cache -----------------------------------------------------------------

/**
 * Durée de cache (secondes) par endpoint.
 *
 * IGDB décrit des jeux : un nom, un genre, une jaquette, une année. Ça ne bouge
 * pratiquement jamais, et le client garde de toute façon sa propre copie sur disque
 * (`igdb_meta_cache_v1.json`). Le cache d'ici sert donc à **mutualiser entre joueurs**,
 * pas à éviter des allers-retours au même joueur.
 *
 * ITAD décrit des prix : ça bouge tous les jours, d'où des durées bien plus courtes.
 */
const TTL_IGDB = { genres: 604800 /* 7 j */ };
const TTL_IGDB_DEFAUT = 86400; // 24 h

function ttlItad(endpoint) {
  if (endpoint === "deals/v2") return 600; // 10 min : vitrine commune à tout le monde
  if (endpoint.startsWith("games/search/")) return 300; // 5 min : autocomplétion
  if (endpoint.startsWith("games/")) return 300; // 5 min : info/overview/lookup d'un jeu
  return 0; // le reste (prix en POST) n'est pas caché ici
}

/**
 * Clé de cache pour une requête POST : la méthode interdit d'utiliser la requête telle
 * quelle, et le corps (la requête Apicalypse, ou la liste d'ids ITAD) est ce qui la
 * distingue. On fabrique donc une URL GET synthétique portant son empreinte.
 *
 * 🔑 Bâtie sur `origin` de la requête entrante : le cache de Cloudflare est cloisonné par
 * zone, une clé pointant ailleurs ne serait jamais relue. Le chemin `/__cache/…` ne
 * correspond à aucune route — s'il est appelé directement, il tombe sur un 405.
 */
async function cleDeCache(origin, prefixe, corps) {
  const octets = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(corps));
  const empreinte = [...new Uint8Array(octets)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return new Request(`${origin}/__cache/${prefixe}?q=${empreinte}`, { method: "GET" });
}

// --- Réponses --------------------------------------------------------------

function texte(status, message) {
  return new Response(message, {
    status,
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}

/**
 * 429 avec `Retry-After`. Le client Rust le distingue d'une panne : il arrête sa passe de
 * métadonnées au lieu de brûler 200 appels de plus, et surtout il ne mémorise pas les
 * jeux concernés comme « absents d'IGDB » (cf. `metadata/igdb.rs`).
 */
function tropDeRequetes() {
  return new Response("trop de requêtes, réessaie dans une minute", {
    status: 429,
    headers: { "retry-after": "60", "content-type": "text/plain; charset=utf-8" },
  });
}

/** Corps de la requête, ou `null` s'il dépasse la borne. */
async function corpsBorne(request) {
  if (Number(request.headers.get("content-length") || 0) > MAX_CORPS) return null;
  // Un envoi en `chunked` n'annonce aucune taille : on revérifie après lecture.
  const corps = await request.text();
  return corps.length > MAX_CORPS ? null : corps;
}

// --- Relais ----------------------------------------------------------------

/**
 * Relaye vers IGDB. POST uniquement (Apicalypse), endpoint sur liste blanche, réponse
 * mise en cache par empreinte de la requête.
 */
async function relaisIgdb(request, url, env, ctx, ip) {
  if (request.method !== "POST") return texte(405, "POST only");

  const endpoint = url.pathname.replace(/^\/+/, "");
  if (!IGDB_ALLOWED.has(endpoint)) return texte(403, `endpoint interdit: ${endpoint}`);

  const corps = await corpsBorne(request);
  if (corps === null) return texte(413, "requête trop longue");

  const cache = caches.default;
  const cle = await cleDeCache(url.origin, `igdb/${endpoint}`, corps);
  const garde = await cache.match(cle);
  if (garde) return garde;

  // 🔑 La limite ne se décompte qu'ici : une réponse servie par le cache ne coûte rien à
  // IGDB, donc rien ne justifie de la faire entrer dans le quota de qui la demande. Un
  // joueur au premier lancement croise surtout des jeux déjà demandés par d'autres ; un
  // script qui fabrique des requêtes inédites, lui, paie chacune des siennes.
  if (!(await env.RL_AMONT.limit({ key: ip })).success) return tropDeRequetes();

  let token;
  try {
    token = await getAppToken(env);
  } catch (e) {
    return texte(502, `auth error: ${e}`);
  }

  const resp = await fetch(`https://api.igdb.com/v4/${endpoint}`, {
    method: "POST",
    headers: {
      "Client-ID": env.TWITCH_CLIENT_ID,
      Authorization: `Bearer ${token}`,
      Accept: "application/json",
    },
    body: corps,
  });

  const ttl = TTL_IGDB[endpoint] ?? TTL_IGDB_DEFAUT;
  const out = new Response(await resp.text(), {
    status: resp.status,
    headers: {
      "content-type": "application/json",
      "cache-control": resp.ok ? `public, max-age=${ttl}` : "no-store",
    },
  });
  // Seuls les succès sont mémorisés : une panne IGDB ne doit pas se figer pour 24 h.
  if (resp.ok) ctx.waitUntil(cache.put(cle, out.clone()));
  return out;
}

/**
 * Relaye vers IsThereAnyDeal en injectant la clé côté serveur. `/itad/<endpoint>` devient
 * `https://api.isthereanydeal.com/<endpoint>` ; la query string est conservée, `key`
 * ajoutée en dernier (donc jamais dictée par le client).
 */
async function relaisItad(request, url, env, ctx, ip) {
  // Ce qui est autorisé se décide avant ce qui est configuré : sinon une clé manquante
  // répondrait « mal configuré » à une requête qui, de toute façon, était refusée.
  const endpoint = url.pathname.slice("/itad/".length).replace(/\/+$/, "");
  if (!ITAD_ALLOWED.has(endpoint)) return texte(403, `endpoint interdit: ${endpoint}`);

  if (!env.ITAD_API_KEY) return texte(500, "ITAD_API_KEY manquant côté serveur");

  const corps = request.method === "POST" ? await corpsBorne(request) : "";
  if (corps === null) return texte(413, "requête trop longue");

  const cache = caches.default;
  // 🔑 GET uniquement, donc jamais les prix : eux arrivent en POST, et une remise affichée
  // avec cinq minutes de retard vaudrait moins que les appels économisés. La clé est
  // l'URL entrante telle quelle, sans la clé ITAD — qui n'est ajoutée qu'en aval.
  const ttl = request.method === "GET" ? ttlItad(endpoint) : 0;
  const cle = ttl > 0 ? new Request(url.toString(), { method: "GET" }) : null;
  if (cle) {
    const garde = await cache.match(cle);
    if (garde) return garde;
  }

  if (!(await env.RL_AMONT.limit({ key: ip })).success) return tropDeRequetes();

  const cible = new URL(`https://api.isthereanydeal.com/${endpoint}`);
  for (const [k, v] of url.searchParams) cible.searchParams.set(k, v);
  cible.searchParams.set("key", env.ITAD_API_KEY);

  const init = { method: request.method, headers: { Accept: "application/json" } };
  if (request.method === "POST") {
    init.headers["content-type"] = "application/json";
    init.body = corps;
  }
  const resp = await fetch(cible, init);

  const out = new Response(await resp.text(), {
    status: resp.status,
    headers: {
      "content-type": "application/json",
      "cache-control": cle && resp.ok ? `public, max-age=${ttl}` : "no-store",
    },
  });
  if (cle && resp.ok) ctx.waitUntil(cache.put(cle, out.clone()));
  return out;
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);

    /**
     * Interrupteur d'urgence, pas une protection. Posé, il coupe le proxy pour tout ce qui
     * n'a pas le jeton — donc aussi pour toutes les versions de Torii déjà installées, qui
     * ne l'envoient pas. À n'utiliser que le jour où il faut fermer la porte en attendant
     * mieux. L'abus ordinaire, lui, est traité par le cache et les limites ci-dessous.
     */
    if (env.PROXY_TOKEN && request.headers.get("x-proxy-token") !== env.PROXY_TOKEN) {
      return texte(401, "unauthorized");
    }

    // Une erreur de configuration doit se voir tout de suite, pas produire un proxy
    // silencieusement grand ouvert — c'est exactement ce qui est arrivé à `PROXY_TOKEN`,
    // resté « optionnel » et jamais posé.
    if (!env.RL_TOTAL || !env.RL_AMONT) {
      return texte(503, "Worker mal configuré : limites par IP absentes");
    }

    if (url.pathname.length > MAX_CHEMIN || url.search.length > MAX_QUERY) {
      return texte(414, "requête trop longue");
    }

    // `cf-connecting-ip` est posée par Cloudflare et ne peut pas être usurpée par le
    // client (une valeur envoyée par lui est écrasée). Absente en local (`wrangler dev`).
    const ip = request.headers.get("cf-connecting-ip") || "local";

    // Compteur grossier sur TOUTES les requêtes, cache compris : il protège le quota de
    // requêtes du compte Cloudflare, que `torii-api` partage avec ce Worker.
    if (!(await env.RL_TOTAL.limit({ key: ip })).success) return tropDeRequetes();

    if (url.pathname.startsWith("/itad/")) return relaisItad(request, url, env, ctx, ip);
    return relaisIgdb(request, url, env, ctx, ip);
  },
};
