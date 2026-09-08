/**
 * Torii — le panneau d'administration : combien de monde, et depuis quand.
 *
 * 🔑 POURQUOI CE FICHIER EXISTE. La décision « quand passer au plan payant » se prend sur
 * une PENTE, pas sur une photo. Or rien n'enregistrait l'histoire du service : la présence
 * expire, les comptes ne font que grossir, et une requête SQL ne donne que l'instant
 * présent. Un chiffre qu'on ne relève pas ne se rattrape jamais — d'où le relevé, qui
 * commence à tourner bien avant qu'on en ait besoin.
 *
 * ⚠️ AUCUNE DONNÉE PERSONNELLE NE PASSE PAR ICI. Que des décomptes : jamais une adresse,
 * jamais un pseudo, jamais un jeu. C'est délibéré — un panneau d'administration est une
 * porte de plus, et une porte ne doit pas donner sur plus que ce qu'elle sert.
 *
 * Secret : ADMIN_TOKEN (`npx wrangler secret put ADMIN_TOKEN`). Absent, la route est
 * fermée — jamais ouverte : une erreur de configuration ne doit pas publier les chiffres.
 */

import { fail, hash, json, now, sameHash } from "./lib.js";

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

  return json({
    maintenant: { ...vif, instant },
    plafond: PLAFOND,
    jours: (results || []).reverse(),
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

const PAGE = `<!doctype html>
<html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>Torii — suivi du service</title>
<style>
  :root {
    --bg:#131019; --surface:#1b1823; --bord:rgba(255,255,255,.1);
    --texte:#ece9f3; --faible:#a39daf; --accent:#ff6b57;
    font-family:"Segoe UI",system-ui,-apple-system,sans-serif;
  }
  * { box-sizing:border-box }
  body { margin:0; background:var(--bg); color:var(--texte); line-height:1.55;
         padding:28px 20px 60px }
  main { max-width:920px; margin:0 auto }
  h1 { font-size:21px; margin:0 0 4px; letter-spacing:-.02em }
  .sous { margin:0 0 26px; color:var(--faible); font-size:13.5px }
  .chiffres { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr));
              gap:12px; margin-bottom:26px }
  .tuile { background:var(--surface); border:1px solid var(--bord); border-radius:14px;
           padding:16px 18px }
  .tuile b { display:block; font-size:27px; font-weight:800; letter-spacing:-.03em;
             color:var(--accent) }
  .tuile span { font-size:12.5px; color:var(--faible) }
  .carte { background:var(--surface); border:1px solid var(--bord); border-radius:14px;
           padding:18px; margin-bottom:16px }
  .carte h2 { font-size:14px; margin:0 0 2px; font-weight:700 }
  .carte p { margin:0 0 14px; font-size:12.5px; color:var(--faible) }
  svg { display:block; width:100%; height:auto; overflow:visible }
  .axe { stroke:var(--bord); stroke-width:1 }
  .trace { fill:none; stroke:var(--accent); stroke-width:2;
           stroke-linecap:round; stroke-linejoin:round }
  .aire { fill:var(--accent); opacity:.13 }
  .repere { stroke:var(--faible); stroke-width:1; stroke-dasharray:4 4; opacity:.6 }
  text { fill:var(--faible); font-size:10px }
  form { display:flex; gap:10px; flex-wrap:wrap; align-items:center }
  input { flex:1; min-width:220px; padding:11px 13px; border-radius:10px;
          border:1px solid var(--bord); background:#0f0d14; color:var(--texte);
          font-size:14px; font-family:inherit }
  button { padding:11px 20px; border:0; border-radius:10px; background:var(--accent);
           color:#1a0f0c; font-weight:700; font-size:14px; cursor:pointer;
           font-family:inherit }
  .lien { background:none; color:var(--faible); padding:6px 0; font-weight:400;
          text-decoration:underline; font-size:12.5px }
  .erreur { color:var(--accent); font-size:13px; margin:12px 0 0 }
  [hidden] { display:none !important }
</style>
</head><body><main>
  <h1>Torii — suivi du service</h1>
  <p class="sous" id="sous">Comptes, usage réel et pic de présence, jour par jour.</p>

  <section id="porte" class="carte" hidden>
    <h2>Jeton d'administration</h2>
    <p>Il reste dans ce navigateur et ne part que dans l'en-tête <code>Authorization</code>.</p>
    <form id="form">
      <input id="jeton" type="password" placeholder="ADMIN_TOKEN" autocomplete="off">
      <button type="submit">Ouvrir</button>
    </form>
    <p class="erreur" id="erreur" hidden></p>
  </section>

  <div id="tableau" hidden>
    <div class="chiffres" id="chiffres"></div>
    <div class="carte">
      <h2>Comptes créés</h2>
      <p>Le cumul. C'est la courbe qui monte tant que le bouche-à-oreille fonctionne.</p>
      <div id="g-comptes"></div>
    </div>
    <div class="carte">
      <h2>Pic de présence simultanée</h2>
      <p id="p-pic">Le maximum atteint chaque jour.</p>
      <div id="g-pic"></div>
    </div>
    <div class="carte">
      <h2>Comptes actifs sur 7 jours</h2>
      <p>Combien s'en servent encore — un compte créé puis abandonné compte dans le cumul,
         pas ici.</p>
      <div id="g-actifs"></div>
    </div>
    <button class="lien" id="oublier">Oublier le jeton sur cet appareil</button>
  </div>

<script>
const CLE = "torii-admin";
const $ = (id) => document.getElementById(id);

/**
 * Trace une courbe. Pas de bibliothèque : la page est servie par le Worker, et faire
 * dépendre un tableau de bord d'un CDN, c'est le casser le jour où le CDN tombe.
 */
function courbe(hote, valeurs, plafond) {
  if (valeurs.length === 0) { hote.innerHTML = "<p style='color:var(--faible);font-size:13px;margin:0'>Pas encore de relevé — le premier arrive à l'heure ronde.</p>"; return; }
  const L = 700, H = 180, m = 26;
  // Un seul point ne fait pas une ligne : on le double pour qu'il se voie quand meme.
  const pts = valeurs.length === 1 ? [valeurs[0], valeurs[0]] : valeurs;
  const sommet = Math.max(...pts.map((p) => p.v));

  /**
   * 🔑 L'ÉCHELLE SUIT LES DONNÉES, PAS LE PLAFOND. Caler l'axe sur 160 alors que le pic
   * vaut 17 écrase la courbe sur l'axe : on voit qu'on est loin de la limite, et plus
   * rien de la tendance — or c'est la tendance qu'on vient chercher. Le repère du plafond
   * n'apparaît donc qu'une fois qu'il entre dans le champ (au quart), c'est-à-dire au
   * moment où il commence à vouloir dire quelque chose. Avant, il est dit en toutes
   * lettres sous le titre.
   */
  const proche = plafond && sommet >= plafond / 4;
  const haut = Math.max(1, sommet, proche ? plafond * 1.05 : 0);
  const x = (i) => m + (i * (L - m * 2)) / (pts.length - 1);
  const y = (v) => H - m - (v / haut) * (H - m * 2);
  const d = pts.map((p, i) => \`\${i ? "L" : "M"}\${x(i).toFixed(1)} \${y(p.v).toFixed(1)}\`).join(" ");
  const aire = \`\${d} L\${x(pts.length - 1).toFixed(1)} \${H - m} L\${x(0).toFixed(1)} \${H - m} Z\`;
  const ligne = proche
    ? \`<path class="repere" d="M\${m} \${y(plafond).toFixed(1)} H\${L - m}"/>
       <text x="\${L - m}" y="\${(y(plafond) - 5).toFixed(1)}" text-anchor="end">plafond estimé \${plafond}</text>\`
    : "";
  hote.innerHTML = \`<svg viewBox="0 0 \${L} \${H}" role="img">
    <path class="aire" d="\${aire}"/><path class="trace" d="\${d}"/>\${ligne}
    <path class="axe" d="M\${m} \${H - m} H\${L - m}"/>
    <text x="\${m}" y="\${H - m + 15}">\${valeurs[0].j}</text>
    <text x="\${L - m}" y="\${H - m + 15}" text-anchor="end">\${valeurs[valeurs.length - 1].j}</text>
    <text x="\${m}" y="\${m - 8}">\${Math.round(haut)}</text>
  </svg>\`;
}

function tuile(valeur, libelle) {
  return \`<div class="tuile"><b>\${valeur}</b><span>\${libelle}</span></div>\`;
}

async function charger(jeton) {
  const r = await fetch("/v1/admin/stats", { headers: { authorization: "Bearer " + jeton } });
  if (!r.ok) throw new Error(r.status === 404 ? "Jeton refusé." : "Le serveur a répondu " + r.status + ".");
  return await r.json();
}

function afficher(d) {
  const n = d.maintenant;
  $("chiffres").innerHTML =
    tuile(n.comptes, "comptes Torii") +
    tuile(n.actifs_7j, "actifs sur 7 jours") +
    tuile(n.en_ligne, "en ligne à l'instant") +
    tuile(n.biblios, "bibliothèques synchronisées");
  const record = Math.max(0, ...d.jours.map((j) => j.pic_en_ligne));
  $("p-pic").textContent =
    "Le maximum atteint chaque jour — c'est ce chiffre qui se compare au plafond, pas le "
    + "nombre de comptes. Record à ce jour : " + record + " sur un plafond estimé à "
    + d.plafond + ".";
  courbe($("g-comptes"), d.jours.map((j) => ({ j: j.jour, v: j.comptes })), 0);
  courbe($("g-pic"), d.jours.map((j) => ({ j: j.jour, v: j.pic_en_ligne })), d.plafond);
  courbe($("g-actifs"), d.jours.map((j) => ({ j: j.jour, v: j.actifs_7j })), 0);
  $("sous").textContent = "Relevé à l'instant · " + d.jours.length + " jour(s) d'historique";
  $("porte").hidden = true;
  $("tableau").hidden = false;
}

async function essayer(jeton, memoriser) {
  try {
    const d = await charger(jeton);
    if (memoriser) { try { localStorage.setItem(CLE, jeton); } catch {} }
    afficher(d);
    return true;
  } catch (e) {
    $("erreur").textContent = e.message;
    $("erreur").hidden = false;
    $("porte").hidden = false;
    return false;
  }
}

$("form").addEventListener("submit", (e) => {
  e.preventDefault();
  $("erreur").hidden = true;
  essayer($("jeton").value.trim(), true);
});

$("oublier").addEventListener("click", () => {
  try { localStorage.removeItem(CLE); } catch {}
  location.reload();
});

// Un jeton déjà connu ouvre la page directement ; sinon on demande.
let garde = null;
try { garde = localStorage.getItem(CLE); } catch {}
if (garde) essayer(garde, false); else $("porte").hidden = false;
</script>
</main></body></html>`;
