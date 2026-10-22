import { etiquetteIntl, t, type Cle } from "../i18n";
import type { StoreShelfItem } from "../types";

/**
 * Libellés des cartes de la Boutique : avis Steam, dates de sortie, fin d'une offre.
 *
 * 🔑 Les dates passent par `Intl`, pas par un gabarit : « 29 oct. » en français,
 * « Oct 29 » en anglais — l'ordre jour/mois change avec la langue.
 */

/** Appréciation Steam (1 à 9) en toutes lettres ; vide s'il n'y a pas encore d'avis. */
export function libelleAvis(score: number): string {
  return score >= 1 && score <= 9 ? t(`boutique.accueil.avis.a${score}` as Cle) : "";
}

/** Ton de l'appréciation, pour la pastille de couleur. */
export function tonAvis(score: number): "pos" | "mixte" | "neg" | "aucun" {
  if (score >= 6) return "pos";
  if (score === 5) return "mixte";
  if (score >= 1) return "neg";
  return "aucun";
}

/** Une date sans l'année si c'est l'année en cours (« 29 oct. »), avec sinon. */
function jourMois(d: Date): string {
  const memeAnnee = d.getFullYear() === new Date().getFullYear();
  return new Intl.DateTimeFormat(etiquetteIntl.value, {
    day: "numeric",
    month: "short",
    ...(memeAnnee ? {} : { year: "numeric" }),
  }).format(d);
}

/**
 * Sortie d'un jeu, à la précision que l'éditeur a donnée à Steam : un jour, un mois,
 * un trimestre, une année — ou un texte libre (« Prochainement »).
 */
export function libelleSortie(jeu: StoreShelfItem): string {
  const d = jeu.releaseDate ? new Date(jeu.releaseDate * 1000) : null;
  if (!jeu.comingSoon) return d ? t("boutique.accueil.carte.sortiLe", { date: jourMois(d) }) : "";
  if (!d || jeu.releaseDisplay.startsWith("text")) return jeu.releaseText ?? "";
  switch (jeu.releaseDisplay) {
    case "date_month":
      return t("boutique.accueil.carte.sortiePrevue", {
        date: new Intl.DateTimeFormat(etiquetteIntl.value, { month: "long", year: "numeric" }).format(d),
      });
    case "date_quarter":
      return t("boutique.accueil.carte.sortiePrevue", {
        date: t("boutique.accueil.carte.trimestre", { n: Math.floor(d.getMonth() / 3) + 1, annee: d.getFullYear() }),
      });
    case "date_year":
      return t("boutique.accueil.carte.sortiePrevue", { date: String(d.getFullYear()) });
    default:
      return t("boutique.accueil.carte.sortie", { date: jourMois(d) });
  }
}

/**
 * Fin d'une offre (« Jusqu'au 3 nov. »). L'heure n'apparaît qu'à moins de deux jours de
 * l'échéance : c'est là qu'elle compte (un jeu offert jusqu'à 17 h aujourd'hui).
 */
export function libelleFin(expiry: string | null | undefined): string {
  if (!expiry) return "";
  const d = new Date(expiry);
  if (Number.isNaN(d.getTime()) || d.getTime() < Date.now()) return "";
  const proche = d.getTime() - Date.now() < 2 * 86_400_000;
  const date = proche
    ? new Intl.DateTimeFormat(etiquetteIntl.value, { day: "numeric", month: "short", hour: "numeric", minute: "2-digit" }).format(d)
    : jourMois(d);
  return t("boutique.accueil.carte.jusquA", { date });
}

/** Adresse de la page Steam d'un jeu, pour ceux que le comparateur ne connaît pas. */
export function pageSteam(appId: number): string {
  return `https://store.steampowered.com/app/${appId}`;
}
