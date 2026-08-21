/**
 * Torii — amis et présence.
 *
 * Deux principes qui expliquent la forme des routes :
 *
 * 1. **Un ami s'ajoute par code**, jamais par e-mail. Chercher quelqu'un par son
 *    adresse transformerait le service en annuaire : il suffirait de tester une liste
 *    d'adresses pour savoir qui utilise Torii. Le code d'ami se donne de la main à la
 *    main, et se régénère si on l'a trop diffusé.
 *
 * 2. **Publier sa présence renvoie celle des autres.** Le client bat le cœur toutes les
 *    30 s ; faire de cet appel la lecture des amis divise par deux le trafic (une requête
 *    au lieu de deux), ce qui compte quand le forfait gratuit se mesure en requêtes/jour.
 */

import { body, clamp, fail, json, newFriendCode, now } from "./lib.js";

/** Durée de vie d'une présence : ~3 battements manqués et la personne passe hors ligne. */
const PRESENCE_TTL = 90;
/** Bornes d'un lot de SteamID envoyé pour la suggestion d'amis. */
const MAX_STEAM_IDS = 200;

const STATUSES = new Set(["in-game", "online", "away"]);

/* ── Amis ──────────────────────────────────────────────────────────────────── */

/**
 * Liste complète : amis (avec leur présence), demandes reçues, demandes envoyées.
 * Une seule requête SQL — c'est l'appel le plus fréquent du service.
 */
async function loadCircle(env, accountId) {
  const rows = await env.DB.prepare(
    `SELECT a.id, a.display_name, a.friend_code, f.state, f.requester_id,
            p.status, p.game_key, p.game_title, p.since
       FROM friendships f
       JOIN accounts a
         ON a.id = CASE WHEN f.requester_id = ? THEN f.addressee_id ELSE f.requester_id END
       LEFT JOIN presence p ON p.account_id = a.id AND p.expires_at > ?
      WHERE f.requester_id = ? OR f.addressee_id = ?`,
  )
    .bind(accountId, now(), accountId, accountId)
    .all();

  const friends = [];
  const incoming = [];
  const outgoing = [];

  for (const r of rows.results || []) {
    const person = { id: r.id, displayName: r.display_name };
    if (r.state === "accepted") {
      friends.push({
        ...person,
        // Pas de ligne de présence valide = la personne n'a pas Torii ouvert. On le dit
        // ainsi plutôt que « hors ligne », qui laisserait croire qu'elle ne joue pas.
        status: r.status || "offline",
        gameKey: r.game_key || null,
        gameTitle: r.game_title || null,
        since: r.since || null,
      });
    } else if (r.requester_id === accountId) {
      outgoing.push(person);
    } else {
      incoming.push(person);
    }
  }

  // En jeu d'abord, puis en ligne, puis absents, puis hors ligne ; alphabétique à égalité.
  const rank = (s) => ({ "in-game": 0, online: 1, away: 2 }[s] ?? 3);
  friends.sort((a, b) => rank(a.status) - rank(b.status) || a.displayName.localeCompare(b.displayName, "fr"));

  return { friends, incoming, outgoing };
}

/** `GET /v1/friends` */
export async function listFriends(request, env, session) {
  return json(await loadCircle(env, session.accountId));
}

/** `POST /v1/friends/invite` — { friendCode } */
export async function invite(request, env, session) {
  const data = (await body(request)) || {};
  const code = clamp(data.friendCode, 16).toUpperCase().replace(/[^A-Z0-9]/g, "");
  if (code.length < 6) return fail(400, "code_invalide", "Ce code d'ami n'est pas valide.");

  const target = await env.DB.prepare("SELECT id, display_name FROM accounts WHERE friend_code = ?")
    .bind(code)
    .first();
  if (!target) return fail(404, "introuvable", "Aucun compte ne correspond à ce code.");
  if (target.id === session.accountId) {
    return fail(400, "soi_meme", "C'est ton propre code d'ami.");
  }

  const existing = await env.DB.prepare(
    `SELECT state, requester_id FROM friendships
      WHERE (requester_id = ? AND addressee_id = ?) OR (requester_id = ? AND addressee_id = ?)`,
  )
    .bind(session.accountId, target.id, target.id, session.accountId)
    .first();

  if (existing?.state === "accepted") {
    return fail(409, "deja_ami", "Vous êtes déjà amis.");
  }
  // L'autre nous avait déjà invité : inviter en retour vaut acceptation.
  if (existing?.state === "pending" && existing.requester_id === target.id) {
    await env.DB.prepare(
      "UPDATE friendships SET state = 'accepted' WHERE requester_id = ? AND addressee_id = ?",
    )
      .bind(target.id, session.accountId)
      .run();
    return json({ ok: true, state: "accepted", friend: { id: target.id, displayName: target.display_name } });
  }
  if (existing?.state === "pending") {
    return fail(409, "deja_envoyee", "Ta demande est déjà partie ; il faut qu'elle soit acceptée.");
  }

  await env.DB.prepare(
    "INSERT INTO friendships (requester_id, addressee_id, state, created_at) VALUES (?, ?, 'pending', ?)",
  )
    .bind(session.accountId, target.id, now())
    .run();
  return json({ ok: true, state: "pending", friend: { id: target.id, displayName: target.display_name } });
}

