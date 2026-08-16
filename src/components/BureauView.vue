<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { PLATFORMS, platformName } from "../data/platforms";
import type { Game, LibraryFilter, SortKey } from "../types";
import Sidebar from "./Sidebar.vue";
import TopBar from "./TopBar.vue";
import HeroFeatured from "./HeroFeatured.vue";
import GameCard from "./GameCard.vue";
import StoreView from "./StoreView.vue";
import FriendsView from "./FriendsView.vue";
import CommonView from "./CommonView.vue";
import WishlistView from "./WishlistView.vue";

const { filtered, games } = useLibrary();
const { section, filter, query, sort, setSort, listView, installedOnly, toggleInstalledOnly, genre, setGenre, openGame } = useUi();

/** Le filtre courant vise-t-il une plateforme précise (vs « Tous », « Favoris »…) ? */
const isPlatformView = computed(() => filter.value in PLATFORMS);

/**
 * Catégories (genres) disponibles dans la bibliothèque visible, triées par
 * nombre de jeux décroissant. Alimente le menu déroulant de filtrage.
 */
const availableGenres = computed(() => {
  const counts = new Map<string, number>();
  for (const g of games.value) {
    if (g.hidden || !g.genre) continue;
    counts.set(g.genre, (counts.get(g.genre) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name, "fr"));
});

// Menu déroulant des catégories.
const genreMenuOpen = ref(false);
function pickGenre(g: string | null) {
  setGenre(g);
  genreMenuOpen.value = false;
}
function onDocClick(e: MouseEvent) {
  if (!(e.target as HTMLElement).closest(".genre-wrap")) genreMenuOpen.value = false;
}
onMounted(() => document.addEventListener("click", onDocClick));
onBeforeUnmount(() => document.removeEventListener("click", onDocClick));

/** Tri de la liste selon la puce active. */
function sortGames(list: Game[], key: SortKey): Game[] {
  const by = [...list];
  switch (key) {
    case "alpha":
      return by.sort((a, b) => a.title.localeCompare(b.title, "fr", { sensitivity: "base" }));
    case "playtime":
      return by.sort((a, b) => (b.hoursPlayed ?? 0) - (a.hoursPlayed ?? 0));
    case "recent":
    default:
      return by.sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));
  }
}

const shownGames = computed(() => {
  let list = filtered(filter.value, query.value);
  // Dans une vue de plateforme, le toggle « Installés uniquement » restreint la liste.
  if (isPlatformView.value && installedOnly.value) list = list.filter((g) => g.installed);
  // Filtre catégorie (genre), combiné aux autres filtres.
  if (genre.value) list = list.filter((g) => g.genre === genre.value);
  return sortGames(list, sort.value);
});

const SORTS: { key: SortKey; label: string }[] = [
  { key: "recent", label: "Récemment joué" },
  { key: "alpha", label: "A → Z" },
  { key: "playtime", label: "Temps de jeu" },
];

const FILTER_LABELS: Record<string, string> = {
  all: "Tous les jeux",
  mine: "Mes jeux",
  family: "Partagés en famille",
  recent: "Joués récemment",
  favorite: "Favoris",
  installed: "Installés",
  hidden: "Masqués",
};
function title(f: LibraryFilter): string {
  return FILTER_LABELS[f] ?? platformName(f as never);
}
</script>

