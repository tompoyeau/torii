import type { ToriiStatus } from "../types";
import { t } from "../i18n";
import type { UnifiedFriend } from "../composables/useFriendList";

/**
 * Comment se dit la présence d'un ami — **une phrase par canal**, pas un état unique.
 *
 * 🔑 Vit ici et non dans un composant : la vue Amis et la page profil disent la même
 * chose de la même personne, et une divergence entre les deux se lirait comme une
 * contradiction (« en ligne » d'un côté, « Torii fermé » de l'autre).
 *
 * L'agrégat (`UnifiedFriend.state`) sert au classement et efface l'essentiel : quelqu'un
 * d'ami des deux côtés, en ligne sur Steam avec Torii fermé, s'affichait « en ligne »
 * sans qu'on puisse savoir que Torii ne voit rien de ce qu'il joue.
 */
export interface CanalPresence {
  key: "steam" | "torii";
  label: string;
  /** Connecté sur ce canal en ce moment. Éteint = relation existante, mais absent. */
  live: boolean;
  title: string;
}

/**
 * ⚠️ Une phrase entière par cas, et pas « En ligne » + « sur Steam » : l'ordre et la
 * préposition changent d'une langue à l'autre, et certaines phrases (« Torii fermé »)
 * n'ont aucune forme commune avec les autres.
 */
function etatSource(canal: "steam" | "torii", etat: ToriiStatus | null) {
  const steam = canal === "steam";
  switch (etat) {
    case "in-game":
      return { live: true, phrase: steam ? t("amis.canaux.enJeuSteam") : t("amis.canaux.enJeuTorii") };
    case "online":
      return { live: true, phrase: steam ? t("amis.canaux.enLigneSteam") : t("amis.canaux.enLigneTorii") };
    case "away":
      return { live: true, phrase: steam ? t("amis.canaux.absentSteam") : t("amis.canaux.absentTorii") };
    default:
      return {
        live: false,
        phrase: steam ? t("amis.canaux.horsLigneSteam") : t("amis.canaux.toriiFerme"),
      };
  }
}

/** Les canaux par lesquels on connaît cette personne, avec leur état courant. */
export function sourcesOf(f: UnifiedFriend): CanalPresence[] {
  const liste: CanalPresence[] = [];
  if (f.toriiState !== null) {
    const e = etatSource("torii", f.toriiState);
    liste.push({
      key: "torii",
      label: "Torii",
      live: e.live,
      title: t("amis.canaux.detailTorii", { etat: e.phrase }),
    });
  }
  if (f.steamState !== null) {
    const e = etatSource("steam", f.steamState);
    liste.push({
      key: "steam",
      label: "Steam",
      live: e.live,
      title: t("amis.canaux.detailSteam", { etat: e.phrase }),
    });
  }
  return liste;
}

/**
 * Pourquoi on ne voit rien de cette personne. « Hors ligne » veut dire « Torii fermé »
 * côté Torii, et non « ne joue pas » — la nuance est celle qu'on a promise à
 * l'utilisateur, elle doit se dire à chaque fois.
 */
export function offlineHintOf(f: UnifiedFriend): string {
  if (f.source === "both") {
    return t("amis.canaux.pourquoiLesDeux");
  }
  return f.source === "torii" ? t("amis.canaux.pourquoiTorii") : t("amis.canaux.pourquoiSteam");
}
