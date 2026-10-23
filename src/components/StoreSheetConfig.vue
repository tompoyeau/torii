<script setup lang="ts">
import { t } from "../i18n";
import type { StoreSteamSheet } from "../types";

/** Configuration requise, minimale et recommandée, telle que Steam la donne. */
defineProps<{ configuration: NonNullable<StoreSteamSheet["configuration"]> }>();
</script>

<template>
  <section>
    <h4>{{ t("boutique.produit.configuration") }}</h4>
    <div class="config">
      <div v-for="col in (['minimale', 'recommandee'] as const)" :key="col" class="config-col">
        <template v-if="configuration[col].length">
          <span class="config-titre">{{ t(`boutique.produit.${col}`) }}</span>
          <div v-for="l in configuration[col]" :key="l.libelle" class="config-ligne">
            <span class="config-libelle">{{ l.libelle }}</span>
            <span>{{ l.valeur }}</span>
          </div>
        </template>
      </div>
    </div>
    <p v-for="n in configuration.notes" :key="n" class="config-note">{{ n }}</p>
  </section>
</template>

<style scoped>
h4 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint); font-weight: 700; margin: 0 0 16px; }
.config { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; }
.config-col { display: flex; flex-direction: column; gap: 9px; padding: 16px 18px; border-radius: 14px; background: var(--surface); border: 1px solid var(--border); }
.config-col:empty { display: none; }
.config-titre { font-weight: 800; font-size: 14px; color: var(--text); }
.config-ligne { display: flex; gap: 12px; font-size: 13px; color: var(--text-dim); }
.config-libelle { flex: 0 0 120px; color: var(--text-faint); }
.config-note { margin: 10px 0 0; font-size: 12px; color: var(--text-faint); line-height: 1.5; }
</style>
