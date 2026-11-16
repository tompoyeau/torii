/**
 * Reprise des réglages enregistrés dans le `localStorage` sous l'ancien nom de code de
 * l'application (`ludo-…`), devenu `torii-…` en 0.30.
 *
 * 🔑 CLÉ PAR CLÉ, AU MOMENT DE LA LIRE. Chaque module appelle cette fonction juste avant de
 * lire sa clé : la reprise ne dépend donc pas de l'ordre de chargement des modules. Une clé
 * déjà présente sous le nouveau nom gagne toujours, et l'ancienne est effacée dans tous les
 * cas : la reprise ne joue qu'une fois.
 *
 * ⚠️ À retirer quand plus personne ne met à jour depuis une version antérieure à la 0.30.
 */
const NOUVEAU = "torii-";
const ANCIEN = "ludo-";

export function reprendreAncienneCle(cle: string): void {
  if (!cle.startsWith(NOUVEAU)) return;
  try {
    const ancienne = ANCIEN + cle.slice(NOUVEAU.length);
    const valeur = localStorage.getItem(ancienne);
    if (valeur === null) return;
    if (localStorage.getItem(cle) === null) localStorage.setItem(cle, valeur);
    localStorage.removeItem(ancienne);
  } catch {
    /* stockage indisponible : il n'y a rien à reprendre. */
  }
}
