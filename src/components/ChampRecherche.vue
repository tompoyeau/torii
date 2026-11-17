<script setup lang="ts">
import { ref } from "vue";
import { t } from "../i18n";

/**
 * Recherche locale d'un écran (Wishlist, En commun, bibliothèque d'un ami) : le dessin de
 * la barre du haut (`TopBar`), loupe à gauche, croix pour vider, à la hauteur des puces
 * d'une en-tête de section.
 *
 * 🔑 `type="text"` ET NOTRE CROIX : en `type="search"`, Chromium dessinait la sienne, d'un
 * autre style que celle du bandeau. `data-recherche` : c'est ce champ que vise Ctrl+F
 * (`useRaccourcis`).
 */
const valeur = defineModel<string>({ required: true });
defineProps<{ placeholder: string; libelle?: string }>();

const champ = ref<HTMLInputElement | null>(null);
/** Vide d'un geste et rend le focus au champ : sinon il resterait sur une croix disparue. */
function effacer() {
  valeur.value = "";
  champ.value?.focus();
}
</script>

<template>
  <label class="recherche">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3-3" /></svg>
    <input
      ref="champ"
      v-model="valeur"
      data-recherche
      type="text"
      autocomplete="off"
      :placeholder="placeholder"
      :aria-label="libelle ?? placeholder"
    />
    <button
      v-if="valeur"
      type="button"
      class="effacer"
      :title="t('bibliotheque.barre.effacer')"
      :aria-label="t('bibliotheque.barre.effacer')"
      @click="effacer"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </label>
</template>

<style scoped>
/* Mêmes couleurs, bordure et arrondi que la recherche du bandeau ; plus basse (34 px), pour
   s'aligner sur les puces d'une en-tête de section. */
.recherche { position: relative; display: flex; align-items: center; width: 240px; min-width: 0; flex: 0 1 240px; }
.recherche > svg { position: absolute; left: 12px; width: 15px; height: 15px; color: var(--text-faint); pointer-events: none; }
.recherche input {
  width: 100%; height: 34px; padding: 0 34px 0 34px; background: var(--surface);
  border: 1px solid var(--border); border-radius: 11px; color: var(--text);
  font-size: 13px; font-family: inherit;
}
.recherche input::placeholder { color: var(--text-faint); }
.recherche input:focus { border-color: var(--border-strong); background: var(--surface-2); }
.effacer {
  position: absolute; right: 5px;
  display: grid; place-items: center; width: 24px; height: 24px; border-radius: 7px;
  background: none; border: none; color: var(--text-faint); cursor: pointer;
}
.effacer svg { width: 14px; height: 14px; }
.effacer:hover { background: var(--surface-2); color: var(--text); }
</style>
