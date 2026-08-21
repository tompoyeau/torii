/**
 * Torii — comptes et connexion par **code à usage unique**.
 *
 * Aucun mot de passe n'existe dans ce service : on saisit son e-mail, on reçoit un code
 * à 6 chiffres, et la session qui en découle dure des mois. Conséquences voulues :
 *   * il n'y a rien à voler côté serveur (ni mot de passe, ni empreinte réutilisable) ;
 *   * la « récupération de compte » n'existe pas comme flux séparé — c'est la connexion ;
 *   * le même flux marchera tel quel sur mobile.
 */

import {
  body, clamp, fail, hash, json, looksLikeEmail, newFriendCode, newId,
  normalizeEmail, now, randomCode, randomToken, sameHash,
} from "./lib.js";

/** Validité d'un code de connexion. Assez court pour limiter la fenêtre d'attaque. */
const CODE_TTL = 10 * 60;
/** Délai minimal entre deux envois pour une même adresse (anti-spam et anti-abus). */
const RESEND_DELAY = 60;
/** Envois maximum par heure et par adresse, pour qu'on ne puisse pas inonder une boîte. */
const MAX_SENDS = 5;
const SEND_WINDOW = 3600;
/** Essais autorisés avant de brûler le code : 6 chiffres ne résistent qu'à ça près. */
const MAX_ATTEMPTS = 5;

/**
 * `POST /v1/auth/request-code` — envoie un code à l'adresse donnée.
 *
 * 🔑 Répond **toujours** la même chose, que l'adresse ait un compte ou non : sinon
 * cette route devient un moyen de savoir qui est inscrit sur Torii.
 */
export async function requestCode(request, env) {
  const data = (await body(request)) || {};
  const email = normalizeEmail(data.email);
  if (!looksLikeEmail(email)) {
    return fail(400, "email_invalide", "Cette adresse e-mail n'est pas valide.");
  }

  const existing = await env.DB.prepare(
    "SELECT sent_at, sends, window_start FROM login_codes WHERE email = ?",
  )
    .bind(email)
    .first();
  if (existing && now() - existing.sent_at < RESEND_DELAY) {
    return fail(429, "trop_frequent", "Un code vient d'être envoyé. Réessaie dans une minute.");
  }
  // Fenêtre glissante d'une heure : au-delà, l'adresse est mise au repos.
  const fresh = !existing || now() - existing.window_start > SEND_WINDOW;
  if (!fresh && existing.sends >= MAX_SENDS) {
    return fail(429, "trop_d_envois", "Trop de codes demandés. Réessaie dans une heure.");
  }
  const sends = fresh ? 1 : existing.sends + 1;
  const windowStart = fresh ? now() : existing.window_start;

  const code = randomCode();
  await env.DB.prepare(
    `INSERT INTO login_codes (email, code_hash, expires_at, attempts, sent_at, sends, window_start)
     VALUES (?, ?, ?, 0, ?, ?, ?)
     ON CONFLICT(email) DO UPDATE SET
       code_hash = excluded.code_hash, expires_at = excluded.expires_at,
       attempts = 0, sent_at = excluded.sent_at,
       sends = excluded.sends, window_start = excluded.window_start`,
  )
    .bind(email, await hash(code, env.PEPPER), now() + CODE_TTL, now(), sends, windowStart)
    .run();

  return await deliver(env, email, code);
}

/**
 * Achemine le code. Deux modes, choisis par la configuration :
 *   * **production** : binding `EMAIL` de Cloudflare Email Sending (exige un domaine
 *     à soi, onboardé via `wrangler email sending enable`) ;
 *   * **développement** : `DEV_CODES=1` renvoie le code dans la réponse HTTP, ce qui
 *     permet de tout tester sans domaine. ⚠️ À ne JAMAIS laisser actif en production :
 *     n'importe qui pourrait alors se connecter avec n'importe quelle adresse.
 */
async function deliver(env, email, code) {
  if (env.EMAIL) {
    const from = env.EMAIL_FROM || "torii@example.com";
    await env.EMAIL.send({
      to: email,
      from: { email: from, name: "Torii" },
      subject: `${code} — ton code de connexion Torii`,
      text: [
        `Ton code de connexion Torii : ${code}`,
        "",
        "Il est valable 10 minutes et ne sert qu'une fois.",
        "Si tu n'as rien demandé, ignore ce message : personne ne peut se connecter sans ce code.",
      ].join("\n"),
      html:
        `<p>Ton code de connexion Torii :</p>` +
        `<p style="font-size:30px;font-weight:700;letter-spacing:.18em;margin:16px 0">${code}</p>` +
        `<p>Il est valable 10 minutes et ne sert qu'une fois.</p>` +
        `<p style="color:#666">Si tu n'as rien demandé, ignore ce message : personne ne peut se connecter sans ce code.</p>`,
    });
    return json({ ok: true, sent: true });
  }

  if (env.DEV_CODES === "1") {
    return json({ ok: true, sent: false, devCode: code });
  }

  return fail(
    503,
    "email_indisponible",
    "L'envoi d'e-mails n'est pas configuré sur ce serveur.",
  );
}

/**
 * `POST /v1/auth/verify` — échange { email, code } contre un jeton de session.
 * Crée le compte au passage s'il n'existe pas : première connexion = inscription.
 */