/** `POST /v1/friends/respond` — { accountId, accept } : répond à une demande reçue. */
export async function respond(request, env, session) {
  const data = (await body(request)) || {};
  const from = clamp(data.accountId, 40);
  if (!from) return fail(400, "requete_invalide", "Demande introuvable.");

  const pending = await env.DB.prepare(
    "SELECT 1 FROM friendships WHERE requester_id = ? AND addressee_id = ? AND state = 'pending'",
  )
    .bind(from, session.accountId)
    .first();
  if (!pending) return fail(404, "introuvable", "Cette demande n'existe plus.");

  if (data.accept) {
    await env.DB.prepare(
      "UPDATE friendships SET state = 'accepted' WHERE requester_id = ? AND addressee_id = ?",
    )
      .bind(from, session.accountId)
      .run();
  } else {
    await env.DB.prepare("DELETE FROM friendships WHERE requester_id = ? AND addressee_id = ?")
      .bind(from, session.accountId)
      .run();
  }
  return json({ ok: true });
}

/** `DELETE /v1/friends/{id}` — retire un ami ou annule une demande, dans les deux sens. */
export async function removeFriend(request, env, session, id) {
  await env.DB.prepare(
    `DELETE FROM friendships
      WHERE (requester_id = ? AND addressee_id = ?) OR (requester_id = ? AND addressee_id = ?)`,
  )
    .bind(session.accountId, id, id, session.accountId)
    .run();
  return json({ ok: true });
}

/** `POST /v1/friends/code` — régénère son code d'ami (l'ancien cesse aussitôt de marcher). */
export async function rotateCode(request, env, session) {
  const code = newFriendCode();
  await env.DB.prepare("UPDATE accounts SET friend_code = ? WHERE id = ?")
    .bind(code, session.accountId)
    .run();
  return json({ friendCode: code });
}

/**
 * `POST /v1/friends/suggestions` — { steamIds } : lesquels de ces joueurs Steam sont
 * déjà sur Torii ?
 *
 * 🔑 Réservé aux comptes qui se sont eux-mêmes rendus découvrables, et ne renvoie que
 * des comptes découvrables. Sans cette double condition, n'importe qui pourrait tester
 * une liste de SteamID pour cartographier les utilisateurs de Torii.
 */
export async function suggestions(request, env, session) {
  if (!session.account.steam_discoverable) {
    return fail(
      403,
      "decouverte_desactivee",
      "Active « visible par mes amis Steam » pour utiliser les suggestions.",
    );
  }
  const data = (await body(request)) || {};
  const ids = Array.isArray(data.steamIds)
    ? data.steamIds.filter((v) => typeof v === "string" && /^\d{17}$/.test(v)).slice(0, MAX_STEAM_IDS)
    : [];
  if (!ids.length) return json({ suggestions: [] });

  const marks = ids.map(() => "?").join(",");
  const rows = await env.DB.prepare(
    `SELECT a.id, a.display_name, a.steam_id
       FROM accounts a
      WHERE a.steam_id IN (${marks})
        AND a.steam_discoverable = 1
        AND a.id <> ?
        AND NOT EXISTS (
          SELECT 1 FROM friendships f
           WHERE (f.requester_id = a.id AND f.addressee_id = ?)
              OR (f.addressee_id = a.id AND f.requester_id = ?)
        )`,
  )
    .bind(...ids, session.accountId, session.accountId, session.accountId)
    .all();

  return json({
    suggestions: (rows.results || []).map((r) => ({
      id: r.id,
      displayName: r.display_name,
      steamId: r.steam_id,
    })),
  });
}

/* ── Présence ──────────────────────────────────────────────────────────────── */

/**
 * `PUT /v1/presence` — publie son état ET récupère celui de ses amis.
 *
 * Le client appelle ça toutes les 30 s. Comme la réponse contient le cercle complet,
 * un seul aller-retour suffit pour tenir la liste d'amis à jour.
 */
export async function publishPresence(request, env, session) {
  const data = (await body(request)) || {};
  const status = STATUSES.has(data.status) ? data.status : "online";
  const gameKey = clamp(data.gameKey, 80) || null;
  const gameTitle = clamp(data.gameTitle, 120) || null;
  // Une date de début fournie par le client ne peut pas être dans le futur : les
  // horloges des machines dérivent, et un « depuis 3 h » erroné est très visible.
  const since = Number.isFinite(data.since) ? Math.min(Math.floor(data.since), now()) : null;

  await env.DB.prepare(
    `INSERT INTO presence (account_id, status, game_key, game_title, since, expires_at)
     VALUES (?, ?, ?, ?, ?, ?)
     ON CONFLICT(account_id) DO UPDATE SET
       status = excluded.status, game_key = excluded.game_key,
       game_title = excluded.game_title, since = excluded.since,
       expires_at = excluded.expires_at`,
  )
    .bind(session.accountId, status, gameKey, gameTitle, since, now() + PRESENCE_TTL)
    .run();

  return json(await loadCircle(env, session.accountId));
}

/** `DELETE /v1/presence` — disparaître immédiatement (mode invisible, déconnexion). */
export async function clearPresence(request, env, session) {
  await env.DB.prepare("DELETE FROM presence WHERE account_id = ?").bind(session.accountId).run();
  return json({ ok: true });
}
