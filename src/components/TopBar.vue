<script setup lang="ts">
import { useUi } from "../composables/useUi";
import { useTheme } from "../composables/useTheme";
import { useLibrary } from "../composables/useLibrary";
import ModeSwitch from "./ModeSwitch.vue";

const { query, listView, toggleListView, openSettings, openAddGame } = useUi();
const { toggle: toggleTheme } = useTheme();
const { loading, enriching, enrichProgress, reload } = useLibrary();
</script>

<template>
  <div class="topbar">
    <label class="search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7" /><path d="m20 20-3-3" /></svg>
      <input v-model="query" type="text" placeholder="Rechercher dans la bibliothèque…" autocomplete="off" />
    </label>
    <div class="topbar-spacer" />
    <span v-if="loading" class="enrich-pill">
      <span class="spinner" />Actualisation…
    </span>
    <span v-else-if="enriching" class="enrich-pill">
      <span class="spinner" />
      <template v-if="enrichProgress">Métadonnées {{ enrichProgress.done }}/{{ enrichProgress.total }}</template>
      <template v-else>Métadonnées…</template>
    </span>
    <button class="add-btn" title="Ajouter un jeu manuellement" @click="openAddGame">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
      <span>Ajouter</span>
    </button>
    <ModeSwitch />
    <button class="icon-btn" title="Resynchroniser" :disabled="loading" @click="reload()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
    </button>
    <button class="icon-btn" title="Grille / liste" @click="toggleListView">
      <svg v-if="listView" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" /></svg>
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 6h16M4 12h16M4 18h16" /></svg>
    </button>
    <button class="icon-btn" title="Thème clair / sombre" @click="toggleTheme">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M20 14.5A8 8 0 1 1 9.5 4 6.5 6.5 0 0 0 20 14.5Z" /></svg>
    </button>
    <button class="icon-btn" title="Réglages / comptes" @click="openSettings">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="3" /><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.3 1a7 7 0 0 0-1.7-1L14.5 2h-5l-.4 2.5a7 7 0 0 0-1.7 1l-2.3-1-2 3.5 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.5 2.3-1a7 7 0 0 0 1.7 1l.4 2.5h5l.4-2.5a7 7 0 0 0 1.7-1l2.3 1 2-3.5-2-1.5a7 7 0 0 0 .1-1Z" /></svg>
    </button>
    <div class="avatar">T</div>
  </div>
</template>

<style scoped>
.topbar {
  position: sticky; top: 0; z-index: 20; display: flex; align-items: center; gap: 14px;
  padding: 18px 0 16px;
  background: linear-gradient(var(--bg) 60%, transparent); backdrop-filter: blur(6px);
}
.search { flex: 1; max-width: 420px; position: relative; display: flex; align-items: center; }
.search svg { position: absolute; left: 13px; width: 16px; height: 16px; color: var(--text-faint); }
.search input {
  width: 100%; padding: 10px 14px 10px 38px; background: var(--surface);
  border: 1px solid var(--border); border-radius: 11px; color: var(--text);
  font-size: 13.5px; font-family: inherit;
}
.search input::placeholder { color: var(--text-faint); }
.search input:focus { outline: none; border-color: var(--border-strong); background: var(--surface-2); }
.topbar-spacer { flex: 1; }
.enrich-pill {
  display: inline-flex; align-items: center; gap: 8px; padding: 6px 12px; border-radius: 99px;
  background: var(--accent-soft); color: var(--accent); font-size: 12px; font-weight: 600;
  font-family: var(--mono);
}
.spinner {
  width: 12px; height: 12px; border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-top-color: var(--accent); animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.icon-btn {
  width: 38px; height: 38px; border-radius: 11px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text-dim); display: grid; place-items: center;
  transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover { color: var(--text); border-color: var(--border-strong); }
.icon-btn svg { width: 17px; height: 17px; }
.add-btn {
  display: inline-flex; align-items: center; gap: 7px; height: 38px; padding: 0 15px 0 12px;
  border-radius: 11px; border: 1px solid var(--border); background: var(--surface); color: var(--text-dim);
  font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.add-btn:hover { color: var(--text); border-color: var(--border-strong); }
.add-btn svg { width: 16px; height: 16px; }
.avatar {
  width: 38px; height: 38px; border-radius: 50%; flex: none;
  background: linear-gradient(140deg, #8a6cff, var(--accent));
  display: grid; place-items: center; color: #fff; font-weight: 700; font-size: 14px;
}
</style>
