<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hasTauriRuntime } from "../lib/tauri";
import { MODE_WEB } from "../lib/mode";
import { t } from "../i18n";

/**
 * Réduire, agrandir, fermer : les commandes de la fenêtre, dessinées par Torii. La barre de
 * titre de Windows est retirée (`decorations: false`, tauri.conf.json) ; la barre du haut de
 * l'appli en tient lieu, et se déplace à la souris (`data-tauri-drag-region`, `TopBar.vue`).
 *
 * 🔑 À LA PLACE EXACTE DE WINDOWS : collées au coin haut droit, 46 × 32 px chacune, avec les
 * glyphes de Windows (police Segoe Fluent Icons, Segoe MDL2 Assets sous Windows 10) et ses
 * survols. La main va les chercher là sans regarder ; seul le fond change : transparent, il
 * laisse voir la barre de Torii (son verre quand le contenu défile dessous).
 *
 * Au-dessus de tout : les Paramètres, les fenêtres et l'écran de démarrage recouvrent la
 * barre du haut, et on doit pouvoir réduire Torii pendant qu'un réglage est ouvert.
 *
 * « Fermer » demande la fermeture comme le faisait la croix de Windows : la règle « fermer =
 * réduire dans la zone de notification » (lib.rs, `CloseRequested`) s'applique telle quelle.
 */
const actives = hasTauriRuntime() && !MODE_WEB;
const agrandie = ref(false);
let arreter: (() => void) | null = null;

onMounted(async () => {
  if (!actives) return;
  // La barre du haut et les visionneuses leur laissent la place (`style.css`, `TopBar.vue`).
  document.documentElement.classList.add("fenetre-perso");
  const fenetre = getCurrentWindow();
  agrandie.value = await fenetre.isMaximized();
  arreter = await fenetre.onResized(async () => {
    agrandie.value = await fenetre.isMaximized();
  });
});
onBeforeUnmount(() => {
  arreter?.();
  document.documentElement.classList.remove("fenetre-perso");
});

const reduire = () => void getCurrentWindow().minimize();
const basculer = () => void getCurrentWindow().toggleMaximize();
const fermer = () => void getCurrentWindow().close();
</script>

<template>
  <div v-if="actives" class="controles" role="group" :aria-label="t('systeme.fenetre.titre')">
    <!-- Glyphes Windows : ChromeMinimize, ChromeMaximize, ChromeRestore, ChromeClose. -->
    <button :title="t('systeme.fenetre.reduire')" :aria-label="t('systeme.fenetre.reduire')" @click="reduire">
      <span aria-hidden="true">&#xE921;</span>
    </button>
    <button
      :title="agrandie ? t('systeme.fenetre.restaurer') : t('systeme.fenetre.agrandir')"
      :aria-label="agrandie ? t('systeme.fenetre.restaurer') : t('systeme.fenetre.agrandir')"
      @click="basculer"
    >
      <span aria-hidden="true">{{ agrandie ? "" : "" }}</span>
    </button>
    <button class="fermer" :title="t('systeme.fenetre.fermer')" :aria-label="t('systeme.fenetre.fermer')" @click="fermer">
      <span aria-hidden="true">&#xE8BB;</span>
    </button>
  </div>
</template>

<style scoped>
/* Au-dessus de tout, écran de démarrage compris (400). */
.controles { position: fixed; top: 0; right: 0; z-index: 450; display: flex; }
.controles button {
  /* Toute la hauteur de la barre du haut (`--hauteur-titre`, style.css). */
  display: grid; place-items: center; width: 46px; height: var(--hauteur-titre, 32px); padding: 0;
  border: none; border-radius: 0; background: transparent; color: var(--text); cursor: default;
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets"; font-size: 10px; line-height: 1;
  /* Lisibles aussi sur la bannière d'une fiche, où la barre n'a pas de fond. */
  text-shadow: 0 0 6px rgba(0, 0, 0, 0.45);
}
/* Les survols de Windows 11 : un voile léger, et le rouge pour fermer. */
.controles button:hover { background: rgba(255, 255, 255, 0.0837); }
.controles button:active { background: rgba(255, 255, 255, 0.0544); color: var(--text-dim); }
:root[data-theme="light"] .controles button { text-shadow: none; }
:root[data-theme="light"] .controles button:hover { background: rgba(0, 0, 0, 0.0373); }
:root[data-theme="light"] .controles button:active { background: rgba(0, 0, 0, 0.0241); }
.controles button.fermer:hover { background: #c42b1c; color: #fff; text-shadow: none; }
.controles button.fermer:active { background: #c84031; color: rgba(255, 255, 255, 0.7); }
</style>
