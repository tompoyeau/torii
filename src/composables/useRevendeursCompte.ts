import { watch } from "vue";
import { CLE_A_ENVOYER, CLE_COMPTE, memeListe, reconcilier, type EtatRevendeurs } from "../lib/revendeurs";
import { getExcludedStores, toriiMe, toriiSetHiddenStores } from "../lib/tauri";
import type { ToriiAccount } from "../types";
import { appliquerRevendeurs, surRevendeursModifies } from "./useStore";
import { useTorii } from "./useTorii";

/**
 * Les revendeurs masqués suivent le compte : le même choix sur tous les appareils. Sans compte,
 * rien ne change, la liste reste sur l'appareil.
 *
 * L'interface garde son fonctionnement : elle lit et écrit la liste locale, et ce module la tient
 * alignée sur le compte. Il passe par les mêmes commandes sur le PC et sur le web.
 */

/** On ne redemande pas le compte à chaque retour sur la fenêtre. */
const INTERVALLE_MS = 60_000;

function lireEtat(): EtatRevendeurs {
  try {
    return { compte: localStorage.getItem(CLE_COMPTE), aEnvoyer: localStorage.getItem(CLE_A_ENVOYER) === "1" };
  } catch {
    return { compte: null, aEnvoyer: false };
  }
}

function ecrireEtat(etat: EtatRevendeurs) {
  try {
    if (etat.compte) localStorage.setItem(CLE_COMPTE, etat.compte);
    else localStorage.removeItem(CLE_COMPTE);
    if (etat.aEnvoyer) localStorage.setItem(CLE_A_ENVOYER, "1");
    else localStorage.removeItem(CLE_A_ENVOYER);
  } catch {
    // Sans stockage, on refera une fusion à la prochaine ouverture : rien ne se perd.
  }
}

/** Monte à chaque geste de l'utilisateur : une réponse partie avant lui ne doit pas l'écraser. */
let gestes = 0;

async function envoyer(compteId: string, liste: string[]) {
  const auDepart = gestes;
  try {
    await toriiSetHiddenStores(liste);
    // Un autre changement est arrivé pendant l'envoi : c'est lui qui soldera le compte.
    if (auDepart === gestes) ecrireEtat({ compte: compteId, aEnvoyer: false });
  } catch {
    // Hors ligne : `aEnvoyer` est déjà posé, et le prochain passage réessaiera.
  }
}

async function synchroniser(compte: ToriiAccount) {
  // Un serveur qui ne connaît pas encore cette liste n'a rien à dire dessus.
  if (!Array.isArray(compte.hiddenStores)) return;
  const auDepart = gestes;
  const local = (await getExcludedStores()) ?? [];
  if (auDepart !== gestes) return;

  const { liste, envoyer: aEnvoyer } = reconcilier(local, compte.hiddenStores, compte.id, lireEtat());
  ecrireEtat({ compte: compte.id, aEnvoyer });
  if (!memeListe(liste, local)) await appliquerRevendeurs(liste);
  if (aEnvoyer) await envoyer(compte.id, liste);
}

/** À appeler une fois, au démarrage de l'interface. */
export function suivreRevendeursDuCompte() {
  const { account } = useTorii();

  watch(
    account,
    (compte, avant) => {
      if (compte) void synchroniser(compte);
      // Déconnecté : la liste reste sur l'appareil, et le prochain compte fusionnera avec elle.
      else if (avant) ecrireEtat({ compte: null, aEnvoyer: false });
    },
    { immediate: true },
  );

  surRevendeursModifies((liste) => {
    gestes++;
    const compte = account.value;
    if (!compte || !Array.isArray(compte.hiddenStores)) return;
    ecrireEtat({ compte: compte.id, aEnvoyer: true });
    void envoyer(compte.id, liste);
  });

  // Un changement fait sur le téléphone se voit quand on revient sur le PC, et inversement.
  let dernier = Date.now();
  const auRetour = async () => {
    if (document.visibilityState !== "visible" || !account.value) return;
    if (Date.now() - dernier < INTERVALLE_MS) return;
    dernier = Date.now();
    const compte = await toriiMe().catch(() => null);
    if (compte) await synchroniser(compte);
  };
  document.addEventListener("visibilitychange", auRetour);
  window.addEventListener("focus", auRetour);
}
