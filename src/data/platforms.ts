import type { LibraryFilter, Platform, PlatformId } from "../types";
import { t } from "../i18n";

export const PLATFORMS: Record<PlatformId, Platform> = {
  steam: { id: "steam", name: "Steam", color: "var(--steam)" },
  epic: { id: "epic", name: "Epic Games", color: "var(--epic)" },
  gog: { id: "gog", name: "GOG", color: "var(--gog)" },
  riot: { id: "riot", name: "Riot Games", color: "var(--riot)" },
  ubisoft: { id: "ubisoft", name: "Ubisoft Connect", color: "var(--ubisoft)" },
  ea: { id: "ea", name: "EA", color: "var(--ea)" },
  battlenet: { id: "battlenet", name: "Battle.net", color: "var(--battlenet)" },
  // 🔑 `manual` et `detected` portent le MÊME nom et la même couleur : ce sont deux
  // façons d'arriver au même endroit — un jeu qu'aucun launcher ne fournit. L'un a été
  // ajouté à la main, l'autre repéré tout seul, mais côté bibliothèque c'est une seule
  // catégorie (cf. `HORS_LAUNCHER`). Les deux identifiants restent distincts en base :
  // ils décident de qui sait éditer et supprimer le jeu, et un id de jeu détecté ne
  // change JAMAIS (favoris, historique et présence s'y accrochent).
  manual: { id: "manual", name: "Hors launcher", color: "var(--manual)" },
  detected: { id: "detected", name: "Hors launcher", color: "var(--manual)" },
};

/**
 * Les plateformes qui ne viennent d'aucun launcher. Une seule définition : la catégorie
 * de la barre latérale, son compteur, le filtre de la grille et les menus qui proposent
 * « Modifier » ou « Retirer » s'y réfèrent tous. En lister deux fois les membres, c'est
 * s'exposer à ce qu'un jeu apparaisse dans la catégorie sans y être éditable.
 */
export const HORS_LAUNCHER: PlatformId[] = ["manual", "detected"];

/** Ce jeu vient-il d'ailleurs que d'un launcher (ajouté à la main ou repéré tout seul) ? */
export function estHorsLauncher(platform: string | undefined): boolean {
  return HORS_LAUNCHER.includes(platform as PlatformId);
}

/**
 * Le nom d'une plateforme, tel qu'affiché.
 *
 * ⚠️ « Hors launcher » est le seul qui se traduit : les autres sont des marques. Il est
 * lu ici, à chaque appel, et pas figé dans `PLATFORMS` — une table calculée au
 * chargement garderait la langue du démarrage.
 */
export function platformName(id: PlatformId): string {
  return estHorsLauncher(id) ? t("bibliotheque.horsLauncher") : PLATFORMS[id].name;
}

/** Les filtres qui ne sont pas une plateforme ; les autres prennent le nom de la plateforme. */
const FILTRES_NOMMES = ["all", "mine", "family", "recent", "favorite", "installed", "hidden", "horsLauncher"] as const;
type FiltreNomme = (typeof FILTRES_NOMMES)[number];

/**
 * Le titre d'une vue de la bibliothèque (« Tous les jeux », « Favoris », « Steam »…).
 * Partagé par l'en-tête de la grille et l'historique de navigation, qui nomme l'étape où
 * mène le bouton retour.
 */
export function titreFiltre(f: LibraryFilter): string {
  return (FILTRES_NOMMES as readonly string[]).includes(f)
    ? t(`bibliotheque.grille.filtres.${f as FiltreNomme}`)
    : platformName(f as PlatformId);
}
