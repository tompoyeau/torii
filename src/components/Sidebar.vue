<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import type { LibraryFilter, PlatformId } from "../types";

const { games } = useLibrary();
const { filter, setFilter } = useUi();

// Les compteurs des vues normales excluent les jeux masqués.
const visible = computed(() => games.value.filter((g) => !g.hidden));
const total = computed(() => visible.value.length);
const count = (f: LibraryFilter) =>
  computed(() => {
    switch (f) {
      case "all": return visible.value.length;
      case "mine": return visible.value.filter((g) => !g.familyShared).length;
      case "family": return visible.value.filter((g) => g.familyShared).length;
      case "recent": return visible.value.filter((g) => g.recent).length;
      case "favorite": return visible.value.filter((g) => g.favorite).length;
      case "installed": return visible.value.filter((g) => g.installed).length;
      case "hidden": return games.value.filter((g) => g.hidden).length;
      default: return visible.value.filter((g) =>
        g.sources ? g.sources.some((s) => s.platform === f) : g.platform === f,
      ).length;
    }
  });

const platforms: { id: PlatformId; label: string }[] = [
  { id: "steam", label: "Steam" },
  { id: "epic", label: "Epic Games" },
  { id: "gog", label: "GOG" },
  { id: "riot", label: "Riot Games" },
  { id: "ubisoft", label: "Ubisoft Connect" },
  { id: "ea", label: "EA" },
  { id: "battlenet", label: "Battle.net" },
  { id: "manual", label: "Manuel" },
];

