/**
 * Torii — la Boutique pour la version web : Steam choisit les jeux, le comparateur donne le prix.
 *
 * Un navigateur ne peut pas faire ce que fait l'application installée : Steam n'autorise pas les
 * appels depuis une page, et le proxy IGDB/ITAD non plus, volontairement. Ce Worker compose donc
 * les mêmes réponses que les commandes `store_*` de l'application, côté serveur, avec les mêmes
 * noms de champs : le front les affiche sans savoir d'où elles viennent.
 *
 * Distinct de `proxy/` (qui relaie sans rien composer et détient les clés) et de `server/` (qui
 * détient les comptes) : ce Worker ne détient ni clé, ni compte. Il passe par les deux autres via
 * des service bindings.
 *
 * 🔑 UNE RÉPONSE POUR TOUT LE MONDE. Tout ce qui sort d'ici est public et ne dépend que de la
 * région, de la langue et des revendeurs masqués : ce qu'un utilisateur a demandé sert aux autres
 * pendant dix minutes, sans un appel à Steam ni au comparateur.
 *
 * Les routes exigent une session Torii (`Authorization: Bearer <jeton>`) : ce n'est pas que les
 * données soient secrètes, c'est que le quota du comparateur est partagé avec l'application PC.
 *
 * Bindings (wrangler.toml) : PROXY, AUTH, RL_IP, RL_COMPTE, RL_ITAD. Variable : ORIGINES.
 */

import { empreinte, fail, json, lireContexte, memo } from "./lib.js";
import { lienPartage } from "./igdb.js";
import { deals, search, suggest, triItad } from "./offres.js";
import { game } from "./produit.js";
import { extras, home, idPourSteam, parcourir, rayonValide, shelf, tagsBoutique, valeurTri } from "./vitrine.js";

const MAX_ID = 4_294_967_295;
const MAX_DLC = 50;
const MAX_TAGS = 20;
const MAX_TAGS_PARCOURS = 10;
const MAX_PAGE = 200;
const MAX_RECHERCHE = 100;
const MAX_TITRE = 120;
const JEU = /^[0-9a-fA-F-]{8,40}$/;

/** À incrémenter si la forme des réponses change : les anciennes ne seront plus relues. */
const VERSION_CACHE = 1;

const DIX_MINUTES = 600;
const CINQ_MINUTES = 300;
const UN_JOUR = 86_400;

// --- Réponses ---------------------------------------------------------------------

/**
 * Sert `calcul()` depuis le cache. `null` = rien à montrer : 503, le front affiche « Réessayer ».
 * La clé porte la région, la langue et les revendeurs masqués, sauf pour ce qui n'en dépend pas.
 */
async function servir(ctx, nom, parties, ttl, calcul, { parLangue = true, parRegion = true, parExclus = true } = {}) {
  const distinctif = [
    ...parties,
    parRegion ? ctx.region : "",
    parLangue ? ctx.langue : "",
    parExclus ? ctx.exclusCle : "",
  ].join("\n");
  const cle = `v${VERSION_CACHE}/${nom}/${await empreinte(distinctif)}`;
  const texte = await memo(ctx, cle, ttl, async () => {
    const valeur = await calcul();
    return valeur === null ? null : JSON.stringify(valeur);
  });
  if (texte === null) {
    return fail(503, "indisponible", "Cette source ne répond pas pour l'instant. Réessaie dans une minute.");
  }
  return new Response(texte, { headers: { "content-type": "application/json; charset=utf-8" } });
}

/** Des entiers positifs séparés par des virgules, `max` au plus. */
function entiers(valeur, max) {
  return (valeur ?? "")
    .split(",")
    .filter((v) => /^\d{1,10}$/.test(v))
    .map(Number)
    .slice(0, max);
}

/** Un entier positif borné ; `defaut` s'il est absent ou illisible. */
function nombre(valeur, max, defaut = 0) {
  return /^\d{1,10}$/.test(valeur ?? "") ? Math.min(Number(valeur), max) : defaut;
}

// --- Routes -----------------------------------------------------------------------

