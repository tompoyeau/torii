/**
 * Torii — mini-proxy IGDB + IsThereAnyDeal (archi Playnite).
 *
 * Deux services dont la clé ne peut pas être embarquée dans l'app distribuée :
 *   - IGDB : auth Twitch (Client-ID + token d'app). POST /<endpoint> (Apicalypse).
 *   - IsThereAnyDeal (ITAD) : clé d'API. Tout chemin sous /itad/* est relayé vers
 *     https://api.isthereanydeal.com/* avec la clé injectée côté serveur (GET et POST).
 *
 * L'app Torii appelle ce Worker, jamais IGDB/ITAD directement.
 *
 * Secrets Cloudflare (voir README) :
 *   - TWITCH_CLIENT_ID       : Client ID de l'app Twitch (IGDB)
 *   - TWITCH_CLIENT_SECRET   : Client Secret de l'app Twitch (IGDB)
 *   - ITAD_API_KEY           : clé d'API IsThereAnyDeal (gratuite, isthereanydeal.com/apps/my/)
 *   - PROXY_TOKEN (option)   : jeton partagé pour limiter l'usage (header x-proxy-token)
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

const ALLOWED = new Set(["games", "external_games", "genres", "covers", "multiquery"]);

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "content-type, x-proxy-token",
};

/**
 * Relaye une requête vers l'API IsThereAnyDeal en injectant la clé côté serveur.
 * Le chemin `/itad/<reste>` devient `https://api.isthereanydeal.com/<reste>` ; la
 * query string est conservée, `key` est ajoutée. GET et POST supportés.
 */
async function proxyItad(request, url, env) {
  if (!env.ITAD_API_KEY) {
    return new Response("ITAD_API_KEY manquant côté serveur", { status: 500, headers: CORS });
  }
  const rest = url.pathname.slice("/itad/".length);
  const target = new URL(`https://api.isthereanydeal.com/${rest}`);
  for (const [k, v] of url.searchParams) target.searchParams.set(k, v);
  target.searchParams.set("key", env.ITAD_API_KEY);

  const init = { method: request.method, headers: { Accept: "application/json" } };
  if (request.method === "POST") {
    init.headers["content-type"] = "application/json";
    init.body = await request.text();
  }
  const resp = await fetch(target, init);
  const text = await resp.text();
  return new Response(text, {
    status: resp.status,
    headers: { ...CORS, "content-type": "application/json" },
  });
}

export default {
  async fetch(request, env) {
    if (request.method === "OPTIONS") return new Response(null, { headers: CORS });

    const url = new URL(request.url);

    // Jeton partagé optionnel pour limiter l'abus du proxy public (IGDB + ITAD).
    if (env.PROXY_TOKEN && request.headers.get("x-proxy-token") !== env.PROXY_TOKEN) {
      return new Response("unauthorized", { status: 401, headers: CORS });
    }

    // --- IsThereAnyDeal (GET/POST) ---
    if (url.pathname.startsWith("/itad/")) {
      return proxyItad(request, url, env);
    }

    // --- IGDB (POST uniquement) ---
    if (request.method !== "POST") {
      return new Response("POST only", { status: 405, headers: CORS });
    }
    const endpoint = url.pathname.replace(/^\/+/, "");
    if (!ALLOWED.has(endpoint)) {
      return new Response(`endpoint interdit: ${endpoint}`, { status: 403, headers: CORS });
    }

    let token;
    try {
      token = await getAppToken(env);
    } catch (e) {
      return new Response(`auth error: ${e}`, { status: 502, headers: CORS });
    }

    const body = await request.text();
    const resp = await fetch(`https://api.igdb.com/v4/${endpoint}`, {
      method: "POST",
      headers: {
        "Client-ID": env.TWITCH_CLIENT_ID,
        Authorization: `Bearer ${token}`,
        Accept: "application/json",
      },
      body,
    });

    const text = await resp.text();
    return new Response(text, {
      status: resp.status,
      headers: { ...CORS, "content-type": "application/json" },
    });
  },
};
