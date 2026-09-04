/**
 * Torii — synchronisation des bibliothèques : la charge utile dans **R2**, l'index dans **D1**.
 *
 * C'est la première donnée **durable** du service. Comptes et amitiés mis à part, tout le
 * reste périme tout seul (la présence en 90 s, les codes de connexion en 10 min) ; une
 * bibliothèque, elle, reste. Deux conséquences assumées, écrites ici pour qu'on ne les
 * redécouvre pas plus tard : la promesse « aucun historique » du README ne couvre QUE la
 * présence, et une copie de cette base dit ce que les gens **possèdent** — pas ce qu'ils
 * jouent, ni quand.
 *
 * ## Pourquoi R2 plutôt qu'une table de jeux
 *
 * Une ligne par jeu et par personne, c'est ~1 000 lignes écrites à chaque resynchronisation.
 * Le plan gratuit D1 en offre 100 000 par jour : une centaine de joueurs et le service
 * s'arrête. Dans R2, la même bibliothèque est **un seul objet** (~120 Ko, une opération),
 * les 10 Go gratuits en absorbent des dizaines de milliers, et le trafic sortant — celui
 * que l'application mobile va consommer — ne coûte rien.
 *
 * D1 garde en échange un **index minuscule** : une ligne par appareil (empreinte, date,
 * nombre de jeux). C'est ce qui permet de savoir **sans rien télécharger** si une
 * bibliothèque a changé. Ce que R2 ne sait pas faire — chercher — se fait côté client,
 * exactement comme `useFriendsCommon` croise déjà les jeux Steam aujourd'hui.
 *
 * ## Un objet par APPAREIL, jamais par compte
 *
 * Deux PC n'ont pas la même bibliothèque (launchers connectés et jeux installés
 * différents). S'ils écrivaient au même endroit, le dernier passé effacerait l'autre et
 * les deux se battraient indéfiniment. La clé R2 porte donc l'appareil, et « la
 * bibliothèque de quelqu'un » est l'union de ses appareils, faite côté client.
 *
 * ## Deux interrupteurs, et ils ne disent pas la même chose
 *
 *   * **synchroniser** (côté client, `social_prefs.json`) : envoyer sa bibliothèque au
 *     serveur, pour la retrouver sur son mobile. Rien ne part tant que c'est éteint.
 *   * **partager** (côté serveur, `accounts.share_library`) : autoriser ses amis à la
 *     consulter. Éteint, la bibliothèque reste disponible pour ses propres appareils,
 *     mais personne d'autre ne peut la lire.
 *
 * 🔑 C'est le **Worker qui garde la porte**, jamais R2 : le bucket n'est pas public, et un
 * objet n'en sort qu'après vérification du jeton, de l'amitié et du partage. L'exposer via
 * son domaine `r2.dev` rendrait toutes les bibliothèques lisibles par qui devine une URL.
 */

import { body, clamp, fail, json, now } from "./lib.js";

/** Format de l'objet stocké. À incrémenter si sa forme change de façon incompatible. */
const SNAPSHOT_VERSION = 1;

/**
 * Bornes de ce qu'on accepte d'écrire. Une route authentifiée qui écrit dans R2 devient un
 * hébergement de fichiers gratuit si on ne la borne pas : on ne stocke donc QUE les champs
 * connus, tronqués, et jamais le JSON reçu tel quel.
 */
const MAX_GAMES = 5000;
const MAX_BODY = 4 * 1024 * 1024;
const MAX_KEY = 80;
const MAX_TITLE = 120;
const MAX_COVER = 300;
const MAX_PLATFORMS = 6;
const MAX_PLATFORM = 16;
/** Appareils mémorisés par compte. Au-delà, c'est un client qui boucle, pas un joueur. */
const MAX_DEVICES = 5;

/** Identifiant d'appareil ou de compte : opaque et borné — il entre dans une clé R2. */
const ID = /^[A-Za-z0-9_-]{1,40}$/;

/** Clé de l'objet R2. Le compte d'abord : c'est ce qui rend le préfixe listable. */
const objectKey = (accountId, deviceId) => `lib/${accountId}/${deviceId}.json`;

