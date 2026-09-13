import { etiquetteIntl, t } from "../i18n";
/** Hash déterministe simple (djb2) pour dériver une couleur stable par jeu. */
function hash(seed: string): number {
  let h = 5381;
  for (let i = 0; i < seed.length; i++) h = (h * 33) ^ seed.charCodeAt(i);
  return Math.abs(h);
}

/**
 * Jaquette de secours : un dégradé cohérent et stable, dérivé de l'identifiant
 * du jeu. Utilisé tant qu'aucune vraie jaquette n'est disponible.
 */
export function gradientFor(seed: string): string {
  const h = hash(seed);
  const hue = h % 360;
  const hue2 = (hue + 40 + (h % 40)) % 360;
  return `linear-gradient(150deg, hsl(${hue} 55% 32%), hsl(${hue2} 62% 52%))`;
}

/**
 * Avatar de secours : un disque degrade portant une initiale, encode en `data:`.
 *
 * Meme principe que `gradientFor` — un visuel stable derive d'une graine — mais pour une
 * personne plutot qu'un jeu, et en SVG car un avatar est un `<img>`, pas un fond CSS.
 */
export function avatarFictif(initiale: string, de: string, vers: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">` +
    `<stop offset="0" stop-color="${de}"/><stop offset="1" stop-color="${vers}"/>` +
    `</linearGradient></defs><circle cx="40" cy="40" r="40" fill="url(#g)"/>` +
    `<text x="40" y="41" fill="#fff" fill-opacity=".92" font-family="Segoe UI,sans-serif"` +
    ` font-size="38" font-weight="700" text-anchor="middle" dominant-baseline="central">` +
    `${initiale}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/**
 * Formate un horodatage Unix (secondes) en libellé relatif court : « il y a 3 j »,
 * « hier », « 2 wk. ago ».
 *
 * 🔑 `Intl.RelativeTimeFormat` et non plus un gabarit « il y a … » : l'anglais place le
 * nombre devant (« 3 days ago ») et dit « yesterday » là où le français dit « hier ».
 * `numeric: "auto"` donne ces mots-là tout seul pour la veille.
 *
 * ⚠️ Le libellé est calculé au chargement de la bibliothèque et rangé dans le jeu
 * (`lastPlayed`) : changer de langue en cours de session ne le retraduit pas. C'est la
 * même limite que pour les descriptions, et le même remède — le redémarrage proposé
 * dans les Paramètres.
 */
export function relativeTime(unixSeconds: number): string {
  const diff = Date.now() / 1000 - unixSeconds;
  const day = 86400;
  if (diff < 3600) return t("bibliotheque.temps.moinsDUneHeure");
  const rtf = new Intl.RelativeTimeFormat(etiquetteIntl.value, { style: "short", numeric: "auto" });
  if (diff < day) return rtf.format(-Math.round(diff / 3600), "hour");
  if (diff < 7 * day) return rtf.format(-Math.max(1, Math.round(diff / day)), "day");
  if (diff < 30 * day) return rtf.format(-Math.round(diff / (7 * day)), "week");
  return rtf.format(-Math.round(diff / (30 * day)), "month");
}
