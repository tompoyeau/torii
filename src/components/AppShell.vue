<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { useStore } from "../composables/useStore";
import { PLATFORMS, titreFiltre } from "../data/platforms";
import type { Game, SortKey } from "../types";
import { etiquetteIntl, t } from "../i18n";
import CategoryFilter from "./CategoryFilter.vue";
import Sidebar from "./Sidebar.vue";
import TopBar from "./TopBar.vue";
import HeroFeatured from "./HeroFeatured.vue";
import GameCard from "./GameCard.vue";
import StoreView from "./StoreView.vue";
import FriendsView from "./FriendsView.vue";
import CommonView from "./CommonView.vue";
import FriendProfileView from "./FriendProfileView.vue";
import FriendLibraryView from "./FriendLibraryView.vue";
import WishlistView from "./WishlistView.vue";
import PageWeb from "./PageWeb.vue";

const { filtered, games } = useLibrary();
const { section, filter, query, sort, setSort, listView, installedOnly, toggleInstalledOnly, categories, clearCategories, openGame, selectedGameId } = useUi();
const { selectedGameId: produitOuvert } = useStore();

/**
 * Une fiche (jeu, Boutique) recouvre le contenu, à droite de la barre latérale : ce qu'elle
 * cache sort de la tabulation et des lecteurs d'écran (`inert`). La barre, visible, reste
 * joignable — c'est pourquoi les fiches n'enferment plus le focus.
 */
const contenuRecouvert = computed(
  () => produitOuvert.value != null || (selectedGameId.value != null && games.value.some((g) => g.id === selectedGameId.value)),
);

/** Le filtre courant vise-t-il une plateforme précise (vs « Tous », « Favoris »…) ? */
const isPlatformView = computed(() => filter.value in PLATFORMS);

/** Tri de la liste selon la puce active. */
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
  // Dans une vue de plateforme, le toggle « Installés uniquement » restreint la liste.
  return isPlatformView.value && installedOnly.value ? list.filter((g) => g.installed) : list;
});

const shownGames = computed(() => {
  let list = avantCategories.value;
  // Catégories cochées : un jeu doit les avoir toutes.
  if (categories.value.length) {
    list = list.filter((g) => categories.value.every((c) => g.tags?.includes(c)));
  }
  return sortGames(list, sort.value);
});

const SORTS = computed<{ key: SortKey; label: string }[]>(() => [
  { key: "recent", label: t("bibliotheque.grille.tris.recent") },
  { key: "alpha", label: t("bibliotheque.grille.tris.alpha") },
  { key: "playtime", label: t("bibliotheque.grille.tris.playtime") },
]);
</script>

<template>
  <div class="shell">
    <Sidebar />
    <main class="main" :class="{ 'sur-page': section === 'web' }" :inert="contenuRecouvert">
      <TopBar />

      <PageWeb v-if="section === 'web'" />
      <StoreView v-else-if="section === 'store'" />
      <FriendsView v-else-if="section === 'friends'" />
      <CommonView v-else-if="section === 'common'" />
      <FriendProfileView v-else-if="section === 'friendProfile'" />
      <FriendLibraryView v-else-if="section === 'friendLibrary'" />
      <WishlistView v-else-if="section === 'wishlist'" />
      <template v-else>
      <HeroFeatured />

      <div class="sec-head">
        <h2>{{ titreFiltre(filter) }}</h2>
        <span class="n">{{ t("bibliotheque.jeux", { n: shownGames.length }) }}</span>
        <span class="spacer" />
        <CategoryFilter :games="avantCategories" />
        <button
          v-if="isPlatformView"
          class="chip toggle"
          :class="{ active: installedOnly }"
          :aria-pressed="installedOnly"
          :title="t('bibliotheque.grille.installesSeulementAide')"
          @click="toggleInstalledOnly()"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v11m0 0l-4-4m4 4l4-4" /><path d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2" /></svg>
          {{ t("bibliotheque.grille.installesSeulement") }}
        </button>
        <button
          v-for="s in SORTS"
          :key="s.key"
          class="chip"
          :class="{ active: sort === s.key }"
          @click="setSort(s.key)"
        >
          {{ s.label }}
        </button>
      </div>

      <div class="grid" :class="{ list: listView }">
        <GameCard v-for="g in shownGames" :key="g.id" :game="g" @open="openGame(g.id)" />
      </div>
      <div v-if="!shownGames.length && categories.length && avantCategories.length" class="empty">
        <p>{{ t("bibliotheque.grille.aucunAvecCategories") }}</p>
        <button class="chip" @click="clearCategories()">{{ t("bibliotheque.grille.effacerCategories") }}</button>
      </div>
      <div v-else-if="!shownGames.length" class="empty">{{ t("bibliotheque.grille.aucun") }}</div>
      </template>
    </main>
  </div>
</template>

<style scoped>
.shell { display: grid; grid-template-columns: var(--largeur-laterale) 1fr; }
.main { min-width: 0; padding: 0 34px 60px; }
/* La page web prend toute la hauteur sous la barre : pas de marge qui ferait défiler. */
.main.sur-page { padding-bottom: 0; }
.sec-head { display: flex; align-items: baseline; gap: 12px; margin-bottom: 18px; }
.sec-head h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.sec-head .n { font-family: var(--mono); font-size: 13px; color: var(--text-faint); font-variant-numeric: tabular-nums; }
.sec-head .spacer { flex: 1; }
/* Base `.chip` dans style.css (partagée avec la Boutique, les Amis, la Wishlist). */
.chip.toggle { display: inline-flex; align-items: center; gap: 6px; }
.chip.toggle svg { width: 14px; height: 14px; }

/* Toggle « Installés » actif : couleur d'accent (se distingue des puces de tri). */
.chip.toggle.active {
  background: var(--accent-soft); color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent); font-weight: 600;
}

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(var(--card-min, 178px), 1fr)); gap: 22px 20px; }
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

@media (max-width: 820px) {
  .shell { grid-template-columns: 1fr; }
  .main { padding: 0 18px 40px; }
}
</style>
