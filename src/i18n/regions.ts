/**
 * Région commerciale — à ne pas confondre avec la langue.
 *
 * 🔑 CE SONT DEUX RÉGLAGES, ET C'EST LE POINT CENTRAL DE TOUT CE MODULE. Jusqu'ici
 * l'application n'en avait qu'un seul, implicite : « français » voulait dire à la fois
 * l'interface en français, les descriptions demandées en français, les prix en euros,
 * la tarification française et les liens vers un revendeur français. Ça tient tant qu'on
 * ne s'adresse qu'à la France.
 *
 * Dès qu'on sort, les deux se séparent : un Belge veut l'interface en français et les
 * prix belges ; un Canadien anglophone veut l'anglais et des dollars canadiens ; un
 * Français expatrié aux États-Unis veut peut-être le français et les prix américains.
 * Lier les deux obligerait chacun d'eux à choisir ce qui le dérange le moins.
 *
 * ⚠️ Un anglophone à qui on affiche des prix en euros et des liens vers une boutique
 * française est plus mal servi qu'un anglophone à qui on n'affiche pas de prix du tout.
 */
import { computed, ref } from "vue";
import { etiquetteIntl } from "./index";

/** Code pays ISO 3166-1 alpha-2, tel qu'attendu par l'API de comparaison de prix. */
export type CodeRegion =
  | "AE" | "AR" | "AT" | "AU" | "BE" | "BG" | "BR" | "CA" | "CH" | "CL" | "CN" | "CO"
  | "CY" | "CZ" | "DE" | "DK" | "EE" | "EG" | "ES" | "FI" | "FR" | "GB" | "GR" | "HK"
  | "HR" | "HU" | "ID" | "IE" | "IL" | "IN" | "IT" | "JP" | "KR" | "KZ" | "LT" | "LU"
  | "LV" | "MT" | "MX" | "MY" | "NG" | "NL" | "NO" | "NZ" | "PE" | "PH" | "PL" | "PT"
  | "RO" | "SA" | "SE" | "SG" | "SI" | "SK" | "TH" | "TR" | "TW" | "UA" | "US" | "VN"
  | "ZA";

/**
 * Les régions proposées, avec la devise que le comparateur renvoie pour chacune.
 *
 * 🔑 CETTE TABLE EST UN RELEVÉ, PAS UNE CONNAISSANCE. Chaque devise a été lue dans la
 * réponse réelle du comparateur (`scripts/sonde-regions.mjs`, relevé du 13 septembre 2026),
 * et le relevé a contredit ce qu'on aurait écrit de mémoire : la Suisse, la Suède, la
 * Norvège, le Danemark, la Tchéquie, la Hongrie, la Roumanie et la Bulgarie sont tarifées
 * **en euros** ; le Mexique, Singapour ou Israël n'ont pas de tarif propre et reçoivent les
 * **prix américains en dollars**. Refaire la sonde avant d'ajouter un pays.
 *
 * ⚠️ LA DEVISE ICI EST INDICATIVE — elle étiquette le sélecteur avant qu'aucun prix ne soit
 * chargé, et décide si Instant Gaming est pertinent (`instantGamingPertinent`). Un prix
 * réel se formate toujours avec la devise renvoyée AVEC lui (`formatPrix`).
 *
 * ⚠️ PAS DE RUSSIE : le comparateur n'y a pas de tarif local, et Steam y bloque des jeux.
 *
 * ⚠️ La liste des pays « en euros » est dupliquée côté Rust (`locale::zone_euro`), qui
 * décide d'interroger Instant Gaming : les deux doivent rester identiques.
 */
export const REGIONS: { code: CodeRegion; devise: string }[] = [
  // Zone euro — et pays que le comparateur tarifie en euros sans être dans la zone.
  { code: "AT", devise: "EUR" },
  { code: "BE", devise: "EUR" },
  { code: "BG", devise: "EUR" },
  { code: "CH", devise: "EUR" },
  { code: "CY", devise: "EUR" },
  { code: "CZ", devise: "EUR" },
  { code: "DE", devise: "EUR" },
  { code: "DK", devise: "EUR" },
  { code: "EE", devise: "EUR" },
  { code: "ES", devise: "EUR" },
  { code: "FI", devise: "EUR" },
  { code: "FR", devise: "EUR" },
  { code: "GR", devise: "EUR" },
  { code: "HR", devise: "EUR" },
  { code: "HU", devise: "EUR" },
  { code: "IE", devise: "EUR" },
  { code: "IT", devise: "EUR" },
  { code: "LT", devise: "EUR" },
  { code: "LU", devise: "EUR" },
  { code: "LV", devise: "EUR" },
  { code: "MT", devise: "EUR" },
  { code: "NL", devise: "EUR" },
  { code: "NO", devise: "EUR" },
  { code: "PT", devise: "EUR" },
  { code: "RO", devise: "EUR" },
  { code: "SE", devise: "EUR" },
  { code: "SI", devise: "EUR" },
  { code: "SK", devise: "EUR" },
  // Pays à devise propre.
  { code: "AR", devise: "ARS" },
  { code: "AU", devise: "AUD" },
  { code: "BR", devise: "BRL" },
  { code: "CA", devise: "CAD" },
  { code: "CN", devise: "CNY" },
  { code: "GB", devise: "GBP" },
  { code: "ID", devise: "IDR" },
  { code: "IN", devise: "INR" },
  { code: "JP", devise: "JPY" },
  { code: "KR", devise: "KRW" },
  { code: "NZ", devise: "NZD" },
  { code: "PH", devise: "PHP" },
  { code: "PL", devise: "PLN" },
  { code: "TR", devise: "TRY" },
  { code: "TW", devise: "TWD" },
  { code: "US", devise: "USD" },
  // Pays sans tarif propre : le comparateur renvoie les prix américains, en dollars.
  { code: "AE", devise: "USD" },
  { code: "CL", devise: "USD" },
  { code: "CO", devise: "USD" },
  { code: "EG", devise: "USD" },
  { code: "HK", devise: "USD" },
  { code: "IL", devise: "USD" },
  { code: "KZ", devise: "USD" },
  { code: "MX", devise: "USD" },
  { code: "MY", devise: "USD" },
  { code: "NG", devise: "USD" },
  { code: "PE", devise: "USD" },
  { code: "SA", devise: "USD" },
  { code: "SG", devise: "USD" },
  { code: "TH", devise: "USD" },
  { code: "UA", devise: "USD" },
  { code: "VN", devise: "USD" },
  { code: "ZA", devise: "USD" },
];

