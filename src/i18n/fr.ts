/**
 * Catalogue français — **langue source**.
 *
 * 🔑 CE CATALOGUE FAIT FOI. Ses clés sont les seules qui existent : `t()` n'accepte
 * qu'elles (type `Cle`), et `en.ts` doit reproduire exactement sa forme, faute de quoi
 * la compilation échoue.
 *
 * Il est découpé par zone de l'application (`fr/*.ts`), chacune doublée d'une jumelle
 * anglaise (`en/*.ts`) typée contre elle. Un fichier de mille cinq cents lignes où l'on
 * cherche la bonne section, c'est l'assurance d'ajouter la même clé deux fois.
 *
 * CONVENTION DE NOMMAGE : `zone.ecran.element`. On nomme par l'**endroit**, pas par le
 * texte — `commun.annuler`, jamais `commun.annuler_la_suppression`. Une clé qui décrit
 * son propre contenu devient un mensonge dès que le texte est reformulé.
 *
 * ⚠️ PAS DE PHRASE RECONSTITUÉE PAR CONCATÉNATION. `"Il y a " + n + " jeux"` place le
 * verbe et le nombre dans un ordre que toutes les langues ne partagent pas ; on écrit
 * `"{n} jeux"` et chaque catalogue place ses mots.
 *
 * ⚠️ PLURIELS : `"un jeu | {n} jeux"`, et l'on passe `n` à `t()`. Le français et
 * l'anglais partagent la même règle (un / plusieurs) — voir `accorder()` dans `index.ts`.
 */
import amis from "./fr/amis";
import bibliotheque from "./fr/bibliotheque";
import boutique from "./fr/boutique";
import commun from "./fr/commun";
import demo from "./fr/demo";
import comptes from "./fr/comptes";
import fiche from "./fr/fiche";
import prix from "./fr/prix";
import reglages from "./fr/reglages";
import systeme from "./fr/systeme";

export default {
  amis,
  bibliotheque,
  boutique,
  commun,
  comptes,
  demo,
  fiche,
  prix,
  reglages,
  systeme,
} as const;
