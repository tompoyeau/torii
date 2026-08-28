/**
 * Torii — API du service social (comptes, amis, présence).
 *
 * Worker distinct du `proxy/` : celui-ci relaie IGDB/ITAD et ne retient rien, alors que
 * celui-là détient des comptes et des relations. Cycles de vie, secrets et risques
 * différents → déploiements séparés.
 *
 * Toutes les routes sont préfixées `/v1`. L'auto-updater fait cohabiter des versions de
 * Torii pendant des semaines, et une application mobile viendra s'y brancher : le
 * préfixe garantit qu'on pourra faire évoluer le contrat sans casser les anciens clients.
 *
 * Secrets (`npx wrangler secret put …`) :
 *   PEPPER     — chaîne aléatoire, sale les empreintes des codes et des jetons. OBLIGATOIRE.
 *   EMAIL_FROM — adresse d'expédition (domaine onboardé sur Email Sending).
 *   DEV_CODES  — « 1 » pour recevoir le code dans la réponse HTTP au lieu d'un e-mail.
 *                ⚠️ Phase de test uniquement : actif, il laisse entrer n'importe qui.
 */

import { CORS, fail, json } from "./lib.js";
import { authenticate, deleteMe, logout, me, requestCode, signup, updateMe, verifyCode } from "./auth.js";
import {
  clearPresence, invite, listFriends, publishPresence, removeFriend, respond,
  rotateCode, suggestions,
} from "./social.js";
import { forgetAll, forgetLibrary, libraryIndex, readLibrary, uploadLibrary } from "./library.js";

/** Routes accessibles sans jeton de session. */
const PUBLIC = {
  "POST /v1/auth/request-code": requestCode,
  "POST /v1/auth/verify": verifyCode,
  "POST /v1/auth/signup": signup,
};

/** Routes exigeant un jeton valide ; la session résolue leur est passée en 3ᵉ argument. */
const PRIVATE = {
  "GET /v1/me": me,
  "PATCH /v1/me": updateMe,
  "DELETE /v1/me": deleteMe,
  "POST /v1/auth/logout": logout,
  "GET /v1/friends": listFriends,
  "POST /v1/friends/invite": invite,
  "POST /v1/friends/respond": respond,
  "POST /v1/friends/code": rotateCode,
  "POST /v1/friends/suggestions": suggestions,
  "PUT /v1/presence": publishPresence,
  "DELETE /v1/presence": clearPresence,
  "PUT /v1/library": uploadLibrary,
  "GET /v1/library": libraryIndex,
  "DELETE /v1/library": forgetAll,
};

/**
 * Routes à segment variable, essayées dans l'ordre après les tables ci-dessus. Les
 * segments capturés sont passés au gestionnaire après la session.
 *
 * ⚠️ Les motifs bornent ce qu'ils acceptent (`{1,40}` sur un alphabet sûr) : un
 * identifiant de bibliothèque finit dans une clé R2, il n'a rien à faire avec un `/`.
 */
const DYNAMIC = [
  { method: "DELETE", pattern: /^\/v1\/friends\/([A-Za-z0-9_-]{1,40})$/, handler: removeFriend },
  {
    method: "GET",
    pattern: /^\/v1\/library\/([A-Za-z0-9_-]{1,40})\/([A-Za-z0-9_-]{1,40})$/,
    handler: readLibrary,
  },
  { method: "DELETE", pattern: /^\/v1\/library\/([A-Za-z0-9_-]{1,40})$/, handler: forgetLibrary },
];

export default {
  async fetch(request, env) {
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS });
    }

    const url = new URL(request.url);
    const path = url.pathname.replace(/\/+$/, "") || "/";

    if (path === "/" || path === "/v1") {
      return json({ service: "torii-api", version: 1 });
    }
    // Une erreur de configuration doit se voir tout de suite, pas produire des
    // empreintes calculées avec un poivre vide.
    if (!env.PEPPER) {
      return fail(500, "mal_configure", "Le serveur n'est pas configuré (PEPPER manquant).");
    }

    const key = `${request.method} ${path}`;

    const open = PUBLIC[key];
    if (open) return await run(open, request, env);

    const guarded = PRIVATE[key];
    if (guarded) {
      const session = await authenticate(request, env);
      if (!session) return fail(401, "non_connecte", "Session expirée ou absente.");
      return await run(guarded, request, env, session);
    }

    for (const route of DYNAMIC) {
      if (route.method !== request.method) continue;
      const found = path.match(route.pattern);
      if (!found) continue;
      const session = await authenticate(request, env);
      if (!session) return fail(401, "non_connecte", "Session expirée ou absente.");
      return await run(route.handler, request, env, session, ...found.slice(1));
    }

    return fail(404, "route_inconnue", "Cette route n'existe pas.");
  },
};

/**
 * Exécute un gestionnaire en transformant toute exception en 500 propre : une erreur
 * SQL ne doit jamais remonter au client (elle décrirait le schéma).
 */
async function run(handler, request, env, session, ...params) {
  try {
    return await handler(request, env, session, ...params);
  } catch (err) {
    console.error(`${request.method} ${new URL(request.url).pathname} —`, err?.stack || err);
    return fail(500, "erreur_serveur", "Une erreur est survenue côté serveur.");
  }
}
