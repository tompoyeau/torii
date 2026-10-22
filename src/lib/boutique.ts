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

// --- Fiche produit ---

/** Un nombre à la manière de la langue (« 91 417 » / « 91,417 »). */
export function nombre(n: number): string {
  return new Intl.NumberFormat(etiquetteIntl.value).format(n);
}

/** Une date `AAAA-MM-JJ` en toutes lettres (« 10 décembre 2020 »). */
export function dateLongue(iso: string): string {
  const d = new Date(`${iso}T12:00:00`);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(etiquetteIntl.value, { day: "numeric", month: "long", year: "numeric" }).format(d);
}

/**
 * Nom d'une langue dans la langue de l'interface, d'après son étiquette BCP 47 :
 * `Intl` le connaît déjà (« français », « portugais (Brésil) »), on n'a rien à traduire.
 */
export function nomLangue(code: string): string {
  try {
    const nom = new Intl.DisplayNames([etiquetteIntl.value], { type: "language" }).of(code) ?? code;
    return nom.charAt(0).toLocaleUpperCase(etiquetteIntl.value) + nom.slice(1);
  } catch {
    return code;
  }
}

/** Libellés Steam des descripteurs PEGI → clés de traduction. Les autres restent tels quels. */
const DESCRIPTEURS: Record<string, string> = {
  violence: "violence",
  "bad language": "langage",
  fear: "peur",
  sex: "sexe",
  drugs: "drogue",
  discrimination: "discrimination",
  gambling: "jeux",
  "in-game purchases": "achats",
  "in-game purchases (includes random items)": "achatsAleatoires",
};
export function descripteur(brut: string): string {
  const cle = DESCRIPTEURS[brut.trim().toLowerCase()];
  return cle ? t(`boutique.produit.pegi.${cle}` as Cle) : brut;
}

/**
 * Ce qu'il faut savoir pour activer une offre : « Sans DRM », « Activation : GOG »… Rien
 * quand la boutique vend sur sa propre plateforme (Steam chez Steam, Epic chez Epic) : ça
 * va sans dire.
 */
export function activation(boutique: string, drm: string[] | undefined): string {
  if (!drm?.length) return "";
  if (drm.some((d) => d.toLowerCase() === "drm free")) return t("boutique.produit.sansDrm");
  const chez = boutique.toLowerCase();
  const autres = drm.filter((d) => !chez.includes(d.toLowerCase().split(" ")[0]));
  return autres.length ? t("boutique.produit.activation", { drm: autres.join(", ") }) : "";
}

/**
 * Titre d'une bande-annonce. Certains éditeurs laissent le nom du fichier de montage
 * (« 032_2_1_UpdateTrailer_FR_PEGI 18_16x9_1080_Burned in Subs », relevé sur Cyberpunk
 * 2077) : on affiche alors « Bande-annonce 4 ».
 */
export function nomBandeAnnonce(nom: string, rang: number): string {
  const brut = !nom.trim() || /_|\b(16x9|1080p?|PEGI\s?\d+)\b/i.test(nom);
  return brut ? t("boutique.produit.bandeAnnonceN", { n: rang }) : nom.trim();
}

/**
 * La WebView sait-elle lire un flux HLS (les bandes-annonces avec le son) ? Oui pour le
 * WebView2 d'Edge 155 (mesuré), sans bibliothèque ; sinon, l'aperçu muet prend le relais.
 */
export function hlsLisible(): boolean {
  try {
    return document.createElement("video").canPlayType("application/vnd.apple.mpegurl") !== "";
  } catch {
    return false;
  }
}

export type Verdict = "record" | "proche" | "troisMois" | "annee" | "cher";

/**
 * « Bon moment pour acheter ? » : le prix actuel comparé aux plus bas du comparateur.
 *
 * 🔑 Les seuils se lisent comme un acheteur les lirait : au plus bas (à un centime près),
 * à 10 % du record, au plus bas des 3 derniers mois, à 10 % du plus bas de l'année, sinon
 * « déjà vu moins cher ». Les plus bas tiennent compte de toutes les boutiques suivies
 * par le comparateur, revendeurs compris.
 */
export function verdict(prix: number, bas3m?: number | null, bas1a?: number | null, basTout?: number | null): Verdict | null {
  if (basTout == null) return null;
  if (prix <= basTout + 0.01) return "record";
  if (prix <= basTout * 1.1) return "proche";
  if (bas3m != null && prix <= bas3m + 0.01) return "troisMois";
  if (bas1a != null && prix <= bas1a * 1.1) return "annee";
  return "cher";
}
