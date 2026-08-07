/**
 * Torii — mini-proxy IGDB (archi Playnite).
 *
 * IGDB (base de données de jeux cross-plateforme, rachetée par Twitch) exige une
 * auth Twitch : Client-ID + un token d'app (client_credentials). Ce token ne peut
 * pas être embarqué dans l'app distribuée (secret extractible) → ce Worker le
 * détient côté serveur et proxifie les requêtes IGDB. L'app Torii appelle ce Worker,
 * jamais IGDB directement.
 *
 * Secrets Cloudflare (voir README) :
 *   - TWITCH_CLIENT_ID       : Client ID de l'app Twitch
 *   - TWITCH_CLIENT_SECRET   : Client Secret de l'app Twitch
 *   - PROXY_TOKEN (option)   : jeton partagé pour limiter l'usage (header x-proxy-token)
 *
 * Usage : POST /<endpoint> avec en corps une requête Apicalypse.
 *   endpoints autorisés : games, external_games, genres, covers, multiquery
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
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Access-Control-Allow-Headers": "content-type, x-proxy-token",
};

export default {
  async fetch(request, env) {
    if (request.method === "OPTIONS") return new Response(null, { headers: CORS });
    if (request.method !== "POST") {
      return new Response("POST only", { status: 405, headers: CORS });
    }

    const endpoint = new URL(request.url).pathname.replace(/^\/+/, "");
    if (!ALLOWED.has(endpoint)) {
      return new Response(`endpoint interdit: ${endpoint}`, { status: 403, headers: CORS });
    }

    // Jeton partagé optionnel pour limiter l'abus du proxy public.
    if (env.PROXY_TOKEN && request.headers.get("x-proxy-token") !== env.PROXY_TOKEN) {
      return new Response("unauthorized", { status: 401, headers: CORS });
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
