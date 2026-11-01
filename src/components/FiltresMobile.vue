<script setup lang="ts">
import { computed } from "vue";
import { useFiltresBibliotheque } from "../composables/useFiltresBibliotheque";
import { useUi } from "../composables/useUi";
import { t } from "../i18n";
import type { LibraryFilter } from "../types";

/** Sur téléphone, les filtres de la barre latérale : une rangée de puces qui défile. */
const { filter, setFilter } = useUi();
const { count, platforms } = useFiltresBibliotheque();

const filtres = computed(() => {
  const liste: { id: LibraryFilter; label: string }[] = [{ id: "all", label: t("bibliotheque.laterale.tous") }];
  if (count("family").value) {
    liste.push(
      { id: "mine", label: t("bibliotheque.laterale.mesJeux") },
      { id: "family", label: t("bibliotheque.laterale.famille") },
    );
  }
  return [...liste, ...platforms.value].map((f) => ({ ...f, n: count(f.id).value }));
});
</script>

<template>
  <nav v-if="filtres.length > 1" class="filtres no-scrollbar" :aria-label="t('bibliotheque.laterale.bibliotheque')">
    <button
      v-for="f in filtres"
      :key="f.id"
      class="chip"
      :class="{ active: filter === f.id }"
      :aria-pressed="filter === f.id"
      @click="setFilter(f.id)"
    >
      {{ f.label }} <span class="n">{{ f.n }}</span>
    </button>
  </nav>
</template>

<style scoped>
.filtres { display: none; }
.n { font-family: var(--mono); font-size: 11px; opacity: 0.75; font-variant-numeric: tabular-nums; }

@media (max-width: 820px) {
  .filtres { display: flex; gap: 8px; overflow-x: auto; margin: 0 -18px 16px; padding: 0 18px; }
  .chip { flex: none; }
}
</style>
