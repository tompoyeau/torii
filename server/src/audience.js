/**
 * Torii — audience du site (torii-app.fr) et téléchargements GitHub.
 *
 * 🔑 COMPTEUR MAISON, SANS COOKIE. Le site est une page statique hébergée ailleurs ; un
 * petit script y envoie une balise à ce Worker à chaque page vue et à chaque clic sur
 * « Télécharger ». Rien n'est déposé chez le visiteur, l'IP n'est jamais écrite : seule
 * une empreinte qui change chaque jour (cf. migration 0004) permet de compter les
 * visiteurs distincts d'une journée. D'où l'absence de bandeau de consentement : il n'y
 * a rien à consentir.
 */

import { hash, now } from "./lib.js";

const DEPOT = "tompoyeau/torii";

/** Durée de conservation des visites. Au-delà, seule la tendance compte, et elle est vieille. */
const CONSERVATION = 400 * 86_400;

/** Robots d'indexation et navigateurs automatisés : ils gonfleraient les chiffres. */
const ROBOT = /bot|crawl|spider|slurp|headless|lighthouse|preview|facebookexternalhit|curl|wget|python/i;

/** Nos propres domaines : venir d'une page du site n'est pas une « source ». */
const NOUS = /(^|\.)torii-app\.fr$/;

/**
 * `POST /v1/hit` — enregistre une page vue ou un clic de téléchargement.
 *
 * Répond toujours 204, même quand la balise est ignorée : c'est un `sendBeacon`, personne
 * ne lit la réponse, et renvoyer une erreur n'apprendrait rien d'utile à un curieux.
 */
export async function enregistrerVisite(request, env) {
  const vide = new Response(null, { status: 204, headers: { "access-control-allow-origin": "*" } });
  const ua = request.headers.get("user-agent") || "";
  if (!ua || ROBOT.test(ua)) return vide;

  // Envoyé en `text/plain` (pas de pré-vol CORS pour une balise) : on parse à la main.
  let corps;
  try {
    const texte = await request.text();
    if (texte.length > 2048) return vide;
    corps = JSON.parse(texte);
  } catch {
    return vide;
  }

  const type = corps?.t === "telechargement" ? "telechargement" : "vue";
  const page = typeof corps?.p === "string" && corps.p.startsWith("/") ? corps.p.slice(0, 120) : null;
  if (!page) return vide;
  const langue = corps?.l === "en" ? "en" : "fr";
  let source = null;
  try {
    const hote = new URL(String(corps?.r || "")).hostname.replace(/^www\./, "");
    if (hote && !NOUS.test(hote)) source = hote.slice(0, 80);
  } catch {
    // Pas de référent, ou référent illisible : visite directe.
  }

  const instant = now();
  const jour = new Date(instant * 1000).toISOString().slice(0, 10);
  const ip = request.headers.get("cf-connecting-ip") || "local";
  const visiteur = (await hash(`${ip}|${ua}|${jour}`, env.PEPPER)).slice(0, 20);

  await env.DB.prepare(
    `INSERT INTO hits (at, jour, type, page, langue, source, pays, mobile, visiteur)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  )
    .bind(
      instant,
      jour,
      type,
      page,
      langue,
      source,
      request.cf?.country || null,
      /Mobi|Android|iPhone|iPad/i.test(ua) ? 1 : 0,
      visiteur,
    )
    .run();
  return vide;
}

/** Ménage : oublie les visites trop anciennes. */
export async function purgerVisites(env) {
  await env.DB.prepare("DELETE FROM hits WHERE at < ?").bind(now() - CONSERVATION).run();
}

/**
 * Les versions publiées et leurs téléchargements, tels que GitHub les compte.
 *
 * ⚠️ QUOTA. Sans jeton, l'API GitHub accorde 60 requêtes par heure et par IP — et les IP
 * de sortie des Workers sont partagées avec d'autres clients. D'où le cache d'un quart
 * d'heure, et le secret facultatif `GITHUB_TOKEN` (lecture seule, aucun droit requis sur
 * un dépôt public) qui porte la limite à 5 000. En cas d'échec : `null`, et la page le dit.
 */
export async function lireGithub(env) {
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
        version: r.tag_name,
        date: r.published_at,
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

/** Tout ce que le tableau de bord montre de l'audience, sur `jours` jours. */
export async function audience(env, jours) {
  const depuis = new Date((now() - jours * 86_400) * 1000).toISOString().slice(0, 10);
  const q = (sql, ...args) =>
    env.DB.prepare(sql)
      .bind(...args)
      .all()
      .then((r) => r.results || []);

  const [parJour, pages, sources, pays, appareils, releves] = await Promise.all([
    q(
      `SELECT jour,
              SUM(type = 'vue')                                      AS vues,
              COUNT(DISTINCT CASE WHEN type = 'vue' THEN visiteur END) AS visiteurs,
              SUM(type = 'telechargement')                           AS clics
         FROM hits WHERE jour >= ? GROUP BY jour ORDER BY jour`,
      depuis,
    ),
    q(
      `SELECT page AS nom, COUNT(*) AS n FROM hits
        WHERE jour >= ? AND type = 'vue' GROUP BY page ORDER BY n DESC LIMIT 10`,
      depuis,
    ),
    q(
      `SELECT COALESCE(source, '(accès direct)') AS nom, COUNT(DISTINCT visiteur || jour) AS n
         FROM hits WHERE jour >= ? AND type = 'vue' GROUP BY nom ORDER BY n DESC LIMIT 10`,
      depuis,
    ),
    q(
      `SELECT COALESCE(pays, '?') AS nom, COUNT(DISTINCT visiteur || jour) AS n
         FROM hits WHERE jour >= ? AND type = 'vue' GROUP BY nom ORDER BY n DESC LIMIT 10`,
      depuis,
    ),
    q(
      `SELECT CASE mobile WHEN 1 THEN 'Mobile' ELSE 'Ordinateur' END AS nom,
              COUNT(DISTINCT visiteur || jour) AS n
         FROM hits WHERE jour >= ? AND type = 'vue' GROUP BY mobile ORDER BY n DESC`,
      depuis,
    ),
    q(`SELECT jour, installeurs, controles FROM github_releve WHERE jour >= ? ORDER BY jour`, depuis),
  ]);

  return { parJour, pages, sources, pays, appareils, releves, versions: await lireGithub(env).catch(() => null) };
}

