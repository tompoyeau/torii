/**
 * Génère les notes de version du site, en français et en anglais :
 *   - `site/notes-de-version/index.html`  ← `CHANGELOG.md`
 *   - `site/en/release-notes/index.html`  ← `CHANGELOG.en.md`
 *
 *     npm run build:notes
 *
 * 🔑 POURQUOI UNE GÉNÉRATION, ET PAS UN `fetch` AU CHARGEMENT. Le bouton de
 * téléchargement, lui, interroge l'API GitHub dans le navigateur — c'est le bon choix
 * pour une donnée qui change à chaque release et tient en une ligne. Ici c'est
 * l'inverse : ces pages n'existent QUE pour leur contenu. Un texte injecté par
 * JavaScript dépend du bon vouloir du robot qui passe — Google exécute le script,
 * beaucoup d'autres non, et un aperçu de lien partagé n'en voit jamais rien. Trente-cinq
 * versions de prose, c'est précisément ce qu'on veut voir indexé : ça va donc dans le
 * HTML, en dur.
 *
 * ⚠️ À RELANCER AVANT CHAQUE ENVOI FTP, après avoir ajouté la section aux DEUX
 * changelogs. Les pages ne se mettent pas à jour toutes seules — c'est le prix du statique.
 *
 * 🔑 LE FRANÇAIS FAIT FOI. Une version présente dans `CHANGELOG.md` mais oubliée dans
 * `CHANGELOG.en.md` n'est PAS retirée de la page anglaise : elle y figure avec son texte
 * français, balisé `lang="fr"` et signalé « Not yet translated ». Masquer une release
 * aux anglophones serait pire qu'une note en français — et le script le crie en console.
 * Une version présente seulement en anglais est une faute de frappe : ignorée, et signalée.
 *
 * Les dates viennent de l'API GitHub (un seul appel). Si le réseau manque, les pages se
 * génèrent quand même, sans les dates : mieux vaut une page à jour sans date qu'un échec
 * de génération qui laisse en ligne la version précédente.
 */
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ICI = dirname(fileURLToPath(import.meta.url));
const RACINE = join(ICI, "..");
const DEPOT = "tompoyeau/torii";
const SITE = "https://torii-app.fr";

/**
 * Tout ce qui diffère d'une langue à l'autre.
 *
 * `r` est le chemin relatif vers la racine du site depuis la page : la page française
 * est à un niveau (`../`), l'anglaise à deux (`../../`). `soi` est le chemin relatif
 * de la page vers elle-même, utilisé par l'entrée active de la navigation.
 */
