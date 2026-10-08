/**
 * Statistiques d'usage : Umami auto-hébergé (stats.topo-host.com), sans cookie.
 *
 * 🔑 Pour l'instant, seule la DÉMO WEB (torii-app.fr/demo/) est mesurée, dans la même
 * fiche que le site (« Torii · site + démo »). L'application installée n'envoie RIEN :
 * `hasTauriRuntime()` coupe court. Si elle doit un jour mesurer son usage, ce sera avec
 * un interrupteur dans les paramètres et une mention sur le site, pas en douce.
 *
 * Les échecs sont silencieux : un bloqueur de pub ou un réseau coupé ne doit rien casser.
 */
import { watch } from "vue";
import { hasTauriRuntime } from "./tauri";
import { useUi } from "../composables/useUi";

const SCRIPT = "https://stats.topo-host.com/t.js";
const SITE_ET_DEMO = "f6c0d016-a53f-48a3-9e1c-0bdc24280cdc";

declare global {
  interface Window { umami?: { track: (event: string, data?: Record<string, unknown>) => void } }
}

let actif = false;

export function demarrerStats(): void {
  if (hasTauriRuntime()) return;
  if (location.hostname !== "torii-app.fr" && location.hostname !== "www.torii-app.fr") return;
  actif = true;
  const s = document.createElement("script");
  s.defer = true;
  s.src = SCRIPT;
  s.dataset.websiteId = SITE_ET_DEMO;
  document.head.appendChild(s);
  suivreInterface();
  // Le bandeau de la démo renvoie vers le téléchargement : c'est la conversion qui compte.
  document.addEventListener("click", (e) => {
    if ((e.target as Element).closest?.('a[href*="#telecharger"]')) stat("Téléchargement depuis la démo");
  }, true);
}

const SECTIONS: Record<string, string> = {
  library: "Bibliothèque", store: "Boutique", wishlist: "Wishlist", friends: "Amis",
  common: "Jeux en commun", friendProfile: "Profil d'un ami", friendLibrary: "Bibliothèque d'un ami",
};

/**
 * La navigation de Torii ne change pas l'URL : on regarde l'état de l'interface plutôt
 * que d'ajouter un appel dans chaque action. Seuls des libellés partent, jamais un titre
 * de jeu ni un nom d'ami.
 */
function suivreInterface(): void {
  const ui = useUi();
  watch(ui.section, (s) => stat("Section", { section: SECTIONS[s] ?? s }));
  watch(ui.selectedGameId, (id) => { if (id) stat("Fiche jeu ouverte"); });
  watch(ui.filter, (f) => stat("Filtre", { filtre: f }));
  watch(ui.sort, (t) => stat("Tri", { tri: t }));
  watch(ui.listView, (liste) => stat("Affichage", { mode: liste ? "liste" : "grille" }));
  watch(ui.settingsOpen, (ouvert) => { if (ouvert) stat("Paramètres ouverts", { onglet: ui.settingsCategory.value }); });
  watch(ui.addGameOpen, (ouvert) => { if (ouvert) stat("Ajout de jeu manuel ouvert"); });
}

/** Compte une action (ex. « Fiche jeu ouverte »). Sans effet hors de la démo en ligne. */
export function stat(evenement: string, donnees?: Record<string, unknown>): void {
  if (!actif) return;
  try { window.umami?.track(evenement, donnees); } catch { /* tant pis */ }
}
