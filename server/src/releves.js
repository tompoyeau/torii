/**
 * Torii — les relevés quotidiens du service, relayés chaque nuit vers Umami (cf. umami.js).
 *
 * 🔑 POURQUOI CE FICHIER EXISTE. La décision « quand passer au plan payant » se prend sur
 * une PENTE, pas sur une photo. Or rien n'enregistrait l'histoire du service : la présence
 * expire, les comptes ne font que grossir, et une requête SQL ne donne que l'instant
 * présent. Un chiffre qu'on ne relève pas ne se rattrape jamais — d'où le relevé.
 *
 * Les chiffres se lisent dans Umami (stats.topo-host.com, fiche « Torii · comptes et
 * téléchargements ») : l'ancien panneau `/admin` a été retiré le 8 octobre 2026 pour que
 * toutes les statistiques des projets soient au même endroit. Les tables `stats` et
 * `github_releve` restent la source du relais.
 */

import { now } from "./lib.js";

const DEPOT = "tompoyeau/torii";

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
 * Les versions publiées et leurs téléchargements, tels que GitHub les compte.
 *
 * ⚠️ QUOTA. Sans jeton, l'API GitHub accorde 60 requêtes par heure et par IP — et les IP
 * de sortie des Workers sont partagées avec d'autres clients. D'où le cache d'un quart
 * d'heure, et le secret facultatif `GITHUB_TOKEN` (lecture seule, aucun droit requis sur
 * un dépôt public) qui porte la limite à 5 000. En cas d'échec : `null`.
 *
 * ⚠️ GitHub ne sépare pas installations et mises à jour : l'updater télécharge le même
 * installeur. `latest.json` compte les vérifications de mise à jour (≈ lancements).
 */
async function lireGithub(env) {
  const url = `https://api.github.com/repos/${DEPOT}/releases?per_page=100`;
  const cache = caches.default;
  const cle = new Request(url);
  let reponse = await cache.match(cle);
  if (!reponse) {
    const headers = { "user-agent": "torii-api", accept: "application/vnd.github+json" };
    if (env.GITHUB_TOKEN) headers.authorization = `Bearer ${env.GITHUB_TOKEN}`;
    const brute = await fetch(url, { headers });
    if (!brute.ok) return null;
    reponse = new Response(await brute.text(), {
      headers: { "content-type": "application/json", "cache-control": "max-age=900" },
    });
    await cache.put(cle, reponse.clone());
  }
  const releases = await reponse.json();
  return releases
    .filter((r) => !r.draft)
    .map((r) => {
      const compte = (test) =>
        r.assets.filter((a) => test(a.name.toLowerCase())).reduce((n, a) => n + a.download_count, 0);
      return {
        exe: compte((n) => n.endsWith(".exe")),
        msi: compte((n) => n.endsWith(".msi")),
        controles: compte((n) => n === "latest.json"),
      };
    });
}

/** Relevé quotidien des cumuls GitHub (appelé par le cron, comme `releverStats`). */
export async function releverGithub(env) {
  const versions = await lireGithub(env);
  if (!versions) return;
  const jour = new Date(now() * 1000).toISOString().slice(0, 10);
  const installeurs = versions.reduce((n, v) => n + v.exe + v.msi, 0);
  const controles = versions.reduce((n, v) => n + v.controles, 0);
  await env.DB.prepare(
    `INSERT INTO github_releve (jour, installeurs, controles) VALUES (?1, ?2, ?3)
     ON CONFLICT(jour) DO UPDATE SET installeurs = ?2, controles = ?3`,
  )
    .bind(jour, installeurs, controles)
    .run();
}
