/**
 * Torii — le panneau d'administration : combien de monde, et depuis quand.
 *
 * 🔑 POURQUOI CE FICHIER EXISTE. La décision « quand passer au plan payant » se prend sur
 * une PENTE, pas sur une photo. Or rien n'enregistrait l'histoire du service : la présence
 * expire, les comptes ne font que grossir, et une requête SQL ne donne que l'instant
 * présent. Un chiffre qu'on ne relève pas ne se rattrape jamais — d'où le relevé, qui
 * commence à tourner bien avant qu'on en ait besoin.
 *
 * ⚠️ LA LISTE DES COMPTES (adresse, pseudo, dates) PASSE PAR ICI depuis le 26 septembre
 * 2026, à la demande de l'exploitant : c'est une porte de plus sur des données
 * personnelles. Elle reste fermée par défaut (`ADMIN_TOKEN`), et n'expose JAMAIS le
 * contenu des bibliothèques, les amitiés ni la présence d'une personne nommée — seulement
 * des décomptes par compte.
 *
 * Secret : ADMIN_TOKEN (`npx wrangler secret put ADMIN_TOKEN`). Absent, la route est
 * fermée — jamais ouverte : une erreur de configuration ne doit pas publier les chiffres.
 */

import { fail, hash, json, now, sameHash } from "./lib.js";
import { audience } from "./audience.js";
// Importée comme texte (règle par défaut de wrangler pour les .html).
import PAGE from "./admin.html";

/** Nombre de jours d'historique renvoyés. Au-delà, la courbe n'apprend plus rien. */
const FENETRE = 180;

/**
 * Plafond estimé de joueurs en ligne SIMULTANÉMENT, offre gratuite Cloudflare (cf. la
 * cadence adaptative du battement). Sert de repère sur le graphique — c'est un ordre de
 * grandeur calculé, pas une limite que le service ferait respecter.
 */
const PLAFOND = 160;

/**
 * Écrit (ou met à jour) le relevé du jour. Appelée par le cron, toutes les heures.
 *
 * 🔑 POURQUOI TOUTES LES HEURES ET PAS UNE FOIS PAR NUIT. Les cumuls se moquent de
 * l'heure, mais la présence simultanée non : relevée à 4 h du matin elle vaudrait zéro
 * tous les jours, et ne préviendrait de rien. Le relevé horaire ne fait que remonter le
 * maximum du jour (`MAX(pic_en_ligne, ?)`), ce qui donne un vrai pic quotidien pour
 * 24 écritures — contre 100 000 offertes.
 */
export async function releverStats(env) {
  const instant = now();
  const jour = new Date(instant * 1000).toISOString().slice(0, 10);

  const r = await env.DB.prepare(
    `SELECT (SELECT COUNT(*) FROM accounts)                                  AS comptes,
            (SELECT COUNT(*) FROM libraries)                                 AS biblios,
            (SELECT COUNT(*) FROM presence WHERE expires_at > ?1)            AS en_ligne,
            (SELECT COUNT(DISTINCT account_id) FROM sessions
              WHERE last_seen_at > ?2)                                       AS actifs_7j`,
  )
    .bind(instant, instant - 7 * 24 * 3600)
    .first();

  await env.DB.prepare(
    `INSERT INTO stats (jour, comptes, biblios, actifs_7j, pic_en_ligne, releve_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(jour) DO UPDATE SET
       comptes      = ?2,
       biblios      = ?3,
       actifs_7j    = ?4,
       pic_en_ligne = MAX(pic_en_ligne, ?5),
       releve_at    = ?6`,
  )
    .bind(jour, r.comptes, r.biblios, r.actifs_7j, r.en_ligne, instant)
    .run();
}

/**
 * Vrai si la requête porte le jeton d'administration.
 *
 * ⚠️ FERMÉ PAR DÉFAUT. Sans `ADMIN_TOKEN` configuré, personne n'entre — surtout pas tout
 * le monde. Comparaison à temps constant sur les empreintes, comme partout ailleurs :
 * comparer les jetons en clair fuirait la position du premier caractère faux.
 */
