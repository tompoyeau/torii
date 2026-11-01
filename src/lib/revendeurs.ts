/**
 * Les revendeurs masqués d'un appareil face à ceux de son compte. Sans dépendance : c'est ici
 * que se joue la promesse « le même choix partout », et `tests/revendeurs.test.mjs` la vérifie.
 */

/** Ce que cet appareil retient de son dernier échange avec le compte. */
export interface EtatRevendeurs {
  /** Le compte dont la liste a déjà été fusionnée ici. */
  compte: string | null;
  /** Un changement fait ici n'a pas encore atteint le serveur. */
  aEnvoyer: boolean;
}

export const CLE_COMPTE = "torii.revendeurs.compte";
export const CLE_A_ENVOYER = "torii.revendeurs.aEnvoyer";

const trier = (liste: string[]) => [...new Set(liste)].sort();

export const memeListe = (a: string[], b: string[]) => trier(a).join("\u001f") === trier(b).join("\u001f");

/**
 * La liste à garder, et s'il faut l'envoyer.
 *
 * 🔑 Trois cas, et seul le premier additionne. La première fois qu'un appareil rencontre un
 * compte, chacun apporte sa liste : rien de ce qu'on avait masqué ne doit se perdre. Ensuite le
 * compte a raison, sinon un revendeur réaffiché ailleurs reviendrait masqué à chaque
 * synchronisation. Sauf si un changement fait ici n'est pas encore parti : c'est lui le plus récent.
 */
export function reconcilier(
  local: string[],
  duCompte: string[],
  compteId: string,
  etat: EtatRevendeurs,
): { liste: string[]; envoyer: boolean } {
  if (etat.compte !== compteId) {
    const liste = trier([...local, ...duCompte]);
    return { liste, envoyer: !memeListe(liste, duCompte) };
  }
  if (etat.aEnvoyer) return { liste: trier(local), envoyer: true };
  return { liste: trier(duCompte), envoyer: false };
}