/**
 * Retient d'un jeu ce qui sert à l'afficher et à le croiser, rien de plus.
 *
 * `key` est la clé cross-launcher (`igdb:1942`, sinon `title:<titre normalisé>`) — la même
 * que celle de la présence, pour qu'un jeu possédé sur GOG et un jeu joué sur Steam se
 * rejoignent. Renvoie `null` si l'entrée est inexploitable : mieux vaut un jeu manquant
 * qu'une case vide dans la grille d'un ami.
 */
function normalizeGame(raw) {
  if (!raw || typeof raw !== "object") return null;
  const key = clamp(raw.key, MAX_KEY);
  const title = clamp(raw.title, MAX_TITLE);
  if (!key || !title) return null;

  const platforms = Array.isArray(raw.platforms)
    ? [
        ...new Set(
          raw.platforms
            .filter((p) => typeof p === "string")
            .map((p) => p.trim().toLowerCase().slice(0, MAX_PLATFORM))
            .filter(Boolean),
        ),
      ].slice(0, MAX_PLATFORMS)
    : [];

  const game = { key, title, platforms };
  // ⚠️ HTTPS seulement : une jaquette en clair déclenche un avertissement de contenu mixte
  // dans la WebView, et un refus pur et simple sur mobile.
  const cover = clamp(raw.cover, MAX_COVER);
  if (cover.startsWith("https://")) game.cover = cover;
  // Jeu emprunté au groupe familial Steam : il est dans la bibliothèque sans lui
  // appartenir. Retenu comme un booléen strict — un client qui envoie autre chose ne
  // doit pas pouvoir glisser une chaîne dans l'objet R2. Absent = possédé, le cas
  // courant, et l'immense majorité des lignes n'a donc pas ce champ.
  if (raw.familyShared === true) game.familyShared = true;
  return game;
}

/** Le service est-il gréé ? Une erreur de configuration doit se voir, pas produire un 500 muet. */
function bucket(env) {
  return env.LIBS || null;
}

/**
 * `PUT /v1/library` — dépose la bibliothèque d'un appareil.
 *
 * Corps : `{ deviceId, deviceName, digest,
 *            games: [{ key, title, platforms, cover, familyShared }] }`.
 *
 * `digest` est une empreinte **opaque** calculée par le client. Le serveur ne la recalcule
 * pas : elle ne sert qu'à ce même client (et aux amis, pour leur cache) à savoir s'il faut
 * retélécharger. Mentir dessus ne trompe que soi.
 */