// Aussitôt remplacée par `usePreferences.apply()`, au chargement du module.
const region = ref<CodeRegion>("US");

/** La région courante, en lecture seule. */
export const regionCourante = computed(() => region.value);

/** La devise attendue pour la région courante (étiquetage seulement — voir ci-dessus). */
export const deviseAttendue = computed(
  () => REGIONS.find((r) => r.code === region.value)?.devise ?? "USD",
);

/**
 * Instant Gaming n'est proposé qu'en zone euro.
 *
 * 🔑 Ce revendeur affiche des prix en euros, nativement, sans équivalent local. Le
 * glisser au milieu d'une liste en livres ou en dollars donnerait une ligne qu'on ne peut
 * pas comparer aux autres — et c'est précisément la comparaison qu'on vient chercher.
 * Mieux vaut une offre de moins qu'une offre trompeuse.
 */
export const instantGamingPertinent = computed(() => deviseAttendue.value === "EUR");

export function appliquerRegion(nouvelle: CodeRegion) {
  region.value = nouvelle;
}

/**
 * Le nom d'un pays dans la langue de l'interface.
 *
 * 🔑 `Intl.DisplayNames` évite de recopier soixante noms de pays dans chaque catalogue de
 * traduction — et de les recopier encore à chaque langue ajoutée.
 * Windows connaît déjà ces noms, dans toutes les langues.
 */
export function nomDeRegion(code: CodeRegion): string {
  try {
    return new Intl.DisplayNames([etiquetteIntl.value], { type: "region" }).of(code) ?? code;
  } catch {
    return code; // navigateur trop ancien : le code seul reste compréhensible
  }
}

/**
 * La région de « Suivre Windows », d'après la langue d'affichage du système.
 *
 * ⚠️ `navigator.language` donne `fr-BE` pour un Belge francophone : c'est la partie pays
 * qui nous intéresse ici, pas la langue.
 *
 * 🔑 REPLI SUR LES ÉTATS-UNIS, plus sur la France. Une locale sans pays (`es` tout court)
 * ou un pays absent de la table donnaient des prix français en euros — un héritage de
 * l'époque où Torii ne visait que la France, absurde pour quelqu'un à l'autre bout du
 * monde. Le dollar est le repli le plus lisible, et c'est d'ailleurs ce que le comparateur
 * renvoie lui-même pour les pays sans tarif propre.
 *
 * ⚠️ Les utilisateurs d'avant les langues ne passent jamais par ici : leur région est
 * épinglée sur la France, la seule qu'ils aient connue (`usePreferences`).
 */
export function regionDuSysteme(): CodeRegion {
  if (typeof navigator === "undefined") return "US";
  const connue = (pays: string | undefined): pays is CodeRegion => REGIONS.some((r) => r.code === pays);
  // ⚠️ WebView2 peut annoncer une langue SANS PAYS (`fr` tout court) : on cherche d'abord
  // un pays dans toute la liste, puis on se rabat sur le pays principal de la langue —
  // sinon « Suivre Windows » donnait des dollars à un Windows français.
  const langues = navigator.languages?.length ? navigator.languages : [navigator.language];
  for (const l of langues) {
    const pays = l.split("-")[1]?.toUpperCase();
    if (connue(pays)) return pays;
  }
  const pays = PAYS_DE_LA_LANGUE[(langues[0] ?? "").split("-")[0].toLowerCase()];
  return connue(pays) ? pays : "US";
}

/** Pays principal d'une langue annoncée sans pays. L'anglais n'y est pas : repli US. */
const PAYS_DE_LA_LANGUE: Record<string, string> = {
  fr: "FR", de: "DE", es: "ES", it: "IT", pt: "PT", nl: "NL", pl: "PL", ja: "JP",
  ko: "KR", zh: "CN", tr: "TR", sv: "SE", da: "DK", nb: "NO", no: "NO", fi: "FI",
  cs: "CZ", el: "GR", hu: "HU", ro: "RO", uk: "UA", id: "ID", th: "TH", vi: "VN",
};