const LANGUES = {
  fr: {
    source: "CHANGELOG.md",
    destination: "site/notes-de-version/index.html",
    chemin: "/notes-de-version/",
    r: "../",
    locale: "fr-FR",
    ogLocale: "fr_FR",
    ogAlternate: "en_US",
    titre: "Notes de version de Torii — toutes les nouveautés, version par version",
    ogTitre: "Notes de version de Torii",
    ogDescription: "Ce qui a changé dans Torii, version par version.",
    description: (derniere, date) =>
      `Toutes les nouveautés et corrections de Torii, version par version. ` +
      `Dernière en date : ${derniere}${date ? `, publiée le ${date}` : ""}. ` +
      `Bibliothèque de jeux unifiée pour Windows, gratuite, en français et en anglais.`,
    capture: "captures/bibliotheque.jpg",
    fil: ["Accueil", "Notes de version"],
    accueil: "",
    version: (v) => `Version ${v}`,
    derniere: "Dernière version",
    nonTraduit: "",
    nav: { steam: ["Avec Steam", "steam/"], notes: "Notes de version", code: "Le code" },
    autre: { libelle: "English", lang: "en" },
    surTitre: "Journal des modifications",
    h1: "Ce qui change, version après version",
    chapeau: `Torii est écrit par une seule personne, et chaque version est publiée avec la
          liste de ce qu'elle corrige et de ce qu'elle ajoute. Rien n'est résumé en
          « améliorations diverses » : si quelque chose a changé sous tes yeux, c'est
          écrit ici.`,
    rappelTitre: "Toujours pas installé&nbsp;?",
    rappelTexte: "Gratuit, sans publicité, sans compte. Windows 10 et 11.",
    telecharger: "Télécharger Torii",
    pied: "Torii — bibliothèque de jeux pour Windows",
    piedAccueil: "Accueil",
    signaler: "Signaler un problème",
  },
  en: {
    source: "CHANGELOG.en.md",
    destination: "site/en/release-notes/index.html",
    chemin: "/en/release-notes/",
    r: "../../",
    locale: "en-US",
    ogLocale: "en_US",
    ogAlternate: "fr_FR",
    titre: "Torii release notes — every change, version by version",
    ogTitre: "Torii release notes",
    ogDescription: "What changed in Torii, version by version.",
    description: (derniere, date) =>
      `Every new feature and fix in Torii, version by version. ` +
      `Latest: ${derniere}${date ? `, released ${date}` : ""}. ` +
      `A unified game library for Windows, free, in English and French.`,
    capture: "captures/library-en.jpg",
    fil: ["Home", "Release notes"],
    accueil: "en/",
    version: (v) => `Version ${v}`,
    derniere: "Latest version",
    nonTraduit: "Not yet translated",
    nav: { steam: ["With Steam", "en/steam/"], notes: "Release notes", code: "Source code" },
    autre: { libelle: "Français", lang: "fr" },
    surTitre: "Changelog",
    h1: "What changes, version after version",
    chapeau: `Torii is written by a single person, and every version ships with the list
          of what it fixes and what it adds. Nothing gets summed up as “various
          improvements”: if something changed in front of you, it's written here.`,
    rappelTitre: "Still not installed?",
    rappelTexte: "Free, no ads, no account needed. Windows 10 and 11.",
    telecharger: "Download Torii",
    pied: "Torii — game library for Windows",
    piedAccueil: "Home",
    signaler: "Report an issue",
  },
};

/* ---------------------------------------------------------------- lecture */

/**
 * Découpe un changelog en versions.
 *
 * Le format est strict et déjà imposé par la CI, qui extrait la section `## X.Y.Z`
 * correspondant au tag pour la bannière de mise à jour. On s'appuie dessus : tout ce
 * qui précède la première section (le préambule expliquant ce format) est ignoré.
 */
function lireLesVersions(markdown) {
  const versions = [];
  let courante = null;

  for (const ligne of markdown.split(/\r?\n/)) {
    const titre = ligne.match(/^##\s+(\d+\.\d+\.\d+)\s*$/);
    if (titre) {
      courante = { version: titre[1], points: [] };
      versions.push(courante);
      continue;
    }
    if (!courante) continue; // préambule

    const puce = ligne.match(/^[-*]\s+(.*)$/);
    if (puce) {
      courante.points.push(puce[1].trim());
    } else if (ligne.trim() && courante.points.length) {
      // Un point écrit sur plusieurs lignes : on le recolle.
      courante.points[courante.points.length - 1] += " " + ligne.trim();
    }
  }
  return versions;
}

/** Les dates de publication, par version. Un seul appel : 35 releases tiennent large. */
async function lireLesDates() {
  try {
    const reponse = await fetch(
      `https://api.github.com/repos/${DEPOT}/releases?per_page=100`,
      { headers: { accept: "application/vnd.github+json" } },
    );
    if (!reponse.ok) throw new Error(`HTTP ${reponse.status}`);
    const dates = new Map();
    for (const release of await reponse.json()) {
      const version = (release.tag_name || "").replace(/^v/, "");
      if (version && release.published_at) dates.set(version, release.published_at);
    }
    return dates;
  } catch (erreur) {
    console.warn(`⚠️  Dates indisponibles (${erreur.message}) — pages générées sans.`);
    return new Map();
  }
}

/* ----------------------------------------------------------------- rendu */

const echapper = (texte) =>
  texte
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

/**
 * Le peu de Markdown en ligne que les changelogs utilisent réellement.
 *
 * ⚠️ On échappe AVANT d'ajouter les balises, jamais après : l'inverse réécrirait les
 * `<strong>` qu'on vient de poser en `&lt;strong&gt;`.
 * ⚠️ Le gras (`**`) AVANT l'italique (`*`) : dans l'autre ordre, chaque `**` serait lu
 * comme deux italiques vides. L'italique manquait jusqu'ici, et `*Synchroniser*` (0.17.0)
 * s'affichait avec ses astérisques.
 */
function enligne(texte) {
  return echapper(texte)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\*([^*\s][^*]*)\*/g, "<em>$1</em>");
}

