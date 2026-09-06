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

/** Formate un horodatage Unix (secondes) en libellé relatif court en français. */
export function relativeTime(unixSeconds: number): string {
  const diff = Date.now() / 1000 - unixSeconds;
  const day = 86400;
  if (diff < 3600) return "il y a moins d'une heure";
  if (diff < day) return `il y a ${Math.round(diff / 3600)} h`;
  if (diff < 2 * day) return "hier";
  if (diff < 7 * day) return `il y a ${Math.round(diff / day)} j`;
  if (diff < 30 * day) return `il y a ${Math.round(diff / (7 * day))} sem`;
  return `il y a ${Math.round(diff / (30 * day))} mois`;
}
