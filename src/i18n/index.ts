/**
 * Traduction de l'interface.
 *
 * 🔑 POURQUOI PAS `vue-i18n`. L'application a six dépendances, et chacune a été discutée.
 * `vue-i18n` pèse une trentaine de kilo-octets pour un catalogue de fonctionnalités dont
 * on n'utiliserait ici que deux lignes : on a deux langues, dont les règles de pluriel
 * sont identiques (un / plusieurs), et aucun formatage de date à déléguer — `Intl` s'en
 * charge déjà. Ce fichier fait le même travail en une soixantaine de lignes de code.
 *
 * 🔑 L'API IMITE VOLONTAIREMENT CELLE DE `vue-i18n` (`t("cle.sous_cle", { nom })`). Si un
 * jour le besoin dépasse ce que fait ce fichier — une troisième langue à règles de pluriel
 * différentes, du formatage riche —, le remplacement se fait par un changement d'import,
 * pas par une réécriture des 900 appels.
 *
 * ⚠️ LE FRANÇAIS EST LA LANGUE SOURCE. `fr.ts` fait foi : c'est lui qui définit les clés
 * existantes. Les clés sont typées (`Cle`) et `en.ts` doit avoir exactement la même forme
 * (`Forme`) : une clé mal orthographiée ou une traduction manquante ne compile pas.
 * Le repli sur le français à l'exécution reste en filet — pour un catalogue chargé
 * autrement que par ce fichier, un jour —, mais il ne devrait jamais servir.
 */
import { computed, ref } from "vue";
import fr from "./fr";
import en from "./en";

export type Langue = "fr" | "en";

/** Les langues proposées, dans l'ordre d'affichage du sélecteur. */
export const LANGUES: { code: Langue; nom: string }[] = [
  { code: "fr", nom: "Français" },
  { code: "en", nom: "English" },
];

type Catalogue = Record<string, unknown>;
const CATALOGUES: Record<Langue, Catalogue> = { fr, en };

/**
 * La forme d'un catalogue : mêmes clés que `fr.ts`, n'importe quel texte aux feuilles.
 *
 * 🔑 C'EST CE TYPE QUI REND LES OUBLIS IMPOSSIBLES. `en.ts` est déclaré avec, donc une
 * clé française sans traduction anglaise — ou une clé anglaise qui n'existe pas en
 * français — est une **erreur de compilation**, pas un avertissement en console qu'on
 * ne verra que si l'on ouvre le bon écran dans la bonne langue.
 */
export type Forme<T> = { [K in keyof T]: T[K] extends string ? string : Forme<T[K]> };

/** Tous les chemins pointés menant à un texte : `"reglages.langue.titre"`, etc. */
type Chemins<T, P extends string = ""> = {
  [K in keyof T & string]: T[K] extends string ? `${P}${K}` : Chemins<T[K], `${P}${K}.`>;
}[keyof T & string];

/**
 * Une clé de traduction valide.
 *
 * 🔑 `t("reglages.langeu.titre")` ne compile pas. Sur un millier d'appels, c'est la
 * différence entre une faute de frappe attrapée par `vue-tsc` et une clé brute affichée
 * à un utilisateur.
 */
export type Cle = Chemins<typeof fr>;

// Aussitôt remplacée par `usePreferences.apply()`, au chargement du module.
const langue = ref<Langue>("en");

/** La langue courante, en lecture seule. Réactive : `t()` se recalcule quand elle change. */
export const langueCourante = computed(() => langue.value);

/**
 * L'étiquette BCP-47 correspondante, pour tout ce qui passe par `Intl` — prix, nombres,
 * dates. ⚠️ Pas `"en"` tout court : sans région, `Intl` choisit l'anglais américain pour
 * les nombres mais reste imprévisible sur les devises. On fixe la région par défaut.
 */
export const etiquetteIntl = computed(() => (langue.value === "fr" ? "fr-FR" : "en-US"));

/** Descend un chemin pointé (`"reglages.langue.titre"`) dans un catalogue. */
function resoudre(catalogue: Catalogue, cle: string): string | undefined {
  let noeud: unknown = catalogue;
  for (const segment of cle.split(".")) {
    if (typeof noeud !== "object" || noeud === null) return undefined;
    noeud = (noeud as Record<string, unknown>)[segment];
  }
  return typeof noeud === "string" ? noeud : undefined;
}