/** Routes sans segment variable : chacune reçoit le contexte et la query string. */
const ROUTES = {
  "/v1/home": (ctx) => servir(ctx, "home", [], DIX_MINUTES, () => home(ctx)),

  "/v1/shelf": (ctx, q) => {
    const nom = q.get("kind") ?? "";
    const tag = nombre(q.get("tag"), MAX_ID, null);
    if (!rayonValide(nom, tag)) return fail(400, "rayon_inconnu", "Ce rayon n'existe pas.");
    return servir(ctx, "shelf", [nom, tag ?? ""], DIX_MINUTES, () => shelf(ctx, nom, tag));
  },

  "/v1/deals": (ctx, q) => {
    const page = nombre(q.get("page"), MAX_PAGE);
    const tri = q.get("sort") ?? "featured";
    return servir(ctx, "deals", [page, triItad(tri)], DIX_MINUTES, () => deals(ctx, page, tri));
  },

  "/v1/tags": (ctx) =>
    servir(ctx, "tags", [], UN_JOUR, () => tagsBoutique(ctx), { parRegion: false, parExclus: false }),

  "/v1/browse": (ctx, q) => {
    const tags = entiers(q.get("tags"), MAX_TAGS_PARCOURS);
    const tri = valeurTri(q.get("sort") ?? "");
    if (tri === null) return fail(400, "tri_inconnu", "Ce tri n'existe pas.");
    const page = nombre(q.get("page"), MAX_PAGE);
    return servir(ctx, "browse", [[...tags].sort().join(), tri, page], DIX_MINUTES, () =>
      parcourir(ctx, tags, tri, page),
    );
  },

  "/v1/extras": (ctx, q) => {
    const appId = nombre(q.get("appId"), MAX_ID, null);
    if (appId === null) return fail(400, "jeu_inconnu", "Il manque l'identifiant du jeu.");
    const dlc = entiers(q.get("dlc"), MAX_DLC);
    const tags = entiers(q.get("tags"), MAX_TAGS);
    return servir(ctx, "extras", [appId, dlc.join(), tags.join()], DIX_MINUTES, () => extras(ctx, appId, dlc, tags));
  },

  "/v1/search": (ctx, q) => {
    const recherche = (q.get("q") ?? "").trim().slice(0, MAX_RECHERCHE);
    return servir(ctx, "search", [recherche.toLowerCase()], CINQ_MINUTES, () => search(ctx, recherche), {
      parLangue: false,
    });
  },

  "/v1/suggest": (ctx, q) => {
    const recherche = (q.get("q") ?? "").trim().slice(0, MAX_RECHERCHE);
    return servir(ctx, "suggest", [recherche.toLowerCase()], CINQ_MINUTES, () => suggest(ctx, recherche), {
      parLangue: false,
      parRegion: false,
      parExclus: false,
    });
  },

  "/v1/share": (ctx, q) => {
    const titre = (q.get("titre") ?? "").trim().slice(0, MAX_TITRE);
    if (!titre) return json({ lien: null });
    return servir(ctx, "share", [titre.toLowerCase()], UN_JOUR, async () => ({ lien: await lienPartage(ctx, titre) }), {
      parLangue: false,
      parRegion: false,
      parExclus: false,
    });
  },
};

/** Routes à segment variable, essayées dans l'ordre. Les motifs bornent ce qu'ils acceptent. */
const DYNAMIQUES = [
  {
    motif: /^\/v1\/game\/([^/]+)$/,
    traiter: (ctx, q, id) => {
      if (!JEU.test(id)) return fail(400, "jeu_inconnu", "Cet identifiant de jeu n'est pas valable.");
      return servir(ctx, "game", [id.toLowerCase()], DIX_MINUTES, () => game(ctx, id), { parExclus: false });
    },
  },
  {
    motif: /^\/v1\/id-for-steam\/(\d{1,10})$/,
    traiter: (ctx, q, appid) =>
      servir(ctx, "id", [appid], UN_JOUR, async () => ({ id: await idPourSteam(ctx, Number(appid)) }), {
        parLangue: false,
        parRegion: false,
        parExclus: false,
      }),
  },
];

function trouverRoute(chemin) {
  if (Object.hasOwn(ROUTES, chemin)) return { traiter: ROUTES[chemin], params: [] };
  for (const { motif, traiter } of DYNAMIQUES) {
    const m = chemin.match(motif);
    if (m) return { traiter, params: m.slice(1) };
  }
  return null;
}

// --- Sessions ---------------------------------------------------------------------

/**
 * Sessions déjà vérifiées par ce processus : une lecture de base de moins à chaque requête. Une
 * session fermée continue donc de marcher ici cinq minutes au plus, ce qui est sans conséquence :
 * elle ne donne accès à rien de personnel.
 */
const SESSIONS = new Map();
const DUREE_SESSION_MS = 5 * 60_000;
const MAX_SESSIONS = 2_000;

/** Pour les tests : l'état de ce module survit d'un test à l'autre. */
export function oublierSessions() {
  SESSIONS.clear();
}

/**
 * Le compte d'un jeton : `{ id }`, `{ refuse: true }` ou `{ panne: true }`.
 *
 * ⚠️ Une panne n'est pas un refus : répondre 401 quand `torii-api` est tombé déconnecterait
 * chaque visiteur qui passe à ce moment-là.
 */
