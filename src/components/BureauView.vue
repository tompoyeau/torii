<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { PLATFORMS, platformName } from "../data/platforms";
import type { Game, LibraryFilter, SortKey } from "../types";
import Sidebar from "./Sidebar.vue";
import TopBar from "./TopBar.vue";
import HeroFeatured from "./HeroFeatured.vue";
import GameCard from "./GameCard.vue";

const { filtered } = useLibrary();
const { filter, query, sort, setSort, listView, installedOnly, toggleInstalledOnly, openGame } = useUi();

/** Le filtre courant vise-t-il une plateforme précise (vs « Tous », « Favoris »…) ? */
const isPlatformView = computed(() => filter.value in PLATFORMS);

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

const games = computed(() => {
  let list = filtered(filter.value, query.value);
  // Dans une vue de plateforme, le toggle « Installés uniquement » restreint la liste.
  if (isPlatformView.value && installedOnly.value) list = list.filter((g) => g.installed);
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
      <HeroFeatured />

      <div class="sec-head">
        <h2>{{ title(filter) }}</h2>
        <span class="n">{{ games.length }} jeu{{ games.length > 1 ? "x" : "" }}</span>
        <span class="spacer" />
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
        <GameCard v-for="g in games" :key="g.id" :game="g" @open="openGame(g.id)" />
      </div>
      <div v-if="!games.length" class="empty">Aucun jeu ne correspond à ta recherche.</div>
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
/* Toggle « Installés » actif : couleur d'accent (se distingue des puces de tri). */
.chip.toggle.active {
  background: var(--accent-soft); color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent); font-weight: 600;
}

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(178px, 1fr)); gap: 22px 20px; }
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