const usedGb = computed(() =>
  Math.round(
    games.value
      .filter((g) => g.installed)
      .reduce((sum, g) => sum + (g.sizeGb ?? 0), 0),
  ),
);
const totalGb = 2000;
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">
        <svg viewBox="0 0 24 24" fill="#1a0f0c">
          <path d="M6.6 7.3 L8.7 7.3 L9.1 19.6 L6.2 19.6 Z" />
          <path d="M15.3 7.3 L17.4 7.3 L17.8 19.6 L14.9 19.6 Z" />
          <rect x="11.2" y="8.9" width="1.6" height="2.6" />
          <rect x="4.5" y="11.1" width="15" height="2.1" rx="0.4" />
          <path d="M2.5 5 Q12 7.5 21.5 5 L21.5 7.4 Q12 9.9 2.5 7.4 Z" />
          <path d="M2.5 5 L1.3 4 L0.9 5.7 L2.5 7.4 Z" />
          <path d="M21.5 5 L22.7 4 L23.1 5.7 L21.5 7.4 Z" />
        </svg>
      </div>
      <div>
        <div class="brand-name">Torii</div>
        <div class="brand-sub">{{ total }} jeux</div>
      </div>
    </div>

    <nav class="nav-group">
      <div class="nav-label">Bibliothèque</div>
      <button class="nav-item" :class="{ active: filter === 'all' }" @click="setFilter('all')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" /></svg>
        Tous les jeux <span class="count">{{ count("all").value }}</span>
      </button>
      <button v-if="count('family').value" class="nav-item" :class="{ active: filter === 'mine' }" @click="setFilter('mine')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="8" r="4" /><path d="M5 20a7 7 0 0 1 14 0" /></svg>
        Mes jeux <span class="count">{{ count("mine").value }}</span>
      </button>
      <button v-if="count('family').value" class="nav-item" :class="{ active: filter === 'family' }" @click="setFilter('family')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="9" cy="9" r="3" /><circle cx="17" cy="15" r="3" /><path d="M3 20a6 6 0 0 1 12 0M13 20a5 5 0 0 1 8 0" /></svg>
        Famille <span class="count">{{ count("family").value }}</span>
      </button>
      <button class="nav-item" :class="{ active: filter === 'recent' }" @click="setFilter('recent')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>
        Récents <span class="count">{{ count("recent").value }}</span>
      </button>
      <button class="nav-item" :class="{ active: filter === 'favorite' }" @click="setFilter('favorite')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M12 4.5l2.3 4.7 5.2.8-3.8 3.7.9 5.1L12 16.9l-4.6 2.4.9-5.1L4.5 10l5.2-.8z" /></svg>
        Favoris <span class="count">{{ count("favorite").value }}</span>
      </button>
      <button class="nav-item" :class="{ active: filter === 'installed' }" @click="setFilter('installed')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M12 3v11m0 0l-4-4m4 4l4-4" /><path d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2" /></svg>
        Installés <span class="count">{{ count("installed").value }}</span>
      </button>
      <button v-if="count('hidden').value" class="nav-item" :class="{ active: filter === 'hidden' }" @click="setFilter('hidden')">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M3 3l18 18M10.6 10.7a2 2 0 0 0 2.8 2.8" /><path d="M9.4 5.2A9.3 9.3 0 0 1 12 5c5 0 9 4.5 9 7a12 12 0 0 1-2.2 3M6.1 6.2A12.7 12.7 0 0 0 3 12c0 2.5 4 7 9 7a9.4 9.4 0 0 0 3.6-.7" /></svg>
        Masqués <span class="count">{{ count("hidden").value }}</span>
      </button>
    </nav>

    <nav class="nav-group">
      <div class="nav-label">Plateformes</div>
      <button v-for="p in platforms" :key="p.id" class="nav-item plat"
              :class="{ active: filter === p.id }" @click="setFilter(p.id)">
        <span class="dot" :style="{ background: `var(--${p.id})` }" />
        {{ p.label }} <span class="count">{{ count(p.id).value }}</span>
      </button>
    </nav>

    <div class="storage">
      <div class="storage-top"><span>Stockage</span><span><b>{{ usedGb }}</b> / {{ totalGb }} Go</span></div>
      <div class="storage-bar"><div class="storage-fill" :style="{ width: `${(usedGb / totalGb) * 100}%` }" /></div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  border-right: 1px solid var(--border); padding: 22px 16px 16px;
  display: flex; flex-direction: column; gap: 22px; height: 100vh;
  position: sticky; top: 0; overflow-y: auto;
}
.brand { display: flex; align-items: center; gap: 11px; padding: 4px 8px; }
.brand-mark {
  width: 34px; height: 34px; border-radius: 10px; flex: none;
  background: linear-gradient(140deg, var(--accent), #ff9a6b);
  display: grid; place-items: center; box-shadow: 0 6px 18px -6px var(--accent);
}
.brand-mark svg { width: 19px; height: 19px; }
.brand-name { font-weight: 700; font-size: 16px; letter-spacing: -0.02em; }
.brand-sub { font-size: 11px; color: var(--text-faint); font-family: var(--mono); letter-spacing: 0.04em; }
.nav-group { display: flex; flex-direction: column; gap: 2px; }
.nav-label {
  font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.13em;
  color: var(--text-faint); font-weight: 700; padding: 0 10px 7px;
}
.nav-item {
  display: flex; align-items: center; gap: 11px; padding: 8px 10px; border-radius: 10px;
  color: var(--text-dim); background: none; border: none; width: 100%; font-size: 13.5px;
  text-align: left; transition: background 0.15s, color 0.15s;
}
.nav-item .ico { width: 17px; height: 17px; flex: none; opacity: 0.85; }
.nav-item .count {
  margin-left: auto; font-family: var(--mono); font-size: 11px; color: var(--text-faint);
  font-variant-numeric: tabular-nums;
}
.nav-item .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
.nav-item:hover { background: var(--surface-2); color: var(--text); }
.nav-item.active { background: var(--accent-soft); color: var(--text); font-weight: 600; }
.nav-item.active .count { color: var(--accent); }
.nav-item.plat { padding-left: 12px; }
.storage { margin-top: auto; padding: 14px 12px 6px; }
.storage-top { display: flex; justify-content: space-between; font-size: 11.5px; color: var(--text-dim); margin-bottom: 8px; }
.storage-top b { color: var(--text); font-family: var(--mono); font-weight: 600; }
.storage-bar { height: 6px; border-radius: 99px; background: var(--surface-3); overflow: hidden; }
.storage-fill { height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--accent), #ffb08a); }

@media (max-width: 820px) {
  .sidebar { display: none; }
}
</style>
