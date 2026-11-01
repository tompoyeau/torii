/**
 * La fiche d'un jeu de la bibliothèque : ce que Steam en dit, pour tout jeu qu'on peut relier à
 * Steam avec certitude. Même contenu que `metadata/possede.rs` dans l'application, sans ce qui
 * dépend du compte Steam (succès, contenus possédés) : son cookie reste sur le PC.
 *
 * 🔑 LE LIEN AVEC STEAM N'EST JAMAIS DEVINÉ PAR LE TITRE. Chercher « Control » sur Steam ramène
 * CONTROL Resonant : une fiche fausse est pire qu'une fiche courte. Liens admis : l'appid envoyé
 * par le PC pour un jeu possédé sur Steam, ou celui qu'IGDB associe à un nom exact (`lienSteam`).
 */

import { chaine, entier, lireJson, DELAI_MS, tableau } from "./lib.js";
import { appdetails } from "./steam.js";
import { ficheSteam, imageSteam, texteBrut } from "./fiche.js";

const NEWS = "https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/";
const JOUEURS = "https://api.steampowered.com/ISteamUserStats/GetNumberOfCurrentPlayers/v1/";
/** Adresse que Steam substitue au jeton `{STEAM_CLAN_IMAGE}` des annonces. */
const IMAGES_CLAN = "https://clan.akamai.steamstatic.com/images/";
const NB_ACTUALITES = 4;
const LONGUEUR_EXTRAIT = 180;

/** Un JSON de l'API publique de Steam ; `null` si elle ne répond pas. */
async function apiSteam(ctx, url) {
  try {
    const reponse = await fetch(url, { signal: AbortSignal.timeout(DELAI_MS) });
    const donnees = reponse.ok ? lireJson(await reponse.text()) : null;
    if (donnees === null) ctx.partiel = true;
    return donnees;
  } catch (e) {
    console.error(`Steam injoignable : ${url.slice(0, 80)} —`, e);
    ctx.partiel = true;
    return null;
  }
}

/** Coupe sur un mot, sans laisser de ponctuation avant les points de suspension. */
function tronquer(texte, max) {
  const lettres = [...texte];
  if (lettres.length <= max) return texte;
  const coupe = lettres.slice(0, max).join("");
  const espace = coupe.lastIndexOf(" ");
  const debut = espace === -1 ? coupe : coupe.slice(0, espace);
  return `${debut.replace(/[\s\p{P}]+$/u, "")}…`;
}

/** Le début du texte : sans BBCode, sans HTML, sans jeton d'image ni lien, coupé sur un mot. */
export function extrait(contenu) {
  const sansBbcode = contenu.replace(/\[[^\]]*\]/g, " ");
  const mots = texteBrut(sansBbcode)
    .split(/\s+/)
    .filter((m) => m && !m.startsWith("{STEAM_CLAN_IMAGE}") && !m.startsWith("http"));
  return tronquer(mots.join(" "), LONGUEUR_EXTRAIT);
}

/** La première image de l'annonce, si elle vient des serveurs de Steam. */
export function imageAnnonce(contenu) {
  for (const morceau of contenu.split(/[\s[\]"'<>=()]/)) {
    const url = morceau.startsWith("{STEAM_CLAN_IMAGE}/")
      ? `${IMAGES_CLAN}${morceau.slice("{STEAM_CLAN_IMAGE}/".length)}`
      : morceau;
    if (/\.(jpe?g|png|gif|webp)$/i.test(url) && imageSteam(url)) return url;
  }
  return null;
}

/** Une annonce de l'éditeur ; `null` sans titre, ou sans adresse sûre. */
export function lireActualite(n) {
  const titre = texteBrut(chaine(n?.title) ?? "");
  const url = chaine(n?.url);
  if (!titre || !url?.startsWith("https://")) return null;
  const contenu = chaine(n?.contents) ?? "";
  return { titre, url, date: entier(n?.date) ?? 0, extrait: extrait(contenu), image: imageAnnonce(contenu) };
}

/**
 * Les dernières annonces officielles. Le fil de Steam mêle aux annonces de l'éditeur des articles
 * de presse : `feeds` ne garde que les premières.
 */
async function actualites(ctx, appid) {
  const racine = await apiSteam(
    ctx,
    `${NEWS}?appid=${appid}&count=${NB_ACTUALITES}&maxlength=0&feeds=steam_community_announcements`,
  );
  return tableau(racine?.appnews?.newsitems).map(lireActualite).filter((a) => a !== null);
}

/** La fiche Steam et les annonces. Les contenus additionnels supposent de savoir ce qu'on possède. */
export async function fichePossede(ctx, appid) {
  const [steam, annonces] = await Promise.all([
    ficheSteam(ctx, appid, appdetails(ctx, appid)),
    actualites(ctx, appid),
  ]);
  return { appId: appid, steam, actualites: annonces, dlc: [], dlcPossedes: [] };
}

/** Les joueurs en ce moment. `null` si Steam ne donne pas de chiffre valable pour ce jeu. */
export async function joueursEnCeMoment(ctx, appid) {
  const racine = await apiSteam(ctx, `${JOUEURS}?appid=${appid}`);
  return racine?.response?.result === 1 ? (entier(racine.response.player_count) ?? null) : null;
}
