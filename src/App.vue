<script setup lang="ts">
import { useUi } from "./composables/useUi";
import BureauView from "./components/BureauView.vue";
import SalonView from "./components/SalonView.vue";
import GameDetail from "./components/GameDetail.vue";
import StoreGameDetail from "./components/StoreGameDetail.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import ContextMenu from "./components/ContextMenu.vue";
import AddGameModal from "./components/AddGameModal.vue";
import UpdateBanner from "./components/UpdateBanner.vue";

const { mode } = useUi();
</script>

<template>
  <div class="app">
    <BureauView v-if="mode === 'bureau'" />
    <SalonView v-else />
    <GameDetail />
    <StoreGameDetail />
    <SettingsPanel />
    <ContextMenu />
    <AddGameModal />
    <UpdateBanner />
  </div>
</template>