export async function uploadLibrary(request, env, session) {
  const libs = bucket(env);
  if (!libs) return fail(500, "mal_configure", "Le stockage des bibliothèques n'est pas configuré.");

  // Deux contrôles, et il en faut deux : celui-ci donne le bon message d'erreur à un vrai
  // client (qui annonce toujours sa taille), et `body(…, MAX_BODY)` borne la lecture même
  // quand rien n'est annoncé — un envoi en `chunked` traversait l'ancien contrôle seul.
  if (Number(request.headers.get("content-length") || 0) > MAX_BODY) {
    return fail(413, "trop_gros", "Cette bibliothèque dépasse la taille acceptée.");
  }

  const data = (await body(request, MAX_BODY)) || {};
  const deviceId = clamp(data.deviceId, 40);
  if (!ID.test(deviceId)) {
    return fail(400, "appareil_invalide", "Identifiant d'appareil invalide.");
  }
  if (!Array.isArray(data.games)) {
    return fail(400, "requete_invalide", "Il manque la liste des jeux.");
  }

  // Dédoublonnage par clé : deux launchers pour le même jeu ne font qu'une entrée, et
  // c'est la première vue qui gagne (le client range ses sources fusionnées en tête).
  const parCle = new Map();
  for (const brut of data.games) {
    if (parCle.size >= MAX_GAMES) break;
    const jeu = normalizeGame(brut);
    if (jeu && !parCle.has(jeu.key)) parCle.set(jeu.key, jeu);
  }
  const games = [...parCle.values()];

  const deviceName = clamp(data.deviceName, 60) || "Cet appareil";
  const digest = clamp(data.digest, 64).replace(/[^A-Za-z0-9_-]/g, "");

  // Un appareil déjà connu se réécrit ; un nouveau ne doit pas pouvoir s'ajouter sans fin.
  const connus = await env.DB.prepare("SELECT device_id FROM libraries WHERE account_id = ?")
    .bind(session.accountId)
    .all();
  const lignes = connus.results || [];
  if (!lignes.some((r) => r.device_id === deviceId) && lignes.length >= MAX_DEVICES) {
    return fail(
      409,
      "trop_d_appareils",
      "Trop d'appareils synchronisés. Retires-en un avant d'en ajouter un autre.",
    );
  }

  const snapshot = JSON.stringify({
    version: SNAPSHOT_VERSION,
    deviceId,
    deviceName,
    updatedAt: now(),
    games,
  });
  const octets = new TextEncoder().encode(snapshot);

  await libs.put(objectKey(session.accountId, deviceId), octets, {
    httpMetadata: { contentType: "application/json; charset=utf-8" },
  });

  // 🔑 L'index est écrit APRÈS l'objet : une ligne qui annonce une bibliothèque absente
  // ferait échouer la lecture d'un ami, alors qu'un objet sans ligne d'index n'est qu'un
  // octet perdu, réécrit à la synchronisation suivante.
  await env.DB.prepare(
    `INSERT INTO libraries (account_id, device_id, device_name, digest, game_count, size_bytes, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(account_id, device_id) DO UPDATE SET
       device_name = excluded.device_name, digest = excluded.digest,
       game_count = excluded.game_count, size_bytes = excluded.size_bytes,
       updated_at = excluded.updated_at`,
  )
    .bind(session.accountId, deviceId, deviceName, digest, games.length, octets.length, now())
    .run();

  return json({ ok: true, deviceId, digest, gameCount: games.length, sizeBytes: octets.length });
}

/**
 * `GET /v1/library` — l'index : mes appareils, et ceux des amis qui partagent.
 *
 * Une seule requête SQL, aucune lecture R2. C'est l'appel fréquent : il dit quoi
 * retélécharger (l'empreinte a changé) et quoi laisser tranquille.
 */
export async function libraryIndex(request, env, session) {
  const rows = await env.DB.prepare(
    `SELECT l.account_id, l.device_id, l.device_name, l.digest, l.game_count,
            l.size_bytes, l.updated_at, a.display_name
       FROM libraries l
       JOIN accounts a ON a.id = l.account_id
      WHERE l.account_id = ?
         OR (a.share_library = 1
             AND EXISTS (SELECT 1 FROM friendships f
                          WHERE f.state = 'accepted'
                            AND ((f.requester_id = ? AND f.addressee_id = l.account_id)
                              OR (f.addressee_id = ? AND f.requester_id = l.account_id))))
      ORDER BY l.updated_at DESC`,
  )
    .bind(session.accountId, session.accountId, session.accountId)
    .all();

  const vue = (r) => ({
    accountId: r.account_id,
    displayName: r.display_name,
    deviceId: r.device_id,
    deviceName: r.device_name,
    digest: r.digest || null,
    gameCount: r.game_count,
    sizeBytes: r.size_bytes,
    updatedAt: r.updated_at,
  });

  const toutes = (rows.results || []).map(vue);
  return json({
    mine: toutes.filter((l) => l.accountId === session.accountId),
    friends: toutes.filter((l) => l.accountId !== session.accountId),
  });
}

/**
 * `GET /v1/library/{accountId}/{deviceId}` — la bibliothèque d'un appareil.
 *
 * 🔑 Deux conditions pour lire celle de quelqu'un d'autre : être **amis acceptés**, et que
 * la personne ait **activé le partage**. Sa propre bibliothèque se lit toujours, partage
 * éteint ou non — c'est la voie qu'empruntera le mobile.
 */
