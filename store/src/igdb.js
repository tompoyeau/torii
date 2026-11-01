/**
 * Le descriptif IGDB d'une fiche (via le proxy, qui détient le jeton Twitch et garde les
 * réponses 24 h). Mêmes règles que `metadata/igdb.rs` dans l'application installée.
 *
 * ⚠️ `search` est flou : on ne prend jamais le premier résultat, le nom normalisé doit être
 * identique. `~` est un égal insensible à la casse, PAS un « contient ».
 */

import { chaine, entier, lireJson, tableau, versProxy } from "./lib.js";

const IMG = "https://images.igdb.com/igdb/image/upload";
const STEAM_SOURCE = 1;
const GOG_SOURCE = 5;
const EPIC_SOURCE = 26;
const SITE_OFFICIEL = 1;
const SITE_STEAM = 13;
const SITE_EPIC = 16;
const SITE_GOG = 17;

const CHAMPS =
  "fields id, name, alternative_names.name, parent_game, genres.name, themes, game_modes, " +
  "player_perspectives, external_games.uid, external_games.external_game_source, summary, " +
  "cover.image_id, artworks.image_id, screenshots.image_id, involved_companies.company.name, " +
  "involved_companies.developer, first_release_date;";

/** Une requête Apicalypse. `null` = panne ; un tableau vide veut dire « IGDB n'a rien trouvé ». */
async function requete(ctx, endpoint, corps) {
  try {
    const reponse = await versProxy(ctx, endpoint, { method: "POST", body: corps });
    const donnees = reponse.ok ? lireJson(await reponse.text()) : null;
    if (donnees === null) ctx.partiel = true;
    return donnees;
  } catch (e) {
    console.error(`IGDB injoignable : ${endpoint} —`, e);
    ctx.partiel = true;
    return null;
  }
}

const image = (id, taille) => `${IMG}/t_${taille}/${id}.jpg`;

/** Abrège les libellés entre parenthèses (« Real Time Strategy (RTS) » → « RTS »). */
export function genreCourt(nom) {
  const o = nom.indexOf("(");
  const c = nom.lastIndexOf(")");
  return o >= 0 && c > o + 1 ? nom.slice(o + 1, c).trim() : nom;
}

/** Les genres IGDB n'existent qu'en anglais, et leur vocabulaire est fermé (23 entrées). */
const GENRES_FR = {
  Adventure: "Aventure",
  "Card & Board Game": "Cartes et plateau",
  Fighting: "Combat",
  "Hack and slash/Beat 'em up": "Beat'em up",
  Indie: "Indépendant",
  Music: "Musique",
  Pinball: "Flipper",
  Platform: "Plateforme",
  "Point-and-click": "Pointer-cliquer",
  Puzzle: "Réflexion",
  "Quiz/Trivia": "Quiz",
  Racing: "Course",
  RTS: "Stratégie temps réel",
  RPG: "Jeu de rôle",
  Shooter: "Tir",
  Simulator: "Simulation",
  Strategy: "Stratégie",
  Tactical: "Tactique",
  TBS: "Stratégie au tour par tour",
  "Visual Novel": "Roman visuel",
};

/** En anglais, les sigles abrégés par `genreCourt` redeviennent lisibles. */
const GENRES_EN = {
  "Card & Board Game": "Card & board",
  "Hack and slash/Beat 'em up": "Beat 'em up",
  "Quiz/Trivia": "Quiz",
  RTS: "Real-time strategy",
  RPG: "Role-playing",
  Simulator: "Simulation",
  TBS: "Turn-based strategy",
};

/** Un genre inconnu ressort tel quel : mieux vaut un libellé anglais qu'un genre perdu. */
export function traduireGenre(genre, langue) {
  const table = langue === "fr" ? GENRES_FR : GENRES_EN;
  return Object.hasOwn(table, genre) ? table[genre] : genre;
}

/** Replie les lettres accentuées du latin, dans lequel les jeux sont titrés. */
const PLIAGE = {
  à: "a", á: "a", â: "a", ã: "a", ä: "a", å: "a", æ: "ae", ç: "c", è: "e", é: "e", ê: "e", ë: "e",
  ì: "i", í: "i", î: "i", ï: "i", ñ: "n", ò: "o", ó: "o", ô: "o", õ: "o", ö: "o", ø: "o", œ: "oe",
  ù: "u", ú: "u", û: "u", ü: "u", ý: "y", ÿ: "y", ß: "ss",
};