async function compteDe(request, env, ip) {
  const entete = request.headers.get("authorization") ?? "";
  if (!entete.startsWith("Bearer ") || entete.length > 200) return { refuse: true };

  const cle = await empreinte(entete);
  const connue = SESSIONS.get(cle);
  if (connue && connue.jusqua > Date.now()) return { id: connue.id };

  let reponse;
  try {
    const headers = { authorization: entete };
    if (ip) headers["cf-connecting-ip"] = ip;
    reponse = await env.AUTH.fetch("https://torii-api.topo-host.com/v1/me", { headers });
  } catch (e) {
    console.error("torii-api injoignable —", e);
    return { panne: true };
  }
  if (reponse.status === 401) {
    SESSIONS.delete(cle);
    return { refuse: true };
  }
  const id = reponse.ok ? (await reponse.json().catch(() => null))?.account?.id : null;
  if (typeof id !== "string") return { panne: true };

  if (SESSIONS.size >= MAX_SESSIONS) SESSIONS.clear();
  SESSIONS.set(cle, { id, jusqua: Date.now() + DUREE_SESSION_MS });
  return { id };
}

// --- Le Worker --------------------------------------------------------------------

function trop(message) {
  return fail(429, "trop_de_requetes", message, { "retry-after": "60" });
}

/** Une requête, sans s'occuper du CORS. */
async function traiter(request, env, exec) {
  // Une erreur de configuration doit se voir tout de suite, pas produire un service sans limite.
  if (!env.RL_IP || !env.RL_COMPTE || !env.RL_ITAD || !env.AUTH) {
    return fail(500, "mal_configure", "Le serveur n'est pas configuré.");
  }

  const url = new URL(request.url);
  const chemin = url.pathname.replace(/\/+$/, "") || "/";
  const ip = request.headers.get("cf-connecting-ip") || "local";

  if (!(await env.RL_IP.limit({ key: ip })).success) {
    return trop("Trop de requêtes depuis cette connexion. Réessaie dans une minute.");
  }
  if (chemin === "/" || chemin === "/v1") return json({ service: "torii-store", version: 1 });
  if (request.method !== "GET") return fail(405, "methode", "Cette route n'accepte que GET.");

  const route = trouverRoute(chemin);
  if (!route) return fail(404, "route_inconnue", "Cette route n'existe pas.");

  const compte = await compteDe(request, env, request.headers.get("cf-connecting-ip"));
  if (compte.refuse) return fail(401, "non_connecte", "Session expirée ou absente.");
  if (compte.panne) return fail(503, "indisponible", "Impossible de vérifier la session pour l'instant.");

  if (!(await env.RL_COMPTE.limit({ key: compte.id })).success) {
    return trop("Trop de requêtes pour ce compte. Réessaie dans une minute.");
  }

  const ctx = lireContexte(request, env, exec);
  const reponse = await route.traiter(ctx, url.searchParams, ...route.params);
  // Une source n'a pas répondu : le front sait que ce qu'il affiche est incomplet.
  if (ctx.partiel) reponse.headers.set("x-torii-partiel", "1");
  return reponse;
}

/**
 * En-têtes CORS : seulement pour les pages listées dans `ORIGINES`.
 *
 * 🔑 Pas d'étoile, contrairement à `server/`. Une page tierce ne doit pas pouvoir utiliser ce
 * Worker depuis le navigateur de ses visiteurs : ce sont autant d'adresses IP différentes, et
 * c'est le quota du comparateur qui paierait.
 */
function enTetesCors(request, env) {
  const origine = request.headers.get("origin");
  const permises = (env.ORIGINES ?? "").split(",").map((o) => o.trim()).filter(Boolean);
  if (!origine || !permises.includes(origine)) return { vary: "Origin" };
  return {
    vary: "Origin",
    "access-control-allow-origin": origine,
    "access-control-allow-methods": "GET, OPTIONS",
    "access-control-allow-headers": "authorization",
    "access-control-expose-headers": "x-torii-partiel, retry-after",
    "access-control-max-age": "86400",
  };
}

export default {
  async fetch(request, env, exec) {
    const cors = enTetesCors(request, env);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: cors });

    let reponse;
    try {
      reponse = await traiter(request, env, exec);
    } catch (e) {
      // Jamais de détail au client : une erreur décrirait l'intérieur du service.
      console.error(`${request.method} ${new URL(request.url).pathname} —`, e?.stack || e);
      reponse = fail(500, "erreur_serveur", "Une erreur est survenue côté serveur.");
    }
    for (const [nom, valeur] of Object.entries(cors)) reponse.headers.set(nom, valeur);
    return reponse;
  },
};
