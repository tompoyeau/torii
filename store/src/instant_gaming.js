/**
 * Prix Instant Gaming : pas d'API publique, et absent du comparateur. L'adresse `?ajax=true` de
 * sa recherche renvoie un fragment HTML qu'on lit au mieux.
 *
 * ⚠️ Fragile par nature : si le site change, on ne trouve simplement plus d'offre, et la fiche
 * reste fonctionnelle. Une panne ici n'est donc jamais une raison de ne pas mémoriser la fiche.
 * Un seul appel par fiche produit, jamais en masse. Mêmes règles que
 * `metadata/instant_gaming.rs` dans l'application installée.
 */

const RECHERCHE_FR = "https://www.instant-gaming.com/fr/rechercher/";
const RECHERCHE_EN = "https://www.instant-gaming.com/en/search/";
const NAVIGATEUR =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";
const DELAI_MS = 12_000;

/** Clé de rapprochement d'un titre (minuscules alphanumériques). */
const cle = (titre) => [...titre].filter((c) => /[\p{Alphabetic}\p{N}]/u.test(c)).join("").toLowerCase();

/** Encodage de la recherche : le `+` pour l'espace, comme le formulaire du site. */
const encoder = (s) =>
  encodeURIComponent(s)
    .replace(/[!'()*]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`)
    .replace(/%20/g, "+");

/** Le premier attribut `nom="…"` du bloc. */
function attr(bloc, nom) {
  const motif = `${nom}="`;
  const i = bloc.indexOf(motif);
  if (i < 0) return null;
  const debut = i + motif.length;
  const fin = bloc.indexOf('"', debut);
  return fin < 0 ? null : bloc.slice(debut, fin);
}

/** Décode le minimum d'entités rencontrées dans les prix et les titres. */
const decode = (s) =>
  s
    .replaceAll("&nbsp;", " ")
    .replaceAll("&amp;", "&")
    .replaceAll("&#039;", "'")
    .replaceAll("&#39;", "'")
    .replaceAll("&quot;", '"')
    .trim();

/** Un prix depuis un texte type `58.99&nbsp;€` ou `58,99 €`. */
export function lirePrix(brut) {
  const nettoye = [...decode(brut)].filter((c) => /[0-9.,]/.test(c)).join("").replaceAll(",", ".");
  if (!nettoye) return null;
  const n = Number(nettoye);
  return Number.isFinite(n) ? n : null;
}

/** Le texte du premier élément qui suit `classe` (`<div class="price">…</div>`). */
function contenu(bloc, classe) {
  const apres = bloc.split(`class="${classe}"`)[1];
  return apres === undefined ? null : (apres.split(">")[1]?.split("<")[0] ?? null);
}

/**
 * La meilleure offre PC d'un fragment de recherche, pour un titre normalisé `voulu`.
 * Le titre doit correspondre exactement : ça écarte les DLC et les éditions (« Elden Ring Shadow
 * of the Erdtree », « … Deluxe »).
 */
export function lireOffres(html, voulu) {
  let meilleure = null;
  for (const bloc of html.split("<article").slice(1)) {
    const apresTitre = bloc.split('class="title"')[1];
    const brut = apresTitre === undefined ? null : attr(apresTitre, "title");
    if (brut === null) continue;
    const titre = decode(brut);

    const lien = attr(bloc, "href");
    if (!lien?.includes("instant-gaming.com")) continue;

    // Les titres sont suffixés « <Nom> - <Plateforme> (<Boutique>) ». On isole le nom (avant le
    // dernier « - ») et on ne garde que les éditions PC.
    const tiret = titre.lastIndexOf(" - ");
    const nom = tiret < 0 ? titre : titre.slice(0, tiret).trim();
    const plateforme = tiret < 0 ? "" : titre.slice(tiret + 3).toLowerCase();
    if (plateforme && !plateforme.includes("pc")) continue;
    if (cle(nom) !== voulu) continue;

    const texte = contenu(bloc, "price");
    const prix = texte === null ? null : lirePrix(texte);
    if (prix === null) continue;
    const remise = [...(contenu(bloc, "discount") ?? "")].filter((c) => /[0-9]/.test(c)).join("");

    if (meilleure === null || prix < meilleure.price) {
      meilleure = { price: prix, savings: remise ? Number(remise) : 0, url: lien, available: true };
    }
  }
  return meilleure;
}

async function lirePage(url, headers) {
  const reponse = await fetch(url, { headers, signal: AbortSignal.timeout(DELAI_MS) });
  return reponse.ok ? reponse.text() : null;
}

async function cherche(base, titre, voulu) {
  try {
    const html = await lirePage(`${base}?q=${encoder(titre)}&ajax=true`, {
      "User-Agent": NAVIGATEUR,
      "X-Requested-With": "XMLHttpRequest",
    });
    return html === null ? null : lireOffres(html, voulu);
  } catch {
    return null;
  }
}

/**
 * La meilleure offre exacte pour un nom, sur le site de la langue. En français, repli sur le site
 * anglais quand le site a traduit le titre (« Shadow of the Erdtree » y devient « L'ombre de
 * l'Arbre-monde »).
 */
async function trouver(titre, francais) {
  const voulu = cle(titre);
  if ([...voulu].length < 3) return null;
  if (!francais) return cherche(RECHERCHE_EN, titre, voulu);
  return (await cherche(RECHERCHE_FR, titre, voulu)) ?? cherche(RECHERCHE_EN, titre, voulu);
}

/** En rupture, le site retire le bouton « add to cart ». Page illisible : on suppose disponible. */
async function disponible(url) {
  try {
    const html = await lirePage(url, { "User-Agent": NAVIGATEUR });
    return html === null ? true : html.includes("addtocart");
  } catch {
    return true;
  }
}

/**
 * L'offre d'un jeu, ou `null`.
 *
 * 🔑 DEUX NOMS PLUTÔT QU'UN : le comparateur ne nomme pas toujours le jeu comme le site (« Dying
 * Light: The Beast Restored Land » d'un côté, « Dying Light: The Beast » de l'autre). Faute de
 * résultat, on réessaie avec `nomSteam()`, le nom du jeu sur Steam. `nomSteam` n'est appelé qu'à
 * ce moment-là.
 */
export async function offreInstantGaming(ctx, titre, nomSteam) {
  const francais = ctx.langue === "fr";
  let offre = await trouver(titre, francais);
  if (offre === null) {
    const nom = await nomSteam();
    if (nom && cle(nom) !== cle(titre)) offre = await trouver(nom, francais);
  }
  if (offre === null) return null;
  // Le slug anglais placé sous `/fr/` est redirigé par le site vers son équivalent français.
  if (francais) offre.url = offre.url.replace("instant-gaming.com/en/", "instant-gaming.com/fr/");
  offre.available = await disponible(offre.url);
  return offre;
}
