<script setup lang="ts">
import type { Possession } from "../composables/usePossession";
import { t } from "../i18n";

/** Pastille « Dans ta bibliothèque » / « Partage familial ». */
defineProps<{ etat: Exclude<Possession, null> }>();
</script>

<template>
  <span class="possede">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
    {{ etat === "famille" ? t("boutique.accueil.carte.famille") : t("boutique.accueil.carte.possede") }}
  </span>
</template>
