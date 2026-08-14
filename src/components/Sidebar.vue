<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { getSettings } from "../lib/tauri";
import type { LibraryFilter, PlatformId, Settings } from "../types";
import PlatformIcon from "./PlatformIcon.vue";

const { games } = useLibrary();
const { section, filter, setFilter, showStore, showCommon, showWishlist, openSettings, settingsOpen } = useUi();

// État de connexion des launchers, pour adapter le libellé du bouton du bas.
// Rafraîchi au montage et à chaque fermeture du panneau (où les connexions changent).
const settings = ref<Settings | null>(null);
async function refreshSettings() {
  // Hors Tauri (preview) : état fictif mixte (Steam + Epic connectés) pour la maquette.
  settings.value =
    (await getSettings()) ??
    { steamConnected: true, steamId: "0", epicConnected: true, eaConnected: false, battlenetConnected: false, gogConnected: false };
}
onMounted(refreshSettings);
watch(settingsOpen, (open) => {
  if (!open) refreshSettings();
});

// Les 5 launchers à compte (les seuls qui se « connectent »).
const allConnected = computed(() => {
  const s = settings.value;
  return (
    !!s && s.steamConnected && s.epicConnected && s.eaConnected && s.battlenetConnected && s.gogConnected
  );
});
const launcherLabel = computed(() =>
  allConnected.value ? "Gérer les connexions" : "Connecter un launcher",
);

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
      <button class="nav-item" :class="{ active: section === 'library' && filter ==='all' }" @click="setFilter('all')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" /></svg></span>
        Tous les jeux <span class="count">{{ count("all").value }}</span>
      </button>
      <button v-if="count('family').value" class="nav-item" :class="{ active: section === 'library' && filter ==='mine' }" @click="setFilter('mine')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="8" r="4" /><path d="M5 20a7 7 0 0 1 14 0" /></svg></span>
        Mes jeux <span class="count">{{ count("mine").value }}</span>
      </button>
      <button v-if="count('family').value" class="nav-item" :class="{ active: section === 'library' && filter ==='family' }" @click="setFilter('family')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="9" cy="9" r="3" /><circle cx="17" cy="15" r="3" /><path d="M3 20a6 6 0 0 1 12 0M13 20a5 5 0 0 1 8 0" /></svg></span>
        Famille <span class="count">{{ count("family").value }}</span>
      </button>
      <button class="nav-item" :class="{ active: section === 'library' && filter ==='recent' }" @click="setFilter('recent')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg></span>
        Récents <span class="count">{{ count("recent").value }}</span>
      </button>
      <button class="nav-item" :class="{ active: section === 'library' && filter ==='favorite' }" @click="setFilter('favorite')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M12 4.5l2.3 4.7 5.2.8-3.8 3.7.9 5.1L12 16.9l-4.6 2.4.9-5.1L4.5 10l5.2-.8z" /></svg></span>
        Favoris <span class="count">{{ count("favorite").value }}</span>
      </button>
      <button class="nav-item" :class="{ active: section === 'library' && filter ==='installed' }" @click="setFilter('installed')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M12 3v11m0 0l-4-4m4 4l4-4" /><path d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2" /></svg></span>
        Installés <span class="count">{{ count("installed").value }}</span>
      </button>
      <button v-if="count('hidden').value" class="nav-item" :class="{ active: section === 'library' && filter ==='hidden' }" @click="setFilter('hidden')">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M3 3l18 18M10.6 10.7a2 2 0 0 0 2.8 2.8" /><path d="M9.4 5.2A9.3 9.3 0 0 1 12 5c5 0 9 4.5 9 7a12 12 0 0 1-2.2 3M6.1 6.2A12.7 12.7 0 0 0 3 12c0 2.5 4 7 9 7a9.4 9.4 0 0 0 3.6-.7" /></svg></span>
        Masqués <span class="count">{{ count("hidden").value }}</span>
      </button>
    </nav>

    <nav class="nav-group">
      <div class="nav-label">Découvrir</div>
      <button class="nav-item" :class="{ active: section === 'store' }" @click="showStore()">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 8h16l-1 4a3 3 0 0 1-3 2.4H8A3 3 0 0 1 5 12Z" /><path d="M4 8l1.4-3.4A2 2 0 0 1 7.2 3.4h9.6a2 2 0 0 1 1.8 1.2L20 8" /><path d="M6 14.4V20a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1v-5.6" /></svg></span>
        Boutique
      </button>
      <button class="nav-item" :class="{ active: section === 'common' }" @click="showCommon()">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="8.5" cy="12" r="5.2" /><circle cx="15.5" cy="12" r="5.2" /></svg></span>
        En commun
      </button>
      <button class="nav-item" :class="{ active: section === 'wishlist' }" @click="showWishlist()">
        <span class="tile"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M12 20s-7-4.3-7-9.3A3.7 3.7 0 0 1 12 8a3.7 3.7 0 0 1 7 2.7c0 5-7 9.3-7 9.3Z" /></svg></span>
        Wishlist
      </button>
    </nav>

    <nav class="nav-group">
      <div class="nav-label">Plateformes</div>
      <button v-for="p in platforms" :key="p.id" class="nav-item plat"
              :class="{ active: section === 'library' && filter ===p.id }" @click="setFilter(p.id)">
        <span class="tile plat-tile"><PlatformIcon :platform="p.id" /></span>
        {{ p.label }} <span class="count">{{ count(p.id).value }}</span>
      </button>
    </nav>

    <button class="add-launcher" :class="{ manage: allConnected }" @click="openSettings">
      <svg v-if="allConnected" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 6h16M4 12h16M4 18h16" /><circle cx="9" cy="6" r="2" fill="var(--surface)" /><circle cx="15" cy="12" r="2" fill="var(--surface)" /><circle cx="8" cy="18" r="2" fill="var(--surface)" /></svg>
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M12 9v6M9 12h6" /></svg>
      <span>{{ launcherLabel }}</span>
    </button>
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
  position: relative;
  display: flex; align-items: center; gap: 10px; padding: 6px 10px; border-radius: 10px;
  color: var(--text-dim); background: none; border: none; width: 100%; font-size: 13.5px;
  text-align: left; transition: background 0.15s, color 0.15s;
}
.tile {
  width: 26px; height: 26px; border-radius: 8px; flex: none;
  background: var(--surface-2); color: var(--text-dim);
  display: grid; place-items: center; transition: background 0.15s, color 0.15s;
}
.tile svg { width: 15px; height: 15px; }
.tile .platform-icon { width: 18px; height: 18px; }
.nav-item .count {
  margin-left: auto; font-family: var(--mono); font-size: 10.5px; color: var(--text-faint);
  background: var(--surface-2); padding: 1px 7px; border-radius: 99px;
  font-variant-numeric: tabular-nums;
}
.nav-item:hover { background: var(--surface-2); color: var(--text); }
.nav-item:hover .tile { background: var(--surface-3); color: var(--text); }
.nav-item.active { background: var(--accent-soft); color: var(--text); font-weight: 600; }
.nav-item.active::before {
  content: ""; position: absolute; left: 0; top: 7px; bottom: 7px; width: 3px;
  border-radius: 0 3px 3px 0; background: var(--accent);
}
.nav-item.active .tile { background: var(--accent); color: var(--accent-ink); }
.nav-item.active .count { background: var(--accent); color: var(--accent-ink); }
/* Les tuiles de plateforme gardent leur icône colorée : pas de remplissage accent. */
.nav-item.plat.active .tile { background: var(--surface-3); }
.add-launcher {
  margin-top: auto; display: flex; align-items: center; gap: 10px; width: 100%;
  padding: 11px 14px; border-radius: 99px; border: none;
  background: var(--surface-2); color: var(--text-dim); font-size: 13px; font-weight: 600;
  font-family: inherit; text-align: left; cursor: pointer;
  transition: color 0.15s, background 0.15s;
}
.add-launcher svg { width: 17px; height: 17px; flex: none; opacity: 0.9; }
.add-launcher:hover { color: var(--text); background: var(--surface-3); }

@media (max-width: 820px) {
  .sidebar { display: none; }
}
</style>
