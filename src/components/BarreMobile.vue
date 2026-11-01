<script setup lang="ts">
import { computed } from "vue";
import { useUi } from "../composables/useUi";
import { useNavigation } from "../composables/useNavigation";
import { t } from "../i18n";

/**
 * La navigation sur téléphone : la barre latérale disparaît sous 820 px. Comme elle, cette
 * barre reste à l'écran quand une fiche est ouverte, et aller ailleurs referme la fiche en
 * une seule étape : le retour y ramène.
 */
const ui = useUi();
const { section, settingsOpen } = ui;
const { quitterLesSurcouches, coucheDuDessus, fermer } = useNavigation();

function aller(cible: "store" | "library") {
  // Déjà sur cet écran : on remonte en haut, comme dans toute application de téléphone.
  if (section.value === cible && !coucheDuDessus()) {
    window.scrollTo({ top: 0, behavior: "smooth" });
    return;
  }
  quitterLesSurcouches();
  if (cible === "store") ui.showStore();
  else ui.showLibrary();
}

function basculerParametres() {
  if (settingsOpen.value) fermer("parametres");
  else ui.openSettings();
}

const surBoutique = computed(() => section.value === "store" && !settingsOpen.value);
const surBibliotheque = computed(() => section.value === "library" && !settingsOpen.value);
</script>

<template>
  <nav class="barre-mobile" :aria-label="t('web.navigation')">
    <button class="onglet" :class="{ on: surBoutique }" :aria-current="surBoutique ? 'page' : undefined" @click="aller('store')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 8h16l-1 4a3 3 0 0 1-3 2.4H8A3 3 0 0 1 5 12Z" /><path d="M4 8l1.4-3.4A2 2 0 0 1 7.2 3.4h9.6a2 2 0 0 1 1.8 1.2L20 8" /><path d="M6 14.4V20a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1v-5.6" /></svg>
      {{ t("bibliotheque.laterale.boutique") }}
    </button>
    <button class="onglet" :class="{ on: surBibliotheque }" :aria-current="surBibliotheque ? 'page' : undefined" @click="aller('library')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" /></svg>
      {{ t("bibliotheque.laterale.bibliotheque") }}
    </button>
    <button class="onglet" :class="{ on: settingsOpen }" :aria-current="settingsOpen ? 'page' : undefined" @click="basculerParametres">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="3.1" /><path d="M19.4 13a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5v.2a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1.1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H2a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1.1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H8a1.6 1.6 0 0 0 1-1.5V2a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V8a1.6 1.6 0 0 0 1.5 1h.2a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" /></svg>
      {{ t("bibliotheque.barre.parametres") }}
    </button>
  </nav>
</template>

<style scoped>
.barre-mobile { display: none; }
.onglet {
  flex: 1; min-width: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px;
  background: none; border: none; color: var(--text-faint); font-size: 11px; font-weight: 600;
}
.onglet svg { width: 22px; height: 22px; }
.onglet.on { color: var(--accent); }

/* Au-dessus des fiches (110) et des Paramètres (200), sous la connexion (260) et les messages (300). */
@media (max-width: 820px) {
  .barre-mobile {
    position: fixed; left: 0; right: 0; bottom: 0; z-index: 250; display: flex;
    height: var(--barre-mobile);
    padding: 0 env(safe-area-inset-right, 0px) env(safe-area-inset-bottom, 0px) env(safe-area-inset-left, 0px);
    background: var(--surface); border-top: 1px solid var(--border);
  }
}
</style>