async function estAdmin(request, env) {
  if (!env.ADMIN_TOKEN) return false;
  const header = request.headers.get("authorization") || "";
  const jeton = header.startsWith("Bearer ") ? header.slice(7).trim() : "";
  if (!jeton) return false;
  const [a, b] = await Promise.all([
    hash(jeton, env.PEPPER),
    hash(env.ADMIN_TOKEN, env.PEPPER),
  ]);
  return sameHash(a, b);
}

/** `GET /v1/admin/stats` — l'historique, plus l'instant présent. */
export async function statsAdmin(request, env) {
  if (!(await estAdmin(request, env))) {
    // 404 et non 401 : une route d'administration n'a pas à confirmer son existence à qui
    // n'a pas le jeton. Le message est le même que pour n'importe quelle URL inconnue.
    return fail(404, "route_inconnue", "Cette route n'existe pas.");
  }

  const instant = now();
  const { results } = await env.DB.prepare(
    `SELECT jour, comptes, biblios, actifs_7j, pic_en_ligne
       FROM stats ORDER BY jour DESC LIMIT ?`,
  )
    .bind(FENETRE)
    .all();

  // Le relevé horaire peut dater d'une cinquantaine de minutes : la ligne « maintenant »
  // est lue en direct, sinon le panneau afficherait un passé récent en se disant à jour.
  const vif = await env.DB.prepare(
    `SELECT (SELECT COUNT(*) FROM accounts)                       AS comptes,
            (SELECT COUNT(*) FROM libraries)                      AS biblios,
            (SELECT COUNT(*) FROM presence WHERE expires_at > ?1) AS en_ligne,
            (SELECT COUNT(DISTINCT account_id) FROM sessions
              WHERE last_seen_at > ?2)                            AS actifs_7j`,
  )
    .bind(instant, instant - 7 * 24 * 3600)
    .first();

  // Un compte : qui, depuis quand, vu quand, sur combien d'appareils, avec quoi de
  // synchronisé. Aucune liste de jeux, aucun ami nommé.
  const comptes = await env.DB.prepare(
    `SELECT a.display_name AS nom, a.email, a.created_at AS cree,
            a.steam_id IS NOT NULL AS steam,
            (SELECT MAX(last_seen_at) FROM sessions s WHERE s.account_id = a.id)  AS vu,
            (SELECT COUNT(*) FROM sessions s WHERE s.account_id = a.id)            AS sessions,
            (SELECT COUNT(*) FROM libraries l WHERE l.account_id = a.id)           AS appareils,
            (SELECT MAX(game_count) FROM libraries l WHERE l.account_id = a.id)    AS jeux,
            (SELECT COUNT(*) FROM friendships f
              WHERE f.state = 'accepted'
                AND (f.requester_id = a.id OR f.addressee_id = a.id))             AS amis
       FROM accounts a ORDER BY a.created_at DESC`,
  ).all();

  const jours = Math.min(365, Math.max(7, Number(new URL(request.url).searchParams.get("jours")) || 30));

  return json({
    maintenant: { ...vif, instant },
    plafond: PLAFOND,
    jours: (results || []).reverse(),
    comptes: comptes.results || [],
    fenetre: jours,
    audience: await audience(env, jours),
  });
}

/**
 * `GET /admin` — la page. Servie sans jeton, et c'est volontaire : elle ne contient
 * aucune donnée, seulement le code qui va les demander. Le jeton est saisi par
 * l'administrateur et gardé dans son navigateur ; il ne transite que dans l'en-tête
 * `Authorization`, jamais dans l'URL (où il finirait dans les journaux et l'historique).
 *
 * Même origine que l'API : pas de CORS à négocier, et rien de tiers à charger.
 */
export function pageAdmin() {
  return new Response(PAGE, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      // Rien d'externe, aucune image distante, pas d'encadrement par un autre site.
      "content-security-policy":
        "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'",
      "referrer-policy": "no-referrer",
      "x-robots-tag": "noindex, nofollow",
    },
  });
}