/** « 6 septembre 2026 » / « September 6, 2026 ». */
function dateLisible(iso, locale) {
  return new Date(iso).toLocaleDateString(locale, {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function rendreUneVersion(L, { version, points, repli }, date, derniere) {
  const ancre = `v${version.replace(/\./g, "-")}`;
  // Une version non traduite garde son texte français : on le déclare, pour les lecteurs
  // d'écran comme pour les moteurs.
  const langueListe = repli ? ` lang="fr"` : "";
  const lignes = points.map((p) => `            <li>${enligne(p)}</li>`).join("\n");

  return `        <article class="note-version" id="${ancre}">
          <header>
            <h2><a href="#${ancre}">${L.version(version)}</a></h2>
${date ? `            <time datetime="${date.slice(0, 10)}">${dateLisible(date, L.locale)}</time>\n` : ""}${derniere ? `            <span class="etiquette">${L.derniere}</span>\n` : ""}${repli ? `            <span class="etiquette">${L.nonTraduit}</span>\n` : ""}          </header>
          <ul${langueListe}>
${lignes}
          </ul>
        </article>`;
}

function rendreLaPage(code, versions, dates) {
  const L = LANGUES[code];
  const autre = LANGUES[L.autre.lang];
  const r = L.r;
  const derniere = versions[0]?.version ?? "";
  const dateDerniere = dates.get(derniere);

  const articles = versions
    .map((v) => rendreUneVersion(L, v, dates.get(v.version), v.version === derniere))
    .join("\n");

  // La description reprend la dernière version : elle change donc à chaque release,
  // ce qui donne au moteur une raison de revenir lire la page.
  const description = L.description(derniere, dateDerniere && dateLisible(dateDerniere, L.locale));

  return `<!doctype html>
<html lang="${code}">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />

    <!--
      ⚠️ FICHIER GÉNÉRÉ — NE PAS MODIFIER À LA MAIN.
      Source : ${L.source} à la racine du dépôt. Régénérer avec \`npm run build:notes\`.

      Cette page existe pour deux raisons, dans cet ordre : donner à quelqu'un qui
      hésite la preuve que le produit est vivant, et donner aux moteurs une page qui
      change pour de bon à chaque version — un site d'une seule page n'a aucune raison
      d'être revisité.
    -->
    <title>${L.titre}</title>
    <meta name="description" content="${echapper(description)}" />
    <link rel="canonical" href="${SITE}${L.chemin}" />
    <!-- Réciproques sur les deux pages, comme partout ailleurs sur le site. -->
    <link rel="alternate" hreflang="fr" href="${SITE}${LANGUES.fr.chemin}" />
    <link rel="alternate" hreflang="en" href="${SITE}${LANGUES.en.chemin}" />
    <link rel="alternate" hreflang="x-default" href="${SITE}${LANGUES.en.chemin}" />
    <link rel="icon" type="image/png" href="${r}favicon.png" />
    <link rel="stylesheet" href="${r}styles.css" />

    <meta property="og:type" content="website" />
    <meta property="og:url" content="${SITE}${L.chemin}" />
    <meta property="og:title" content="${L.ogTitre}" />
    <meta property="og:description" content="${L.ogDescription}" />
    <meta property="og:image" content="${SITE}/${L.capture}" />
    <meta property="og:locale" content="${L.ogLocale}" />
    <meta property="og:locale:alternate" content="${L.ogAlternate}" />
    <meta name="twitter:card" content="summary_large_image" />

    <script type="application/ld+json">
      ${JSON.stringify({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        itemListElement: [
          { "@type": "ListItem", position: 1, name: L.fil[0], item: `${SITE}/${L.accueil}` },
          { "@type": "ListItem", position: 2, name: L.fil[1], item: `${SITE}${L.chemin}` },
        ],
      })}
    </script>
  </head>

  <body>
    <div class="barre-site">
      <a class="marque" href="${r}${L.accueil}">
        <img src="${r}torii.svg" alt="" width="30" height="30" />
        Torii
      </a>
      <nav>
        <a href="${r}${L.nav.steam[1]}">${L.nav.steam[0]}</a>
        <a href="${r}${L.chemin.slice(1)}" class="actif" aria-current="page">${L.nav.notes}</a>
        <a href="https://github.com/tompoyeau/torii">${L.nav.code}</a>
        <a href="${r}${autre.chemin.slice(1)}" hreflang="${L.autre.lang}" lang="${L.autre.lang}">${L.autre.libelle}</a>
      </nav>
    </div>

    <main class="page">
      <div class="page-entete">
        <p class="sur-titre">${L.surTitre}</p>
        <h1>${L.h1}</h1>
        <p class="chapeau">
          ${L.chapeau}
        </p>
      </div>

      <div class="notes">
${articles}
      </div>

      <section class="rappel">
        <h2>${L.rappelTitre}</h2>
        <p>${L.rappelTexte}</p>
        <a class="cta" href="https://github.com/tompoyeau/torii/releases/latest">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" aria-hidden="true">
            <path d="M12 3v12m0 0l-4.5-4.5M12 15l4.5-4.5" />
            <path d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2" />
          </svg>
          ${L.telecharger}
        </a>
      </section>
    </main>

    <footer>
      <img class="mark-mini" src="${r}torii.svg" alt="" width="28" height="28" />
      <p>${L.pied}</p>
      <nav>
        <a href="${r}${L.accueil}">${L.piedAccueil}</a>
        <a href="${r}${L.nav.steam[1]}">${L.nav.steam[0]}</a>
        <a href="https://github.com/tompoyeau/torii">${L.nav.code}</a>
        <a href="https://github.com/tompoyeau/torii/issues">${L.signaler}</a>
      </nav>
    </footer>
  </body>
</html>
`;
}

/* ------------------------------------------------------------------- main */

const francaises = lireLesVersions(await readFile(join(RACINE, LANGUES.fr.source), "utf8"));
if (!francaises.length) {
  console.error("✗ Aucune section `## X.Y.Z` trouvée dans CHANGELOG.md — rien généré.");
  process.exit(1);
}

let anglaises = [];
try {
  anglaises = lireLesVersions(await readFile(join(RACINE, LANGUES.en.source), "utf8"));
} catch {
  console.warn("⚠️  CHANGELOG.en.md introuvable — la page anglaise reprendra tout en français.");
}

// L'anglais suit l'ordre et la liste du français, qui fait foi (voir l'en-tête).
const parVersion = new Map(anglaises.map((v) => [v.version, v]));
const nonTraduites = [];
const versionsAnglaises = francaises.map((fr) => {
  const en = parVersion.get(fr.version);
  if (en) return en;
  nonTraduites.push(fr.version);
  return { ...fr, repli: true };
});
const orphelines = anglaises.filter((en) => !francaises.some((fr) => fr.version === en.version));

const dates = await lireLesDates();
const entrees = (liste) => liste.reduce((n, v) => n + v.points.length, 0);

for (const [code, versions] of [["fr", francaises], ["en", versionsAnglaises]]) {
  const destination = join(RACINE, LANGUES[code].destination);
  await mkdir(dirname(destination), { recursive: true });
  await writeFile(destination, rendreLaPage(code, versions, dates), "utf8");
  const sansDate = versions.filter((v) => !dates.has(v.version)).length;
  console.log(
    `✓ ${LANGUES[code].destination} — ${versions.length} versions, ${entrees(versions)} entrées` +
      (sansDate ? `, ${sansDate} sans date` : ""),
  );
}

if (nonTraduites.length) {
  console.warn(
    `⚠️  Non traduites dans CHANGELOG.en.md (affichées en français sur la page anglaise) : ${nonTraduites.join(", ")}`,
  );
}
if (orphelines.length) {
  console.warn(
    `⚠️  Présentes seulement dans CHANGELOG.en.md (ignorées — faute de frappe ?) : ${orphelines.map((v) => v.version).join(", ")}`,
  );
}
