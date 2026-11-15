<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { titreFiltre } from "../data/platforms";
import type { Game, SortKey } from "../types";
import { etiquetteIntl, t } from "../i18n";
import { MODE_WEB } from "../lib/mode";
import FiltresBibliotheque from "./FiltresBibliotheque.vue";
import HeroFeatured from "./HeroFeatured.vue";
import GameCard from "./GameCard.vue";

/**
 * L'écran Bibliothèque : la vedette, la barre d'outils et la grille.
 *
 * Un composant à part entière, et pas un morceau du gabarit d'`AppShell`, pour que `<KeepAlive>`
 * puisse le garder entre deux onglets : reconstruire la grille coûtait ~200 ms à chaque retour
 * avec quelques centaines de jeux (mesuré sur 364).
 */
const { filtered, games, loading, echec, reload } = useLibrary();
const { filter, query, sort, listView, installedOnly, categories, clearCategories, openGame, showStore } = useUi();

/** Tri de la liste selon le menu Trier. */
function sortGames(list: Game[], key: SortKey): Game[] {
  const by = [...list];
  switch (key) {
    case "alpha":
      return by.sort((a, b) => a.title.localeCompare(b.title, etiquetteIntl.value, { sensitivity: "base" }));
    case "playtime":
      return by.sort((a, b) => (b.hoursPlayed ?? 0) - (a.hoursPlayed ?? 0));
    case "recent":
    default:
      return by.sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));
  }
}

/** La liste avant le filtre des catégories : c'est sur elle qu'il compte ses jeux. */
const avantCategories = computed(() => {
  const list = filtered(filter.value, query.value);
  // « Installés uniquement » restreint n'importe quelle vue (tous, favoris, une plateforme…).
  return installedOnly.value && !MODE_WEB ? list.filter((g) => g.installed) : list;
});

const shownGames = computed(() => {
  let list = avantCategories.value;
  // Catégories cochées : un jeu doit les avoir toutes.
  if (categories.value.length) {
    list = list.filter((g) => categories.value.every((c) => g.tags?.includes(c)));
  }
  return sortGames(list, sort.value);
});
</script>

<template>
  <div>
    <HeroFeatured v-if="!MODE_WEB" />

    <!-- Le titre reste pour la structure de la page ; à l'écran, la barre d'outils le dit. -->
    <h2 class="cache">{{ titreFiltre(filter) }}</h2>
    <FiltresBibliotheque :affiches="shownGames.length" :avant-categories="avantCategories" />

    <div class="grid" :class="{ list: listView }">
      <GameCard v-for="g in shownGames" :key="g.id" :game="g" :actions="!MODE_WEB" @open="openGame(g.id)" />
    </div>
    <div v-if="!shownGames.length && categories.length && avantCategories.length" class="empty">
      <p>{{ t("bibliotheque.grille.aucunAvecCategories") }}</p>
      <button class="chip" @click="clearCategories()">{{ t("bibliotheque.grille.effacerCategories") }}</button>
    </div>
    <!-- Sur le web, une bibliothèque vide veut presque toujours dire : la synchronisation n'est pas
         activée sur le PC. Autant le dire, et ne pas laisser croire que les jeux ont disparu. -->
    <div v-else-if="MODE_WEB && echec && !games.length" class="empty">
      <p>{{ t("web.bibliotheque.indisponible") }}</p>
      <button class="chip" :disabled="loading" @click="reload()">{{ t("boutique.accueil.reessayer") }}</button>
    </div>
    <div v-else-if="MODE_WEB && !games.length && !loading" class="empty invitation">
      <h3>{{ t("web.bibliotheque.videTitre") }}</h3>
      <p>{{ t("web.bibliotheque.videTexte") }}</p>
      <button class="chip" @click="showStore()">{{ t("web.bibliotheque.versBoutique") }}</button>
    </div>
    <div v-else-if="!shownGames.length && !(MODE_WEB && loading)" class="empty">{{ t("bibliotheque.grille.aucun") }}</div>
  </div>
</template>

<style scoped>
/* Lu par les lecteurs d'écran, invisible à l'écran. */
.cache {
  position: absolute; width: 1px; height: 1px; margin: -1px; padding: 0; border: 0;
  overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap;
}
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(var(--card-min, 178px), 1fr)); gap: 22px 20px; }
/* Les cartes hors de l'écran ne sont ni mises en page ni dessinées (`content-visibility`) : avec
   des centaines de jeux, revenir sur l'onglet ne recalcule que ce qu'on voit. `auto` dans la
   taille réservée reprend la vraie hauteur une fois la carte vue : la barre de défilement ne
   saute pas. */
/* ⚠️ `content-visibility` rogne ce qui dépasse de la carte (confinement du dessin) : l'ombre de la
   jaquette déborde d'une vingtaine de pixels et elle se soulève au survol. `overflow-clip-margin`
   repousse ce bord. */
.grid:not(.list) :deep(.card) {
  content-visibility: auto; contain-intrinsic-size: auto 330px; overflow-clip-margin: 32px;
}
.grid.list { grid-template-columns: 1fr; gap: 8px; }
.grid.list :deep(.card) { flex-direction: row; align-items: center; gap: 16px; padding: 8px; border-radius: 14px; }
.grid.list :deep(.card:hover) { background: var(--surface); }
.grid.list :deep(.cover) { width: 52px; aspect-ratio: 1; flex: none; border-radius: 11px; }
.grid.list :deep(.cover-title),
.grid.list :deep(.cover-hover),
.grid.list :deep(.cover-hide),
.grid.list :deep(.cover-scrim) { display: none; }
.grid.list :deep(.cover-plat) { top: 5px; left: 5px; width: 20px; height: 20px; }
.grid.list :deep(.cover-plat .platform-icon) { width: 11px; height: 11px; }
.grid.list :deep(.card:hover .cover) { transform: none; }
.grid.list :deep(.card-name) { font-size: 14.5px; }

.empty { padding: 60px 0; text-align: center; color: var(--text-faint); font-size: 14px; }
.empty p { margin: 0 0 14px; }
.invitation h3 { margin: 0 0 8px; font-size: 16px; color: var(--text); }
.invitation p { max-width: 52ch; margin-inline: auto; line-height: 1.5; color: var(--text-dim); }

/* Deux jaquettes par ligne sur téléphone : le réglage de leur taille vaut pour un écran de PC. */
@media (max-width: 620px) {
  .grid:not(.list) { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px 12px; }
}
</style>
