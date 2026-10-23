<script setup lang="ts">
import { computed } from "vue";
import { t } from "../i18n";

/**
 * Onglets de pages numérotés (‹ 1 2 3 … 16 ›), pour feuilleter une longue liste de la
 * fiche cinq éléments à la fois. Au-delà de 7 pages, seuls la première, la dernière et
 * les voisines de la page courante sont proposées.
 */
const props = defineProps<{ pages: number; libelle: string }>();
/** Page affichée, à partir de 0. */
const page = defineModel<number>({ required: true });

const numeros = computed<(number | null)[]>(() => {
  const n = props.pages;
  const p = page.value;
  if (n <= 7) return [...Array(n).keys()];
  const gardes = new Set([0, n - 1, p - 1, p, p + 1]);
  // Près d'un bout, on garde quelques pages de plus pour que la rangée ne rétrécisse pas.
  if (p <= 2) [1, 2, 3].forEach((i) => gardes.add(i));
  if (p >= n - 3) [n - 4, n - 3, n - 2].forEach((i) => gardes.add(i));
  const tries = [...gardes].filter((i) => i >= 0 && i < n).sort((a, b) => a - b);
  const out: (number | null)[] = [];
  tries.forEach((i, k) => {
    if (k && i - tries[k - 1] > 1) out.push(null);
    out.push(i);
  });
  return out;
});
</script>

<template>
  <nav v-if="pages > 1" class="pages" :aria-label="libelle">
    <button class="chip fleche" :disabled="page === 0" :aria-label="t('fiche.pages.precedente')" @click="page--">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
    </button>
    <template v-for="(i, k) in numeros" :key="i ?? `e${k}`">
      <span v-if="i === null" class="ellipse" aria-hidden="true">…</span>
      <button
        v-else
        class="chip"
        :class="{ active: i === page }"
        :aria-current="i === page ? 'page' : undefined"
        :aria-label="t('fiche.pages.page', { n: i + 1 })"
        @click="page = i"
      >
        {{ i + 1 }}
      </button>
    </template>
    <button class="chip fleche" :disabled="page === pages - 1" :aria-label="t('fiche.pages.suivante')" @click="page++">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
    </button>
  </nav>
</template>

<style scoped>
.pages { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.chip { min-width: 34px; text-align: center; font-family: var(--mono); font-variant-numeric: tabular-nums; }
.fleche { display: inline-grid; place-items: center; padding: 6px 9px; }
.fleche svg { width: 15px; height: 15px; }
.fleche:disabled { opacity: 0.35; cursor: default; }
.ellipse { color: var(--text-faint); font-family: var(--mono); padding: 0 2px; }
</style>
