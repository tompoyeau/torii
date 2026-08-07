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