export async function verifyCode(request, env) {
  const data = (await body(request)) || {};
  const email = normalizeEmail(data.email);
  const code = clamp(data.code, 6);
  const device = clamp(data.device, 60) || "inconnu";
  if (!looksLikeEmail(email) || code.length !== 6) {
    return fail(400, "requete_invalide", "Adresse ou code manquant.");
  }

  const row = await env.DB.prepare(
    "SELECT code_hash, expires_at, attempts FROM login_codes WHERE email = ?",
  )
    .bind(email)
    .first();
  if (!row || row.expires_at < now()) {
    return fail(400, "code_expire", "Ce code n'est plus valable. Demandes-en un nouveau.");
  }
  if (row.attempts >= MAX_ATTEMPTS) {
    return fail(429, "trop_d_essais", "Trop d'essais. Demande un nouveau code.");
  }
  if (!sameHash(await hash(code, env.PEPPER), row.code_hash)) {
    await env.DB.prepare("UPDATE login_codes SET attempts = attempts + 1 WHERE email = ?")
      .bind(email)
      .run();
    return fail(400, "code_incorrect", "Code incorrect.");
  }

  // Le code a servi : il disparaît immédiatement, il ne peut pas être rejoué.
  await env.DB.prepare("DELETE FROM login_codes WHERE email = ?").bind(email).run();

  let account = await env.DB.prepare(
    "SELECT id, email, display_name, friend_code, steam_id, steam_discoverable FROM accounts WHERE email = ?",
  )
    .bind(email)
    .first();

  if (!account) {
    account = {
      id: newId(),
      email,
      display_name: email.split("@")[0].slice(0, 40),
      friend_code: newFriendCode(),
      steam_id: null,
      steam_discoverable: 0,
    };
    await env.DB.prepare(
      `INSERT INTO accounts (id, email, display_name, friend_code, created_at)
       VALUES (?, ?, ?, ?, ?)`,
    )
      .bind(account.id, account.email, account.display_name, account.friend_code, now())
      .run();
  }

  const token = randomToken();
  await env.DB.prepare(
    `INSERT INTO sessions (token_hash, account_id, device, created_at, last_seen_at)
     VALUES (?, ?, ?, ?, ?)`,
  )
    .bind(await hash(token, env.PEPPER), account.id, device, now(), now())
    .run();

  return json({ token, account: publicAccount(account) });
}

/** `POST /v1/auth/logout` — révoque la session courante (les autres appareils restent connectés). */
export async function logout(request, env, session) {
  await env.DB.prepare("DELETE FROM sessions WHERE token_hash = ?").bind(session.tokenHash).run();
  return json({ ok: true });
}

/**
 * Résout l'en-tête `Authorization: Bearer …` en session. Renvoie `null` si le jeton est
 * absent ou inconnu — l'appelant décide alors du code d'erreur.
 */
export async function authenticate(request, env) {
  const header = request.headers.get("authorization") || "";
  const token = header.startsWith("Bearer ") ? header.slice(7).trim() : "";
  if (!token) return null;

  const tokenHash = await hash(token, env.PEPPER);
  const row = await env.DB.prepare(
    `SELECT s.token_hash, s.account_id, a.id, a.email, a.display_name, a.friend_code,
            a.steam_id, a.steam_discoverable
       FROM sessions s JOIN accounts a ON a.id = s.account_id
      WHERE s.token_hash = ?`,
  )
    .bind(tokenHash)
    .first();
  if (!row) return null;

  return { tokenHash, accountId: row.account_id, account: row };
}

/** `GET /v1/me` — le compte connecté. */
export async function me(request, env, session) {
  return json({ account: publicAccount(session.account) });
}

/**
 * `PATCH /v1/me` — nom affiché, SteamID, découvrabilité.
 *
 * Le SteamID sert à retrouver ses amis Steam déjà sur Torii ; `steamDiscoverable`
 * contrôle si les autres peuvent nous trouver ainsi. Les deux côtés doivent l'avoir
 * activé pour qu'une suggestion apparaisse (cf. `suggestions` dans social.js).
 */
export async function updateMe(request, env, session) {
  const data = (await body(request)) || {};
  const patch = [];
  const values = [];

  if (typeof data.displayName === "string") {
    const name = clamp(data.displayName, 40);
    if (!name) return fail(400, "nom_vide", "Le nom affiché ne peut pas être vide.");
    patch.push("display_name = ?");
    values.push(name);
  }
  if ("steamId" in data) {
    const steamId = clamp(data.steamId, 20);
    if (steamId && !/^\d{17}$/.test(steamId)) {
      return fail(400, "steamid_invalide", "Un SteamID compte 17 chiffres.");
    }
    patch.push("steam_id = ?");
    values.push(steamId || null);
  }
  if ("steamDiscoverable" in data) {
    patch.push("steam_discoverable = ?");
    values.push(data.steamDiscoverable ? 1 : 0);
  }
  if (!patch.length) return json({ account: publicAccount(session.account) });

  values.push(session.accountId);
  await env.DB.prepare(`UPDATE accounts SET ${patch.join(", ")} WHERE id = ?`)
    .bind(...values)
    .run();

  const fresh = await env.DB.prepare(
    "SELECT id, email, display_name, friend_code, steam_id, steam_discoverable FROM accounts WHERE id = ?",
  )
    .bind(session.accountId)
    .first();
  return json({ account: publicAccount(fresh) });
}

/** Vue d'un compte destinée à son propriétaire (camelCase, comme le reste du front). */
export function publicAccount(row) {
  return {
    id: row.id,
    email: row.email,
    displayName: row.display_name,
    friendCode: row.friend_code,
    steamId: row.steam_id || null,
    steamDiscoverable: !!row.steam_discoverable,
  };
}