/**
 * Remplace `{nom}` par la valeur fournie.
 *
 * ⚠️ Les valeurs ne sont PAS échappées : elles sont interpolées dans du texte, que Vue
 * insère ensuite comme contenu textuel. Ne jamais passer le résultat à `v-html`.
 */
function interpoler(texte: string, params?: Record<string, string | number>): string {
  if (!params) return texte;
  return texte.replace(/\{(\w+)\}/g, (brut, nom: string) =>
    nom in params ? String(params[nom]) : brut,
  );
}

/**
 * Choisit entre singulier et pluriel, séparés par `|` dans le catalogue :
 * `"un jeu | {n} jeux"`. Le français et l'anglais partagent la même règle, d'où
 * l'absence de table de pluriels — la rouvrir sera le premier travail d'une 3ᵉ langue.
 */
function accorder(texte: string, n: number | undefined): string {
  if (n === undefined || !texte.includes("|")) return texte;
  const [singulier, pluriel] = texte.split("|");
  return (Math.abs(n) === 1 ? singulier : pluriel).trim();
}

/**
 * Traduit une clé.
 *
 * @param cle    chemin pointé, tel qu'il apparaît dans `fr.ts`
 * @param params valeurs à interpoler ; `n` déclenche en plus l'accord en nombre
 */
export function t(cle: Cle, params?: Record<string, string | number>): string {
  const brut =
    resoudre(CATALOGUES[langue.value], cle) ??
    resoudre(CATALOGUES.fr, cle) ??
    cle;

  if (import.meta.env.DEV && !resoudre(CATALOGUES[langue.value], cle)) {
    console.warn(`[i18n] clé manquante en « ${langue.value} » : ${cle}`);
  }

  const n = typeof params?.n === "number" ? params.n : undefined;
  return interpoler(accorder(brut, n), params);
}

/**
 * Applique une langue à l'interface.
 *
 * 🔑 `document.documentElement.lang` FAIT PARTIE DU TRAVAIL. Sans lui, un lecteur d'écran
 * lit toute l'interface avec la prononciation de la langue déclarée — le problème corrigé
 * en 0.19.0, qui reviendrait à l'identique si la déclaration restait figée sur `fr`.
 */
export function appliquerLangue(nouvelle: Langue) {
  langue.value = nouvelle;
  document.documentElement.lang = nouvelle;
  // Le titre d'`index.html` est écrit en français : sans ça, l'onglet de la démo en ligne
  // resterait « bibliothèque de jeux » au-dessus d'une interface anglaise.
  document.title = t("systeme.titrePage");
}

/**
 * La langue de Windows — pour le choix « Suivre Windows » des Paramètres, qui n'est plus le
 * défaut (voir `langueParDefaut`).
 *
 * ⚠️ On ne teste pas l'égalité avec `"fr"` : `navigator.language` vaut `fr-FR`, `fr-BE`,
 * `fr-CA`… Tout ce qui commence par `fr` est du français ; le reste bascule en anglais,
 * qui est le meilleur repli possible pour une langue qu'on ne parle pas.
 */
export function langueDuSysteme(): Langue {
  const brut = typeof navigator !== "undefined" ? navigator.language : "";
  return brut.toLowerCase().startsWith("fr") ? "fr" : "en";
}

/**
 * La langue de quelqu'un qui n'en a jamais choisi : **l'anglais**, quelle que soit la
 * langue de Windows.
 *
 * 🔑 POURQUOI PAS LA LANGUE DU SYSTÈME. Torii vise désormais un public international, et
 * l'anglais est la langue que le plus grand nombre lira. « Suivre Windows » reste proposé
 * dans les Paramètres, pour qui le veut.
 *
 * ⚠️ DEUX EXCEPTIONS, qui passent avant ce défaut :
 *   - la démo en ligne ouverte avec `?lang=fr|en` depuis une page du site : quelqu'un qui
 *     lit la page française doit arriver sur une démo française. Sans effet dans
 *     l'application installée, dont l'adresse ne porte jamais de paramètre ;
 *   - une installation antérieure aux langues, restée en français : ce cas-là est réglé
 *     par `usePreferences`, qui demande à Rust et épingle le français dans les préférences.
 */
export function langueParDefaut(): Langue {
  const demandee =
    typeof location !== "undefined" ? new URLSearchParams(location.search).get("lang") : null;
  if (demandee === "fr" || demandee === "en") return demandee;
  return "en";
}

/** Pour un composant : `const { t } = useI18n()`. */
export function useI18n() {
  return { t, langue: langueCourante, etiquetteIntl };
}