export async function readLibrary(request, env, session, accountId, deviceId) {
  const libs = bucket(env);
  if (!libs) return fail(500, "mal_configure", "Le stockage des bibliothèques n'est pas configuré.");
  if (!ID.test(accountId) || !ID.test(deviceId)) {
    return fail(404, "introuvable", "Cette bibliothèque n'existe pas.");
  }

  if (accountId !== session.accountId) {
    const acces = await env.DB.prepare(
      `SELECT a.share_library
         FROM accounts a
         JOIN friendships f
           ON f.state = 'accepted'
          AND ((f.requester_id = ? AND f.addressee_id = a.id)
            OR (f.addressee_id = ? AND f.requester_id = a.id))
        WHERE a.id = ?`,
    )
      .bind(session.accountId, session.accountId, accountId)
      .first();
    // Pas ami : on ne confirme même pas que le compte existe.
    if (!acces) return fail(404, "introuvable", "Cette bibliothèque n'existe pas.");
    if (!acces.share_library) {
      return fail(403, "partage_desactive", "Cette personne ne partage pas sa bibliothèque.");
    }
  }

  const index = await env.DB.prepare(
    "SELECT digest, updated_at FROM libraries WHERE account_id = ? AND device_id = ?",
  )
    .bind(accountId, deviceId)
    .first();
  if (!index) return fail(404, "introuvable", "Cette bibliothèque n'existe pas.");

  // L'empreinte sert d'ETag : un client à jour repart avec un 304, sans lecture R2 ni
  // transfert. C'est ce qui rend la synchronisation mobile presque gratuite quand rien
  // n'a bougé — et une opération de classe B économisée à chaque fois.
  const etag = index.digest ? `"${index.digest}"` : null;
  if (etag && request.headers.get("if-none-match") === etag) {
    return new Response(null, { status: 304, headers: { etag } });
  }

  const objet = await libs.get(objectKey(accountId, deviceId));
  // Index sans objet : la ligne ment (objet supprimé à la main, écriture interrompue). On
  // le dit introuvable plutôt que de renvoyer un corps vide que le client afficherait.
  if (!objet) return fail(404, "introuvable", "Cette bibliothèque n'est plus disponible.");

  const headers = {
    "content-type": "application/json; charset=utf-8",
    "access-control-allow-origin": "*",
    "cache-control": "private, no-cache",
  };
  if (etag) headers.etag = etag;
  return new Response(objet.body, { headers });
}

/**
 * `DELETE /v1/library/{deviceId}` — oublie un appareil.
 *
 * 🔑 L'objet R2 part AVANT sa ligne d'index : l'index est la seule carte qui mène à
 * l'objet. Dans l'autre ordre, une panne au milieu laisserait dans R2 une bibliothèque
 * que plus rien ne désigne — donc que plus rien ne peut effacer.
 */
export async function forgetLibrary(request, env, session, deviceId) {
  const libs = bucket(env);
  if (!libs) return fail(500, "mal_configure", "Le stockage des bibliothèques n'est pas configuré.");
  if (!ID.test(deviceId)) return json({ ok: true });

  await libs.delete(objectKey(session.accountId, deviceId));
  await env.DB.prepare("DELETE FROM libraries WHERE account_id = ? AND device_id = ?")
    .bind(session.accountId, deviceId)
    .run();
  return json({ ok: true });
}

/** `DELETE /v1/library` — cesser de synchroniser : tout part, tout de suite. */
export async function forgetAll(request, env, session) {
  await forgetAllLibraries(env, session.accountId);
  return json({ ok: true });
}

/**
 * Efface toutes les bibliothèques d'un compte (arrêt de la synchronisation, suppression du
 * compte). Exportée pour que `deleteMe` s'en serve : un compte supprimé qui laisse ses
 * bibliothèques dans R2, c'est une promesse rompue.
 *
 * ⚠️ Volontairement **sans filet** : si R2 refuse, l'exception remonte et l'appel échoue
 * AVANT que les lignes d'index disparaissent. Mieux vaut une suppression à réessayer
 * qu'une suppression qui se déclare faite en laissant les objets derrière elle.
 */
export async function forgetAllLibraries(env, accountId) {
  const rows = await env.DB.prepare("SELECT device_id FROM libraries WHERE account_id = ?")
    .bind(accountId)
    .all();
  const cles = (rows.results || []).map((r) => objectKey(accountId, r.device_id));

  const libs = bucket(env);
  if (libs && cles.length) await libs.delete(cles);
  await env.DB.prepare("DELETE FROM libraries WHERE account_id = ?").bind(accountId).run();
}
