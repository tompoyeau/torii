/**
 * Fiche produit : ce que Steam sait d'un jeu, au-delà du prix. Deux appels sans clé, hors du
 * quota partagé du comparateur : `GetItems` (captures, bandes-annonces, langues, tags, avis) et
 * `appdetails` (configuration, fonctionnalités, succès, classification, DLC, description).
 *
 * 🔑 AUCUN HTML NE PASSE À L'INTERFACE. Steam décrit ses jeux et leur configuration en HTML : on
 * le convertit ici en blocs typés, le texte est décodé et rendu comme du texte, les images ne
 * sont gardées que si elles viennent des serveurs de Steam. Mêmes règles que
 * `metadata/fiche.rs` dans l'application installée.
 */

import { chaine, entier, tableau } from "./lib.js";
import { EXCLUS, GET_ITEMS, contexteSteam, chargerTags, demos, steamItems } from "./steam.js";

const IMAGES = "https://shared.akamai.steamstatic.com/store_item_assets/";
const VIDEOS = "https://video.akamai.steamstatic.com/store_trailers/";

/** Au-delà, ce sont des variantes (versions sous-titrées, montages par classe…). */
const MAX_BANDES_ANNONCES = 8;
/** Au-delà, c'est la page Steam qu'il faut lire. */
const MAX_BLOCS = 120;
/** Tags jamais affichés ni utilisés pour les similaires : contenu, et généralités (solo, multijoueur…). */
const TAGS_ECARTES = new Set([6650, 12095, 4345, 4667, 113, 4182, 3859, 1756, 4166, 4191, 3871]);

/** Énumération `ELanguage` de Steam → étiquette BCP 47 (l'interface en tire le nom). */
const CODES_LANGUE = [
  "en", "de", "fr", "it", "ko", "es", "zh-Hans", "zh-Hant", "ru", "th", "ja", "pt", "pl", "da", "nl",
  "fi", "no", "sv", "hu", "cs", "ro", "tr", "pt-BR", "bg", "el", "ar", "uk", "es-419", "vi", "id",
];

// --- HTML -----------------------------------------------------------------------------

const ENTITES = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };

/** Le caractère d'un code numérique, `null` s'il n'en désigne aucun. */
function caractere(code) {
  if (!Number.isInteger(code) || code < 0 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) return null;
  return String.fromCodePoint(code);
}

