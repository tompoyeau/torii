<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import { useUi } from "./composables/useUi";
import { useLibrary } from "./composables/useLibrary";
import { useNavigation } from "./composables/useNavigation";
import { installerRaccourcis } from "./composables/useRaccourcis";
import { onGameDetected, onGameExited, onGameLaunched, onLibraryUpdated } from "./lib/tauri";
import { startWishlistNotifier } from "./composables/useWishlistNotifier";
import { useTorii } from "./composables/useTorii";
import { MODE_WEB } from "./lib/mode";
import AppShell from "./components/AppShell.vue";
import BarreMobile from "./components/BarreMobile.vue";
import EcranConnexion from "./components/EcranConnexion.vue";
import GameDetail from "./components/GameDetail.vue";
import StoreGameDetail from "./components/StoreGameDetail.vue";
import SettingsView from "./components/SettingsView.vue";
import ContextMenu from "./components/ContextMenu.vue";
import AddGameModal from "./components/AddGameModal.vue";
import ToriiSignInDialog from "./components/ToriiSignInDialog.vue";
import UpdateBanner from "./components/UpdateBanner.vue";
import Toast from "./components/Toast.vue";
import SplashScreen from "./components/SplashScreen.vue";
import DemoBanner from "./components/DemoBanner.vue";

const { openGame } = useUi();
const { notePlayed, noteDetected, applyUpdate } = useLibrary();
const { reculer, avancer } = useNavigation();

/**
 * Sur le web, tout est derrière la connexion : le quota du comparateur est partagé avec
 * l'application. `booted` : la session a été vérifiée. Sans cette attente, un visiteur déjà
 * connecté verrait passer l'écran de connexion à chaque ouverture.
 */
const { connected, booted: compteVerifie } = useTorii();

// Suivi de session : à la fermeture d'un jeu, on ouvre sa fiche (la fenêtre a déjà
// été restaurée au premier plan côté Rust).
let unlistenGameExit: (() => void) | null = null;
let unlistenGameLaunch: (() => void) | null = null;
let unlistenGameDetected: (() => void) | null = null;
let unlistenLibrary: (() => void) | null = null;
let retirerRaccourcis: (() => void) | null = null;

/**
 * Supprime le menu contextuel natif de la WebView (« Inspecter », « Enregistrer sous »,
 * « Imprimer »…). Il n'a aucun sens dans une application, et il trahit qu'on regarde
 * une page web — d'autant que Torii a son propre clic droit sur les jeux.
 *
 * 🔑 Exception : les champs de saisie. Y couper le menu retirerait « Coller », que tout
 * le monde attend sur un champ texte, et qu'aucun menu maison ne remplace ici.
 */
function onContextMenu(e: MouseEvent) {
  if (MODE_WEB) return; // Dans un navigateur, c'est le menu qu'on attend.
  const cible = e.target as HTMLElement | null;
  if (cible?.closest("input, textarea, [contenteditable='true']")) return;
  e.preventDefault();
}

// Les boutons « retour » et « avant » de la souris = MouseEvent.button 3 et 4. On
// empêche la webview de naviguer dans son propre historique (mousedown) et on pilote
// l'historique de l'application (`useNavigation`) sur le relâchement (mouseup).
// Sur le web, on laisse faire le navigateur : `useNavigation` suit son historique.
function onMouseDown(e: MouseEvent) {
  if (MODE_WEB) return;
  if (e.button === 3 || e.button === 4) e.preventDefault();
}
function onMouseUp(e: MouseEvent) {
  if (MODE_WEB || (e.button !== 3 && e.button !== 4)) return;
  e.preventDefault();
  if (e.button === 3) reculer();
  else avancer();
}
onMounted(async () => {
  window.addEventListener("mousedown", onMouseDown);
  window.addEventListener("mouseup", onMouseUp);
  window.addEventListener("contextmenu", onContextMenu);
  retirerRaccourcis = installerRaccourcis();
  unlistenGameExit = await onGameExited((id) => openGame(id));
  // Une partie détectée (même lancée depuis Steam ou le bureau) remonte aussitôt dans
  // « Récemment joué », sans attendre une resynchronisation.
  unlistenGameLaunch = await onGameLaunched((id, at) => notePlayed(id, at));
  // Un jeu repéré hors launcher (Genshin, Dofus, Game Pass…) entre dans la
  // bibliothèque à chaud : sans sa carte, « Récemment joué » n'aurait rien à dater.
  unlistenGameDetected = await onGameDetected((game) => noteDetected(game));
  // Jeu installé, désinstallé ou acheté : la bibliothèque suit sans resynchronisation.
  unlistenLibrary = await onLibraryUpdated((list) => applyUpdate(list));
  if (!MODE_WEB) startWishlistNotifier();
});
onBeforeUnmount(() => {
  window.removeEventListener("mousedown", onMouseDown);
  window.removeEventListener("mouseup", onMouseUp);
  window.removeEventListener("contextmenu", onContextMenu);
  if (retirerRaccourcis) retirerRaccourcis();
  if (unlistenGameExit) unlistenGameExit();
  if (unlistenGameLaunch) unlistenGameLaunch();
  if (unlistenGameDetected) unlistenGameDetected();
  if (unlistenLibrary) unlistenLibrary();
});
</script>

<template>
  <div class="app">
    <template v-if="!MODE_WEB || connected">
      <AppShell />
      <GameDetail />
      <StoreGameDetail />
      <SettingsView />
      <ContextMenu />
      <AddGameModal />
      <UpdateBanner />
      <BarreMobile v-if="MODE_WEB" />
    </template>
    <EcranConnexion v-else-if="compteVerifie" />
    <ToriiSignInDialog />
    <Toast />
    <SplashScreen v-if="!MODE_WEB || connected || !compteVerifie" />
    <DemoBanner />
  </div>
</template>