/** Réduit un titre à sa forme comparable : minuscules, sans ponctuation, sans accents. */
export function norm(s) {
  let out = "";
  for (const c of s.toLowerCase()) {
    if (/[\p{Alphabetic}\p{N}]/u.test(c)) out += PLIAGE[c] ?? c;
  }
  return out;
}

/**
 * `cible` (déjà normalisée) correspond-elle au nom principal OU à un nom alternatif ?
 *
 * 🔑 IGDB ne crée pas toujours une entrée par titre commercial : « Overwatch 2 » n'existe que
 * comme alias de « Overwatch ». ⚠️ Cela n'assouplit rien : l'égalité reste stricte, les DLC
 * (« Overwatch 2: Invasion Bundle ») restent rejetés.
 */
export function nomCorrespond(g, cible) {
  if (norm(chaine(g?.name) ?? "") === cible) return true;
  return tableau(g?.alternative_names).some((n) => norm(chaine(n?.name) ?? "") === cible);
}

/** Nettoie un titre pour l'insérer dans une chaîne Apicalypse. */
const titrePropre = (t) => t.replace(/["™®©]/g, "").trim();

/** Ce que la fiche retient d'un objet jeu IGDB. */
export function lireMeta(g, langue) {
  const genre = chaine(tableau(g?.genres)[0]?.name);
  const heroId = chaine(tableau(g?.artworks)[0]?.image_id) ?? chaine(tableau(g?.screenshots)[0]?.image_id);
  const couverture = chaine(g?.cover?.image_id);
  const sortie = g?.first_release_date;
  return {
    id: entier(g?.id),
    genre: genre === null ? null : traduireGenre(genreCourt(genre), langue),
    description: chaine(g?.summary),
    coverUrl: couverture === null ? null : image(couverture, "cover_big_2x"),
    heroUrl: heroId === null ? null : image(heroId, "1080p"),
    developer: chaine(tableau(g?.involved_companies).find((c) => c?.developer === true)?.company?.name),
    year: Number.isInteger(sortie) ? 1970 + Math.trunc(sortie / 31_556_952) : null,
    screenshots: tableau(g?.screenshots)
      .map((s) => chaine(s?.image_id))
      .filter((id) => id !== null)
      .slice(0, 6)
      .map((id) => image(id, "1080p")),
  };
}

/** Le descriptif d'un jeu Steam, par son appid (`external_games` puis `games`). */
export async function metaSteam(ctx, appid) {
  const liens = await requete(
    ctx,
    "external_games",
    `fields game,uid; where external_game_source = ${STEAM_SOURCE} & uid = ("${appid}"); limit 500;`,
  );
  const ids = [...new Set(tableau(liens).map((e) => entier(e?.game)).filter((g) => g !== null))];
  if (!ids.length) return null;
  const jeux = await requete(ctx, "games", `${CHAMPS} where id = (${ids.join(",")}); limit 500;`);
  const dernier = tableau(jeux).at(-1);
  return dernier ? lireMeta(dernier, ctx.langue) : null;
}

/** Le descriptif d'un jeu hors Steam, par son titre : égalité exacte, puis `search` filtré. */
export async function metaTitre(ctx, titre) {
  const propre = titrePropre(titre);
  if (!propre) return null;
  const cible = norm(propre);

  // 🔑 L'alias est INTERROGÉ ici : `search "Overwatch 2"` renvoie quinze DLC et jamais l'entrée
  // qui le porte. Il faut le demander à IGDB, pas l'espérer.
  const exacts = await requete(
    ctx,
    "games",
    `${CHAMPS} where name ~ "${propre}" | alternative_names.name ~ "${propre}"; limit 5;`,
  );
  // Sans réponse, un « rien trouvé » au second appel ne prouverait rien de plus.
  if (exacts === null) return null;
  const exact = tableau(exacts).find((g) => nomCorrespond(g, cible));
  if (exact) return lireMeta(exact, ctx.langue);

  const proches = await requete(ctx, "games", `search "${propre}"; ${CHAMPS} limit 15;`);
  const proche = tableau(proches).find((g) => nomCorrespond(g, cible));
  return proche ? lireMeta(proche, ctx.langue) : null;
}

// --- Le lien avec Steam ---------------------------------------------------------------

/**
 * L'identifiant Steam commun aux entrées dont le nom correspond exactement, DLC et éditions écartés.
 *
 * 🔑 UN SEUL, sinon aucun. IGDB connaît deux « Hades », dont un seul est sur Steam : c'est celui-là.
 * Deux jeux du même nom tous deux sur Steam ne donnent aucun lien : le deviner, c'est risquer
 * d'afficher la fiche d'un autre jeu.
 */
export function appidUnique(jeux, cible) {
  const ids = new Set();
  for (const g of jeux) {
    if (g?.parent_game != null || g?.version_parent != null || !nomCorrespond(g, cible)) continue;
    for (const e of tableau(g.external_games)) {
      if (e?.external_game_source !== STEAM_SOURCE) continue;
      const uid = chaine(e.uid);
      if (uid && /^\d+$/.test(uid)) ids.add(Number(uid));
    }
  }
  return ids.size === 1 ? [...ids][0] : null;
}

/** L'appid Steam d'un jeu acheté ailleurs, d'après IGDB, sur un nom exact. `null` : pas de lien sûr. */
export async function lienSteam(ctx, titre) {
  const propre = titrePropre(titre);
  if (!propre) return null;
  const jeux = await requete(
    ctx,
    "games",
    "fields name, alternative_names.name, parent_game, version_parent, external_games.uid, " +
      "external_games.external_game_source; " +
      `where name ~ "${propre}" | alternative_names.name ~ "${propre}"; limit 10;`,
  );
  return appidUnique(tableau(jeux), norm(propre));
}

// --- Le lien à partager ---------------------------------------------------------------

/**
 * La page Epic d'un jeu, sans la langue : un lien en `/en-US/` enverrait un ami français sur la
 * page anglaise. IGDB connaît trois formes d'adresse, qui donnent toutes la même page.
 */
export function pageEpic(url) {
  const i = url.indexOf("epicgames.com");
  if (i < 0) return null;
  const segments = url.slice(i + "epicgames.com".length).split(/[/?#]/).filter(Boolean);
  const p = segments.findIndex((s) => s === "p" || s === "product");
  const slug = p < 0 ? undefined : segments[p + 1];
  return slug ? `https://store.epicgames.com/p/${slug}` : null;
}

/** Les pages connues d'un jeu, d'après sa fiche IGDB. */
export function lirePages(g) {
  const p = { steam: null, epic: null, gog: null, site: null };
  for (const e of tableau(g?.external_games)) {
    const source = entier(e?.external_game_source);
    const url = chaine(e?.url);
    if (source === STEAM_SOURCE && p.steam === null) {
      const uid = chaine(e?.uid);
      if (uid && /^\d+$/.test(uid)) p.steam = `https://store.steampowered.com/app/${uid}/`;
    } else if (source === EPIC_SOURCE && p.epic === null && url) {
      p.epic = pageEpic(url);
    } else if (source === GOG_SOURCE && p.gog === null && url?.includes("gog.com/")) {
      p.gog = url;
    }
  }
  for (const w of tableau(g?.websites)) {
    const url = chaine(w?.url);
    if (url === null) continue;
    const type = entier(w?.type);
    if (type === SITE_STEAM && p.steam === null) p.steam = url;
    else if (type === SITE_EPIC && p.epic === null) p.epic = pageEpic(url);
    else if (type === SITE_GOG && p.gog === null) p.gog = url;
    else if (type === SITE_OFFICIEL && p.site === null) p.site = url;
  }
  return p;
}

/** Le lien à partager d'un jeu : sa page Steam, sinon Epic, GOG, ou son site officiel. */
export async function lienPartage(ctx, titre) {
  const meta = await metaTitre(ctx, titre);
  if (meta?.id == null) return null;
  const jeux = await requete(
    ctx,
    "games",
    "fields websites.url, websites.type, external_games.url, external_games.uid, " +
      `external_games.external_game_source; where id = ${meta.id};`,
  );
  const fiche = tableau(jeux)[0];
  if (!fiche) return null;
  const p = lirePages(fiche);
  return p.steam ?? p.epic ?? p.gog ?? p.site;
}
