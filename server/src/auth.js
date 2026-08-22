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
/** Validité du laissez-passer d'inscription : le temps de choisir un pseudo, pas plus. */
const SIGNUP_TTL = 15 * 60;

function b64url(texte) {
  const octets = new TextEncoder().encode(texte);
  return btoa(String.fromCharCode(...octets)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function deb64url(texte) {
  const brut = atob(texte.replace(/-/g, "+").replace(/_/g, "/"));
  return new TextDecoder().decode(Uint8Array.from(brut, (c) => c.charCodeAt(0)));
}

/**
 * Laissez-passer d'inscription : `<adresse base64url>.<expiration>.<signature>`.
 *
 * 🔑 Ni table ni ligne à nettoyer : la signature au poivre du serveur suffit à prouver
 * que cette adresse vient d'être vérifiée ici. C'est tout l'objet de la manœuvre — tant
 * que le pseudo n'est pas choisi, RIEN n'existe en base.
 */
async function emettreLaissezPasser(email, env) {
  const charge = `${b64url(email)}.${now() + SIGNUP_TTL}`;
  return `${charge}.${await hash(charge, env.PEPPER)}`;
}

/** Renvoie l'adresse portée par le laissez-passer, ou `null` s'il ne vaut rien. */
async function lireLaissezPasser(jeton, env) {
  const morceaux = String(jeton || "").split(".");
  if (morceaux.length !== 3) return null;
  const [adresse, expiration, signature] = morceaux;
  if (!/^\d+$/.test(expiration) || Number(expiration) < now()) return null;
  if (!sameHash(await hash(`${adresse}.${expiration}`, env.PEPPER), signature)) return null;
  try {
    const email = normalizeEmail(deb64url(adresse));
    return looksLikeEmail(email) ? email : null;
  } catch {
    return null;
  }
}

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

/** Corps du message, en texte et en HTML — les deux, pour le rendu et pour l'antispam. */
function codeMessage(code) {
  return {
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
  };
}

/**
 * Achemine le code. Trois modes, choisis par la configuration présente — dans cet ordre :
 *
 *   1. **Resend** (`RESEND_API_KEY`) : envoi HTTP depuis le Worker. C'est la voie retenue,
 *      parce que l'envoi natif de Cloudflare exige le plan Workers payant.
 *   2. **Cloudflare Email Sending** (binding `EMAIL`) : gardé pour le jour où le plan
 *      change — décommenter `[[send_email]]` suffira alors à basculer.
 *   3. **développement** (`DEV_CODES=1`) : le code revient dans la réponse HTTP, ce qui
 *      permet de tester sans aucun service d'envoi. ⚠️ Jamais en production : il suffit
 *      alors de lire la réponse pour se connecter avec n'importe quelle adresse.
 */
async function deliver(env, email, code) {
  const from = env.EMAIL_FROM || "torii@example.com";
  const msg = codeMessage(code);

  if (env.RESEND_API_KEY) {
    const r = await fetch("https://api.resend.com/emails", {
      method: "POST",
      headers: {
        authorization: `Bearer ${env.RESEND_API_KEY}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({ from: `Torii <${from}>`, to: [email], ...msg }),
    });
    if (!r.ok) {
      // Le détail part dans les journaux, jamais au client : il contient l'adresse visée
      // et la raison exacte du refus.
      console.error("resend", r.status, await r.text());
      return fail(502, "envoi_impossible", "Impossible d'envoyer le code pour l'instant.");
    }
    return json({ ok: true, sent: true });
  }

  if (env.EMAIL) {
    await env.EMAIL.send({ to: email, from: { email: from, name: "Torii" }, ...msg });
    return json({ ok: true, sent: true });
  }

  if (env.DEV_CODES === "1") {
    return json({ ok: true, sent: false, devCode: code });
  }

  return fail(503, "email_indisponible", "L'envoi d'e-mails n'est pas configuré sur ce serveur.");
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

  // Première connexion = inscription. Le client s'en sert pour proposer de choisir un
  // pseudo tout de suite, plutôt que de laisser le nom dérivé de l'adresse e-mail.
  const created = !account;

  // 🔑 Inscription différée, quand le client le demande. On ne crée rien ici : on renvoie
  // un laissez-passer, et le compte n'existera qu'une fois le pseudo choisi. Sans ça,
  // fermer la fenêtre au milieu laisse derrière soi un compte jamais terminé, portant un
  // pseudo tiré de l'adresse e-mail que personne n'a validé.
  //
  // ⚠️ Sans le drapeau : comportement d'avant, à l'identique. Les clients déjà installés
  // chez les joueurs continuent de s'inscrire comme ils l'ont toujours fait.
  if (created && data.deferProfile) {
    return json({
      created: true,
      needsProfile: true,
      signupToken: await emettreLaissezPasser(email, env),
    });
  }

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

  return json({ token, created, account: publicAccount(account) });
}

/**
 * `POST /v1/auth/signup` — { signupToken, displayName } : crée enfin le compte.
 *
 * C'est le SEUL endroit où naît un compte en inscription différée. Pas de pseudo, pas de
 * compte : c'est exactement la garantie demandée côté application.
 */
export async function signup(request, env) {
  const data = (await body(request)) || {};
  const displayName = clamp(data.displayName, 40).trim();
  const device = clamp(data.device, 60) || "inconnu";
  if (!displayName) return fail(400, "pseudo_manquant", "Choisis un pseudo.");

  const email = await lireLaissezPasser(data.signupToken, env);
  if (!email) {
    return fail(400, "inscription_expiree", "Cette inscription a expiré. Recommence depuis ton adresse.");
  }

  // Un laissez-passer rejoué, ou deux fenêtres ouvertes en même temps : le compte déjà
  // créé l'emporte, et on se contente d'ouvrir une session dessus. Jamais de doublon.
  let account = await env.DB.prepare(
    "SELECT id, email, display_name, friend_code, steam_id, steam_discoverable FROM accounts WHERE email = ?",
  )
    .bind(email)
    .first();
  const created = !account;

  if (!account) {
    account = {
      id: newId(),
      email,
      display_name: displayName,
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

  return json({ token, created, account: publicAccount(account) });
}

/**
 * `DELETE /v1/me` — supprime le compte et tout ce qui s'y rattache.
 *
 * 🔑 Suppression explicite table par table, plutôt que de s'en remettre au
 * `ON DELETE CASCADE` du schéma : si les clés étrangères ne sont pas appliquées — ce qui
 * ne se voit pas, ça ne lève aucune erreur — on laisserait derrière soi des amitiés vers
 * un compte disparu et un code de connexion encore valable pour cette adresse. La cascade
 * reste en place, mais comme filet, pas comme mécanisme.
 *
 * `batch()` s'exécute en transaction : tout part, ou rien ne part.
 */
export async function deleteMe(request, env, session) {
  const id = session.accountId;
  const email = session.account.email;
  await env.DB.batch([
    env.DB.prepare("DELETE FROM presence WHERE account_id = ?").bind(id),
    // Les deux sens : une amitié est une seule ligne, orientée par qui a demandé.
    env.DB.prepare("DELETE FROM friendships WHERE requester_id = ? OR addressee_id = ?").bind(id, id),
    // Toutes les sessions, pas seulement celle-ci : les autres appareils doivent tomber.
    env.DB.prepare("DELETE FROM sessions WHERE account_id = ?").bind(id),
    // Un code en cours de route ne doit pas ressusciter l'adresse.
    env.DB.prepare("DELETE FROM login_codes WHERE email = ?").bind(email),
    env.DB.prepare("DELETE FROM accounts WHERE id = ?").bind(id),
  ]);
  return json({ ok: true });
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
    // 🔑 Un compte Steam ne se relie qu'à un seul compte Torii. Sans ça, la même personne
    // apparaît deux fois chez ses amis — dont une ligne inerte — et n'importe qui peut
    // rattacher à son compte le SteamID d'un autre pour en porter l'avatar.
    if (steamId) {
      const pris = await env.DB.prepare(
        "SELECT id FROM accounts WHERE steam_id = ? AND id <> ?",
      )
        .bind(steamId, session.accountId)
        .first();
      if (pris) {
        return fail(
          409,
          "steam_deja_lie",
          "Ce compte Steam est déjà relié à un autre compte Torii. Délie-le depuis ce compte avant de le rattacher ici.",
        );
      }
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
  try {
    await env.DB.prepare(`UPDATE accounts SET ${patch.join(", ")} WHERE id = ?`)
      .bind(...values)
      .run();
  } catch (e) {
    // L'index unique tranche les courses que le contrôle ci-dessus ne peut pas voir :
    // deux requêtes parties en même temps pour le même SteamID.
    if (String(e).includes("UNIQUE")) {
      return fail(
        409,
        "steam_deja_lie",
        "Ce compte Steam vient d'être relié à un autre compte Torii.",
      );
    }
    throw e;
  }

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
