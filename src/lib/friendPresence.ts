import type { ToriiStatus } from "../types";
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

function etatSource(canal: "steam" | "torii", etat: ToriiStatus | null) {
  const ou = canal === "steam" ? "sur Steam" : "sur Torii";
  switch (etat) {
    case "in-game":
      return { live: true, phrase: `En jeu, vu ${ou}` };
    case "online":
      return { live: true, phrase: `En ligne ${ou}` };
    case "away":
      return { live: true, phrase: `Absent ${ou}` };
    default:
      return {
        live: false,
        phrase:
          canal === "steam"
            ? "Hors ligne sur Steam"
            : "Torii fermé : ce qu'il joue hors Steam reste invisible",
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
      title: `${e.phrase}. Ami Torii : tu vois ses jeux quel que soit son launcher, tant qu'il a Torii ouvert.`,
    });
  }
  if (f.steamState !== null) {
    const e = etatSource("steam", f.steamState);
    liste.push({
      key: "steam",
      label: "Steam",
      live: e.live,
      title: `${e.phrase}. Ami Steam : cette liste vient de Steam et se gère depuis Steam.`,
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
    return "Hors ligne sur Steam et Torii fermé : elle joue peut-être sans qu'on le voie.";
  }
  return f.source === "torii"
    ? "Cette personne n'a pas Torii ouvert : elle joue peut-être sans qu'on le voie."
    : "Hors ligne sur Steam.";
}