/** Décode les entités HTML courantes (`&amp;`, `&#39;`, `&#x27;`, `&nbsp;`…). */
export function decoder(s) {
  if (!s.includes("&")) return s;
  let out = "";
  let i = 0;
  while (i < s.length) {
    const debut = s.indexOf("&", i);
    if (debut < 0) break;
    out += s.slice(i, debut);
    const fin = s.indexOf(";", debut);
    let car = null;
    if (fin >= 0 && fin - debut <= 10) {
      const entite = s.slice(debut + 1, fin);
      if (Object.hasOwn(ENTITES, entite)) car = ENTITES[entite];
      else if (/^#[xX][0-9a-fA-F]+$/.test(entite)) car = caractere(Number.parseInt(entite.slice(2), 16));
      else if (/^#\d+$/.test(entite)) car = caractere(Number.parseInt(entite.slice(1), 10));
    }
    if (car === null) {
      out += "&";
      i = debut + 1;
    } else {
      out += car;
      i = fin + 1;
    }
  }
  return out + s.slice(i);
}

/** Espaces repliés et retirés aux deux bouts (le HTML ne compte pas les espaces). */
const espaces = (s) => s.trim().replace(/\s+/g, " ");

/** Valeur d'un attribut entre guillemets (`src="…"` ou `src='…'`). */
function attribut(balise, nom) {
  const bas = balise.toLowerCase();
  let debut = 0;
  for (;;) {
    const i = bas.indexOf(nom, debut);
    if (i < 0) return null;
    const pos = i + nom.length;
    const suite = balise.slice(pos).trimStart();
    if (suite.startsWith("=")) {
      const valeur = suite.slice(1).trimStart();
      const guillemet = valeur[0];
      if (guillemet === undefined) return null;
      if (guillemet === '"' || guillemet === "'") {
        const fin = valeur.indexOf(guillemet, 1);
        return fin < 0 ? null : decoder(valeur.slice(1, fin));
      }
    }
    debut = pos;
  }
}

/**
 * Découpe du HTML en texte et balises. Volontairement minimal : on ne reconstruit pas d'arbre,
 * on ne garde que le nom de la balise, son sens et un éventuel `src`.
 */
function jetons(html) {
  const out = [];
  let i = 0;
  while (i < html.length) {
    const ouvrant = html.indexOf("<", i);
    if (ouvrant < 0) {
      out.push({ texte: decoder(html.slice(i)) });
      break;
    }
    if (ouvrant > i) {
      out.push({ texte: decoder(html.slice(i, ouvrant)) });
      i = ouvrant;
    }
    const fin = html.indexOf(">", i);
    if (fin < 0) {
      out.push({ texte: decoder(html.slice(i)) });
      break;
    }
    const dedans = html.slice(i + 1, fin);
    i = fin + 1;
    const corps = dedans.replace(/^\/+/, "");
    const nom = (corps.match(/^[A-Za-z0-9]+/)?.[0] ?? "").toLowerCase();
    if (!nom) continue; // commentaire, doctype…
    out.push({
      nom,
      fermante: dedans.startsWith("/"),
      src: nom === "img" || nom === "source" ? attribut(corps, "src") : null,
      poster: nom === "video" ? attribut(corps, "poster") : null,
    });
  }
  return out;
}

/** Une image n'est gardée que si elle vient des serveurs d'images de Steam, en https. */
export function imageSteam(url) {
  if (!url?.startsWith("https://")) return false;
  const hote = url.slice("https://".length).split("/")[0];
  return hote.endsWith(".steamstatic.com") || hote === "steamcdn-a.akamaihd.net";
}

/**
 * La description complète en blocs sûrs. Seuls les images et les vidéos des serveurs de Steam
 * sont gardées ; liens, styles et tout le reste disparaissent, leur texte restant.
 *
 * ⚠️ Beaucoup d'éditeurs ne mettent QUE des images et des vidéos en boucle dans cette description
 * (celle de Cyberpunk 2077 ne contient pas un mot : son texte est dans les images).
 */
export function blocs(html) {
  const sortie = [];
  let texte = "";
  let genre = "p";
  let video = null; // { affiche, mp4 } : la vidéo dont on lit les balises
  const vider = () => {
    const t = texte
      .split("\n")
      .map(espaces)
      .join("\n")
      .replace(/^\n+|\n+$/g, "");
    if (t) sortie.push({ genre, texte: t, url: null, affiche: null });
    texte = "";
  };
  for (const j of jetons(html)) {
    if (j.nom === undefined) {
      texte += j.texte;
    } else if (j.nom === "br") {
      texte += "\n";
    } else if (["p", "div", "ul", "ol"].includes(j.nom)) {
      vider();
      genre = "p";
    } else if (/^h[1-6]$/.test(j.nom)) {
      vider();
      genre = j.fermante ? "p" : "h";
    } else if (j.nom === "li") {
      vider();
      genre = j.fermante ? "p" : "li";
    } else if (j.nom === "img") {
      if (imageSteam(j.src)) {
        vider();
        sortie.push({ genre: "img", texte: "", url: j.src, affiche: null });
      }
    } else if (j.nom === "video" && !j.fermante) {
      vider();
      video = { affiche: imageSteam(j.poster) ? j.poster : null, mp4: null };
    } else if (j.nom === "source") {
      if (video && j.src && !video.mp4 && j.src.split("?")[0].endsWith(".mp4") && imageSteam(j.src)) {
        video.mp4 = j.src;
      }
    } else if (j.nom === "video") {
      if (video?.mp4) {
        sortie.push({ genre: "video", texte: "", url: video.mp4, affiche: video.affiche });
      } else if (video?.affiche) {
        // Pas de mp4 lisible : l'affiche seule vaut mieux que rien.
        sortie.push({ genre: "img", texte: "", url: video.affiche, affiche: null });
      }
      video = null;
    }
    if (sortie.length >= MAX_BLOCS) return sortie;
  }
  vider();
  return sortie;
}

/** Le texte seul d'un fragment HTML : balises retirées, entités décodées, espaces repliés. */
export function texteBrut(html) {
  let out = "";
  let dansBalise = false;
  for (const c of html) {
    if (c === "<") {
      dansBalise = true;
    } else if (c === ">" && dansBalise) {
      dansBalise = false;
      out += " ";
    } else if (!dansBalise) {
      out += c;
    }
  }
  return espaces(decoder(out));
}

/** Une ligne de texte par élément de liste (ou par saut de ligne, faute de liste). */
function lignesHtml(html) {
  const lignes = [];
  let courant = "";
  const couper = () => {
    const l = espaces(courant);
    if (l) lignes.push(l);
    courant = "";
  };
  for (const j of jetons(html)) {
    if (j.nom === undefined) courant += j.texte;
    else if (["li", "br", "ul", "p"].includes(j.nom)) couper();
  }
  couper();
  return lignes;
}

// --- Configuration, classification ------------------------------------------------------

/** « Notes supplémentaires » / « Additional Notes » : texte libre, pas une ligne de tableau. */
function estNote(libelle) {
  const l = libelle.toLowerCase();
  return l.startsWith("notes") || l.startsWith("additional");
}

/**
 * Les deux colonnes répètent souvent les mêmes notes, parfois l'une en version longue : une note
 * déjà contenue dans une autre n'est pas ajoutée.
 */
function ajouterNote(notes, note) {
  if (notes.some((n) => n.includes(note))) return;
  for (let i = notes.length - 1; i >= 0; i--) {
    if (note.includes(notes[i])) notes.splice(i, 1);
  }
  notes.push(note);
}

/** Configuration requise PC : listes « libellé : valeur » tirées du HTML de Steam. */
export function configuration(pc) {
  const c = { minimale: [], recommandee: [], notes: [] };
  for (const [cle, cible] of [["minimum", c.minimale], ["recommended", c.recommandee]]) {
    const html = chaine(pc?.[cle]);
    if (html === null) continue;
    for (const ligne of lignesHtml(html)) {
      const deuxPoints = ligne.indexOf(":");
      if (deuxPoints < 0) {
        ajouterNote(c.notes, ligne);
        continue;
      }
      const brut = ligne.slice(0, deuxPoints);
      const valeur = ligne.slice(deuxPoints + 1).trim();
      // « Minimale : » / « Recommandée : » : le titre de la colonne, qu'on a déjà.
      if (!valeur) continue;
      if ([...brut].length > 40) {
        ajouterNote(c.notes, ligne);
        continue;
      }
      // « Système d'exploitation * » : l'astérisque renvoie à une note absente de la réponse.
      const libelle = brut.trim().replace(/\*+$/, "").trim();
      if (estNote(libelle)) ajouterNote(c.notes, valeur);
      else cible.push({ libelle, valeur });
    }
  }
  return c.minimale.length || c.recommandee.length ? c : null;
}

/**
 * La classification d'âge de la région : PEGI en Europe, USK en Allemagne, ESRB en Amérique du
 * Nord… avec repli sur PEGI puis ESRB quand l'éditeur n'a pas donné celle du pays.
 */
export function classification(notes, region) {
  const prefere = { DE: "usk", US: "esrb", CA: "esrb", MX: "esrb", AU: "oflc", NZ: "oflc", BR: "dejus" }[region] ?? "pegi";
  for (const systeme of [prefere, "pegi", "esrb"]) {
    const n = notes?.[systeme];
    const note = chaine(n?.rating);
    if (!note) continue;
    return {
      systeme: systeme.toUpperCase(),
      note: note.toUpperCase(),
      descripteurs: (chaine(n.descriptors) ?? "")
        .split(/[\r\n]/)
        .map((s) => s.trim())
        .filter(Boolean),
    };
  }
  return null;
}

// --- Lire les réponses ------------------------------------------------------------------

/** `steam/apps/…/ss_xxx.jpg?t=…` → miniature 600×338 et grande image 1920×1080. */
export function adressesCapture(chemin) {
  const q = chemin.indexOf("?");
  const base = q < 0 ? chemin : chemin.slice(0, q);
  const suffixe = q < 0 ? "" : chemin.slice(q);
  const sansExt = base.endsWith(".jpg") ? base.slice(0, -4) : base;
  return {
    mini: `${IMAGES}${sansExt}.600x338.jpg${suffixe}`,
    grande: `${IMAGES}${sansExt}.1920x1080.jpg${suffixe}`,
  };
}

/**
 * Une entrée de `trailers.highlights`.
 *
 * 🔑 LE SON EST DANS LE FLUX HLS, PAS DANS LE « MICROTRAILER » (mp4/webm, muet, quelques
 * secondes), gardé en repli pour un navigateur qui ne saurait pas lire le flux.
 */
function bandeAnnonce(t) {
  const micro = tableau(t?.microtrailer);
  const fichier = (genre) => {
    const f = chaine(micro.find((m) => m?.type === genre)?.filename);
    return f === null ? null : `${VIDEOS}${f}`;
  };
  const flux = chaine(tableau(t?.adaptive_trailers).find((f) => f?.encoding === "hls_h264")?.cdn_path);
  const affiche = chaine(t?.screenshot_full);
  const b = {
    nom: (chaine(t?.trailer_name) ?? "").trim(),
    affiche: affiche === null ? null : `${IMAGES}steam/apps/${affiche}`,
    hls: flux === null ? null : `${VIDEOS}${flux}`,
    mp4: fichier("video/mp4"),
    webm: fichier("video/webm"),
  };
  // Rien à lire du tout : pas une bande-annonce.
  return b.hls || b.mp4 || b.webm ? b : null;
}

export const ficheVide = () => ({
  reviewScore: 0,
  reviewPercent: null,
  reviewCount: 0,
  tags: [],
  tagIds: [],
  etiquettes: [],
  captures: [],
  bandesAnnonces: [],
  langues: [],
  classification: null,
  categories: [],
  succes: null,
  configuration: null,
  dlc: [],
  site: null,
  metacritic: null,
  metacriticUrl: null,
  resume: null,
  description: [],
  demos: [],
  jeuComplet: null,
});

export function lireItem(f, item, noms) {
  f.demos = demos(item);
  const avis = item?.reviews?.summary_filtered;
  f.reviewCount = entier(avis?.review_count) ?? 0;
  if (f.reviewCount > 0) {
    f.reviewScore = entier(avis?.review_score) ?? 0;
    f.reviewPercent = entier(avis?.percent_positive);
  }

  const ids = tableau(item?.tags)
    .map((t) => entier(t?.tagid))
    .filter((id) => id !== null);
  f.tagIds = ids.filter((id) => !TAGS_ECARTES.has(id)).slice(0, 6);
  f.tags = f.tagIds.map((id) => noms.get(id)).filter(Boolean).slice(0, 4);
  // Nom et identifiant ENSEMBLE : un tag que Steam ne nomme pas manquerait à l'un, pas à l'autre.
  f.etiquettes = f.tagIds
    .filter((id) => !EXCLUS.has(id) && noms.has(id))
    .map((id) => ({ id, nom: noms.get(id) }))
    .slice(0, 5);

  f.captures = tableau(item?.screenshots?.all_ages_screenshots)
    .map((s) => chaine(s?.filename))
    .filter((nom) => nom?.includes(".jpg"))
    .map(adressesCapture);

  f.bandesAnnonces = tableau(item?.trailers?.highlights)
    .map(bandeAnnonce)
    .filter((b) => b !== null)
    .slice(0, MAX_BANDES_ANNONCES);

  f.langues = tableau(item?.supported_languages)
    .filter((l) => l?.supported !== false)
    .map((l) => {
      const code = CODES_LANGUE[entier(l?.elanguage) ?? -1];
      return code ? { code, voix: l.full_audio === true, sousTitres: l.subtitles === true } : null;
    })
    .filter((l) => l !== null);
}

/** Le jeu complet d'une démo. Steam donne son appid en texte (`"3642910"`). */
function jeuComplet(v) {
  const appId = entier(v?.appid) ?? (/^\d+$/.test(chaine(v?.appid) ?? "") ? Number(v.appid) : null);
  const titre = chaine(v?.name)?.trim();
  return appId === null || !titre ? null : { appId, titre };
}

export function lireDetails(f, d, region) {
  f.categories = tableau(d?.categories)
    .map((c) => {
      const id = entier(c?.id);
      const libelle = chaine(c?.description);
      return id === null || libelle === null ? null : { id, libelle };
    })
    .filter((c) => c !== null);
  const succes = entier(d?.achievements?.total);
  f.succes = succes ? succes : null;
  f.dlc = tableau(d?.dlc).map(entier).filter((a) => a !== null);
  f.site = chaine(d?.website)?.startsWith("https://") ? d.website : null;
  const note = entier(d?.metacritic?.score);
  f.metacritic = note >= 1 && note <= 100 ? note : null;
  const page = chaine(d?.metacritic?.url);
  f.metacriticUrl = f.metacritic !== null && page?.startsWith("https://www.metacritic.com/") ? page : null;
  const resume = chaine(d?.short_description);
  f.resume = resume === null ? null : texteBrut(resume) || null;
  f.classification = classification(d?.ratings, region);
  f.configuration = configuration(d?.pc_requirements);
  f.description = blocs(chaine(d?.about_the_game) ?? "");
  // `appdetails` donne aussi les démos : le repli quand `GetItems` n'a pas répondu.
  if (!f.demos.length) {
    f.demos = tableau(d?.demos).map((demo) => entier(demo?.appid)).filter((a) => a !== null);
  }
  f.jeuComplet = jeuComplet(d?.fullgame);
}

/**
 * La fiche Steam d'un jeu ; `null` si Steam n'a rien répondu du tout.
 * `details` : la réponse `appdetails`, déjà demandée par l'appelant (elle sert aussi ailleurs).
 */
export async function ficheSteam(ctx, appid, details) {
  const [items, d, { noms }] = await Promise.all([
    steamItems(ctx, GET_ITEMS, {
      ids: [{ appid }],
      context: contexteSteam(ctx),
      data_request: {
        include_screenshots: true,
        include_trailers: true,
        include_supported_languages: true,
        include_reviews: true,
        include_tag_count: 12,
      },
    }),
    details,
    chargerTags(ctx),
  ]);
  const item = items?.[0] ?? null;
  if (item === null && d === null) return null;
  const f = ficheVide();
  if (item !== null) lireItem(f, item, noms);
  if (d !== null) lireDetails(f, d, ctx.region);
  return f;
}
