<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useTheme } from "../composables/useTheme";
import ModeSwitch from "./ModeSwitch.vue";
import SalonHero from "./SalonHero.vue";
import SalonRow from "./SalonRow.vue";

const { games } = useLibrary();
const { toggle: toggleTheme } = useTheme();

const rows = computed(() =>
  [
    { title: "Reprendre", games: games.value.filter((g) => g.recent) },
    { title: "Tes favoris", games: games.value.filter((g) => g.favorite) },
    { title: "Installés", games: games.value.filter((g) => g.installed) },
    { title: "Toute la bibliothèque", games: games.value },
  ].filter((row) => row.games.length > 0),
);
</script>

<template>
  <div class="salon">
    <div class="salon-bar">
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
        <div class="brand-name">Torii</div>
      </div>
      <div class="spacer" />
      <ModeSwitch />
      <button class="icon-btn" title="Thème clair / sombre" @click="toggleTheme">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M20 14.5A8 8 0 1 1 9.5 4 6.5 6.5 0 0 0 20 14.5Z" /></svg>
      </button>
      <div class="avatar">T</div>
    </div>

    <SalonHero />

    <div class="salon-rows">
      <SalonRow v-for="row in rows" :key="row.title" :title="row.title" :games="row.games" />
    </div>
  </div>
</template>

<style scoped>
.salon { min-height: 100vh; }
.salon-bar {
  position: sticky; top: 0; z-index: 30; display: flex; align-items: center; gap: 16px;
  padding: 16px 40px; background: linear-gradient(180deg, rgba(10, 7, 16, 0.75), transparent);
  backdrop-filter: blur(3px);
}
:root[data-theme="light"] .salon-bar { background: linear-gradient(180deg, rgba(255, 255, 255, 0.7), transparent); }
.brand { display: flex; align-items: center; gap: 11px; }
.brand-mark {
  width: 34px; height: 34px; border-radius: 10px; flex: none;
  background: linear-gradient(140deg, var(--accent), #ff9a6b); display: grid; place-items: center;
  box-shadow: 0 6px 18px -6px var(--accent);
}
.brand-mark svg { width: 19px; height: 19px; }
.brand-name { font-weight: 700; font-size: 16px; letter-spacing: -0.02em; color: #fff; }
:root[data-theme="light"] .brand-name { color: var(--text); }
.spacer { flex: 1; }
.icon-btn {
  width: 38px; height: 38px; border-radius: 11px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text-dim); display: grid; place-items: center; transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover { color: var(--text); border-color: var(--border-strong); }
.icon-btn svg { width: 17px; height: 17px; }
.avatar {
  width: 38px; height: 38px; border-radius: 50%; flex: none;
  background: linear-gradient(140deg, #8a6cff, var(--accent)); display: grid; place-items: center;
  color: #fff; font-weight: 700; font-size: 14px;
}
.salon-rows { padding: 8px 0 70px; display: flex; flex-direction: column; gap: 38px; position: relative; z-index: 2; }
</style>
