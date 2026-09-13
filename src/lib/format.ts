import { etiquetteIntl, t } from "../i18n";
import { deviseAttendue } from "../i18n/regions";

/**
 * Formate un montant dans la devise où il a été relevé.
 *
 * ⚠️ CE FICHIER S'APPELAIT `formatEur` ET SUPPOSAIT L'EURO PARTOUT. C'était vrai tant que
 * la région était figée sur la France. Ça ne l'est plus : l'API de comparaison renvoie les
 * prix dans la devise du pays demandé, et un montant américain affiché « 59,99 € » n'est
 * pas une approximation, c'est un chiffre faux.
 *
 * 🔑 LA DEVISE VIENT DU PRIX, PAS DU RÉGLAGE. L'argument `devise` est celui que l'API a
 * renvoyé avec ce montant-là. On ne retombe sur la devise de la région que lorsqu'un
 * appelant n'en a pas — et ce repli est une commodité de transition, pas une règle :
 * chaque source de prix doit finir par transporter la sienne.
 *
 * 🔑 `Intl` PLACE LE SYMBOLE, PAS NOUS. « 44,99 € » en français, « $44.99 » en anglais :
 * le symbole change de côté, le séparateur décimal change de caractère, et l'espace avant
 * l'euro est une insécable que personne ne tape à la main. C'était la vraie raison de ne
 * pas garder un gabarit maison.
 */
export function formatPrix(montant: number, devise?: string | null): string {
  const code = devise || deviseAttendue.value;
  try {
    // ⚠️ PAS DE `minimumFractionDigits` FORCÉ. Le yen, le won ou la roupie indonésienne n'ont
    // pas de centimes : deux décimales imposées donnaient « ¥8,499.00 » pour un vrai prix
    // japonais. `Intl` connaît le nombre de décimales de chaque devise, et donne exactement
    // le même résultat qu'avant pour l'euro, le dollar ou la livre (vérifié).
    return new Intl.NumberFormat(etiquetteIntl.value, {
      style: "currency",
      currency: code,
    }).format(montant);
  } catch {
    // Code de devise inconnu d'`Intl` (jamais vu, mais une exception ici blanchirait
    // tout le comparateur) : on affiche le montant et le code, ce qui reste lisible.
    return `${montant.toFixed(2)} ${code}`;
  }
}

/**
 * « il y a 3 min » / « 3 min. ago » — à partir d'un horodatage Unix en **secondes**.
 *
 * 🔑 `Intl.RelativeTimeFormat` REMPLACE TROIS GABARITS ÉCRITS À LA MAIN (Paramètres,
 * bibliothèque d'un ami…), qui collaient « il y a » devant un nombre. En anglais le
 * nombre passe devant (« 3 min. ago ») : aucune concaténation ne s'adapte à ça, alors
 * qu'`Intl` connaît déjà l'ordre, les abréviations et les pluriels de chaque langue.
 *
 * `seuilInstant` : en deçà (en secondes), on écrit « à l'instant » plutôt qu'un nombre
 * de secondes qui n'apprendrait rien. Les écrans n'ont pas tous la même précision — une
 * bibliothèque partagée datée « il y a 40 min » n'a pas d'intérêt, un envoi qu'on vient
 * de déclencher, si.
 */
export function ilYA(horodatage: number, seuilInstant = 90): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - horodatage);
  if (s < seuilInstant) return t("commun.aLInstant");
  const rtf = new Intl.RelativeTimeFormat(etiquetteIntl.value, { style: "short", numeric: "always" });
  if (s < 3600) return rtf.format(-Math.round(s / 60), "minute");
  if (s < 86400) return rtf.format(-Math.round(s / 3600), "hour");
  return rtf.format(-Math.round(s / 86400), "day");
}
