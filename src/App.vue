<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import { useUi } from "./composables/useUi";
import { useStore } from "./composables/useStore";
import { onGameExited } from "./lib/tauri";
import { startWishlistNotifier } from "./composables/useWishlistNotifier";
import BureauView from "./components/BureauView.vue";
import SalonView from "./components/SalonView.vue";
import GameDetail from "./components/GameDetail.vue";
import StoreGameDetail from "./components/StoreGameDetail.vue";
import SettingsView from "./components/SettingsView.vue";
import ContextMenu from "./components/ContextMenu.vue";
import AddGameModal from "./components/AddGameModal.vue";
import UpdateBanner from "./components/UpdateBanner.vue";
import Toast from "./components/Toast.vue";
import SplashScreen from "./components/SplashScreen.vue";

const { mode, addGameOpen, closeAddGame, goBack, openGame } = useUi();
const { selectedGameId: storeProductId, closeProduct } = useStore();

// Suivi de session : à la fermeture d'un jeu, on ouvre sa fiche (la fenêtre a déjà
// été restaurée au premier plan côté Rust).
let unlistenGameExit: (() => void) | null = null;

/**
 * Navigation « précédent » : ferme d'abord les surcouches ouvertes (fiche produit
 * boutique, modale d'ajout), sinon dépile l'historique de navigation (fiche jeu,
 * pop-in Paramètres, sections). Déclenché par le bouton « retour » de la souris.
 */
function navigateBack() {
  if (storeProductId.value != null) {
    closeProduct();
    return;
  }
  if (addGameOpen.value) {
    closeAddGame();
    return;
  }
  goBack();
}

// Le bouton « retour » de la souris = MouseEvent.button === 3 (le 4 = « suivant »).
// On empêche la webview de naviguer dans son propre historique (mousedown) et on
// pilote notre navigation applicative sur le relâchement (mouseup).
function onMouseDown(e: MouseEvent) {
  if (e.button === 3 || e.button === 4) e.preventDefault();
}
function onMouseUp(e: MouseEvent) {
  if (e.button === 3) {
    e.preventDefault();
    navigateBack();
  }
}
onMounted(async () => {
  window.addEventListener("mousedown", onMouseDown);
  window.addEventListener("mouseup", onMouseUp);
  unlistenGameExit = await onGameExited((id) => openGame(id));
  startWishlistNotifier();
});
onBeforeUnmount(() => {
  window.removeEventListener("mousedown", onMouseDown);
  window.removeEventListener("mouseup", onMouseUp);
  if (unlistenGameExit) unlistenGameExit();
});
</script>

<template>
  <div class="app">
    <BureauView v-if="mode === 'bureau'" />
    <SalonView v-else />
    <GameDetail />
    <StoreGameDetail />
    <SettingsView />
    <ContextMenu />
    <AddGameModal />
    <UpdateBanner />
    <Toast />
    <SplashScreen />
  </div>
</template>
