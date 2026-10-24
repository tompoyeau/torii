import { reactive } from "vue";
import { hasTauriRuntime, onPageWeb, openExternal, pageAction, type EtatPage } from "../lib/tauri";
import { brancherHistoriqueInterne, noterTitrePage, useNavigation } from "./useNavigation";
import { useUi } from "./useUi";

/**
 * La page web intégrée, côté interface : ce qu'elle affiche (adresse, titre, chargement,
 * historique), et le seul point d'entrée pour en ouvrir une, `ouvrirPage`.
 *
 * 🔑 UN SEUL HISTORIQUE POUR L'UTILISATEUR. Sur une page, « précédent » (souris, Alt+←,
 * flèche de la barre) recule d'abord DANS la page, comme un navigateur, puis dans Torii
 * quand la page est revenue à son début. Deux jeux de flèches, l'un pour le site et
 * l'autre pour Torii, obligeaient à savoir lequel des deux historiques on voulait.
 */

const ui = useUi();
const nav = useNavigation();

const etat = reactive<EtatPage>({ url: "", titre: "", chargement: false, peutReculer: false, peutAvancer: false });

brancherHistoriqueInterne({
  reculer: () => {
    if (!etat.peutReculer) return false;
    void pageAction("precedent");
    return true;
  },
  avancer: () => {
    if (!etat.peutAvancer) return false;
    void pageAction("suivant");
    return true;
  },
});

let ecoute: Promise<() => void> | null = null;
function ecouter() {
  ecoute ??= onPageWeb((e) => {
    Object.assign(etat, e);
    noterTitrePage(e.url, e.titre);
    // La page a suivi un lien : l'étape en cours retient sa nouvelle adresse, pour que
    // revenir à cette étape rouvre la page là où on l'avait laissée.
    if (e.url && ui.section.value === "web" && e.url !== ui.pageWeb.value) ui.pageWeb.value = e.url;
  });
}

/**
 * Ouvre une adresse dans Torii (page web intégrée), en refermant d'abord fiches et
 * Paramètres : c'est une étape de l'historique, et le retour ramène sur la fiche d'où
 * l'on venait. Hors de l'application (démo web), un nouvel onglet du navigateur.
 *
 * ⚠️ RÉSERVÉ AUX ADRESSES QUE TORII FOURNIT LUI-MÊME : pages des launchers, liens
 * d'achat du comparateur. La page partage les sessions de Torii (Steam, Epic…) : une
 * adresse venue d'un ami, d'un pseudo ou d'un message n'a rien à y faire.
 */
export function ouvrirPage(url: string) {
  // Hors de l'application, ou adresse sans HTTPS (que la page refuserait) : navigateur.
  if (!hasTauriRuntime() || !url.startsWith("https://")) {
    void openExternal(url);
    return;
  }
  nav.quitterLesSurcouches();
  ui.showWeb(url);
}

export function usePageWeb() {
  ecouter();
  return { etat, ouvrirPage };
}