<template>
  <div class="bureau">
    <Sidebar />
    <main class="main">
      <TopBar />

      <StoreView v-if="section === 'store'" />
      <FriendsView v-else-if="section === 'friends'" />
      <CommonView v-else-if="section === 'common'" />
      <WishlistView v-else-if="section === 'wishlist'" />
      <template v-else>
      <HeroFeatured />

      <div class="sec-head">
        <h2>{{ title(filter) }}</h2>
        <span class="n">{{ shownGames.length }} jeu{{ shownGames.length > 1 ? "x" : "" }}</span>
        <span class="spacer" />
        <div v-if="availableGenres.length" class="genre-wrap">
          <button
            class="chip genre-btn"
            :class="{ active: !!genre }"
            :aria-expanded="genreMenuOpen"
            @click.stop="genreMenuOpen = !genreMenuOpen"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 5h18M6 12h12M10 19h4" /></svg>
            {{ genre ?? "Toutes catégories" }}
            <svg class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
          </button>
          <div v-if="genreMenuOpen" class="genre-menu" @click.stop>
            <button class="genre-opt" :class="{ on: !genre }" @click="pickGenre(null)">
              <span>Toutes les catégories</span>
            </button>
            <div class="genre-sep" />
            <button
              v-for="g in availableGenres"
              :key="g.name"
              class="genre-opt"
              :class="{ on: genre === g.name }"
              @click="pickGenre(g.name)"
            >
              <span class="genre-name">{{ g.name }}</span>
              <span class="genre-count">{{ g.count }}</span>
            </button>
          </div>
        </div>
        <button
          v-if="isPlatformView"
          class="chip toggle"
          :class="{ active: installedOnly }"
          :aria-pressed="installedOnly"
          title="N'afficher que les jeux installés"
          @click="toggleInstalledOnly()"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v11m0 0l-4-4m4 4l4-4" /><path d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2" /></svg>
          Installés uniquement
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
      <div v-if="!shownGames.length" class="empty">Aucun jeu ne correspond à ta recherche.</div>
      </template>
    </main>
  </div>
</template>

<style scoped>
.bureau { display: grid; grid-template-columns: 262px 1fr; }
.main { min-width: 0; padding: 0 34px 60px; }
.sec-head { display: flex; align-items: baseline; gap: 12px; margin-bottom: 18px; }
.sec-head h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.sec-head .n { font-family: var(--mono); font-size: 13px; color: var(--text-faint); font-variant-numeric: tabular-nums; }
.sec-head .spacer { flex: 1; }
.chip {
  padding: 6px 13px; border-radius: 99px; font-size: 12.5px; color: var(--text-dim);
  background: var(--surface); border: 1px solid var(--border); transition: all 0.15s;
}
.chip:hover { color: var(--text); border-color: var(--border-strong); }
.chip.active { background: var(--text); color: var(--bg); border-color: var(--text); font-weight: 600; }
.chip.toggle { display: inline-flex; align-items: center; gap: 6px; }
.chip.toggle svg { width: 14px; height: 14px; }

/* Filtre catégorie (genre) : bouton + menu déroulant. */
.genre-wrap { position: relative; }
.genre-btn { display: inline-flex; align-items: center; gap: 6px; }
.genre-btn svg { width: 14px; height: 14px; }
.genre-btn .caret { width: 13px; height: 13px; margin-left: -1px; opacity: 0.7; }
.genre-btn.active {
  background: var(--accent-soft); color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent); font-weight: 600;
}
.genre-menu {
  position: absolute; top: calc(100% + 6px); right: 0; z-index: 30; min-width: 210px;
  max-height: 320px; overflow-y: auto;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px;
  box-shadow: var(--shadow-hero); padding: 6px; display: flex; flex-direction: column; gap: 1px;
}
.genre-sep { height: 1px; background: var(--border); margin: 4px 6px; }
.genre-opt {
  display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13px; text-align: left;
  width: 100%; cursor: pointer;
}
.genre-opt:hover { background: var(--surface-2); }
.genre-opt.on { color: var(--accent); font-weight: 600; }
.genre-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.genre-count {
  font-family: var(--mono); font-size: 11px; color: var(--text-faint); font-variant-numeric: tabular-nums;
}
.genre-opt.on .genre-count { color: var(--accent); }
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

@media (max-width: 820px) {
  .bureau { grid-template-columns: 1fr; }
  .main { padding: 0 18px 40px; }
}
</style>
