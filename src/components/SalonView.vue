<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useTheme } from "../composables/useTheme";
import ModeSwitch from "./ModeSwitch.vue";
import SalonHero from "./SalonHero.vue";
import SalonRow from "./SalonRow.vue";

import { useSalonNav } from "../composables/useSalonNav";

const { games } = useLibrary();
const { toggle: toggleTheme } = useTheme();

const rows = computed(() => {
  const visible = games.value.filter((g) => !g.hidden);
  const now = Date.now() / 1000;
  const SIXTY_DAYS = 60 * 24 * 3600;

  const recent = [...visible]
    .filter((g) => g.recent)
    .sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));

  // À redécouvrir : déjà joués mais pas récemment (> 60 j), du plus ancien au plus récent.
  const rediscover = visible
    .filter((g) => !g.recent && (g.hoursPlayed ?? 0) > 0 && g.lastPlayedAt && now - g.lastPlayedAt > SIXTY_DAYS)
    .sort((a, b) => (a.lastPlayedAt ?? 0) - (b.lastPlayedAt ?? 0))
    .slice(0, 20);

  // Genres les mieux fournis (≥ 3 jeux) → une rangée chacun, jusqu'à 2.
  const byGenre = new Map<string, number>();
  for (const g of visible) if (g.genre) byGenre.set(g.genre, (byGenre.get(g.genre) ?? 0) + 1);
  const topGenres = [...byGenre.entries()]
    .filter(([, n]) => n >= 3)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 2)
    .map(([genre]) => genre);
  const genreRows = topGenres.map((genre) => ({
    title: genre,
    games: visible.filter((g) => g.genre === genre),
  }));

  return [
    { title: "Reprendre", games: recent },
    { title: "Tes favoris", games: visible.filter((g) => g.favorite) },
    { title: "À redécouvrir", games: rediscover },
    ...genreRows,
    { title: "Installés", games: visible.filter((g) => g.installed) },
    { title: "Toute la bibliothèque", games: [...visible].sort((a, b) => a.title.localeCompare(b.title, "fr")) },
  ].filter((row) => row.games.length > 0);
});

// Navigation clavier / manette : rangée 0 = hero, rangées 1..N = contenu.
const { active, row, col, heroIndex, heroActive, setHero } = useSalonNav(() => rows.value);
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

    <SalonHero :index="heroIndex" :focused="heroActive" @select="setHero" />

    <div class="salon-rows">
      <SalonRow
        v-for="(r, i) in rows"
        :key="r.title"
        :title="r.title"
        :games="r.games"
        :active-col="active && row === i + 1 ? col : -1"
      />
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

/* Apparition en cascade des rangées au chargement du Salon. */
.salon-rows > * { animation: salon-rise 0.5s cubic-bezier(0.2, 0.7, 0.3, 1) both; }
.salon-rows > *:nth-child(1) { animation-delay: 0.02s; }
.salon-rows > *:nth-child(2) { animation-delay: 0.08s; }
.salon-rows > *:nth-child(3) { animation-delay: 0.14s; }
.salon-rows > *:nth-child(4) { animation-delay: 0.2s; }
.salon-rows > *:nth-child(5) { animation-delay: 0.26s; }
.salon-rows > *:nth-child(n + 6) { animation-delay: 0.32s; }
@keyframes salon-rise {
  from { opacity: 0; transform: translateY(16px); }
  to { opacity: 1; transform: translateY(0); }
}
@media (prefers-reduced-motion: reduce) {
  .salon-rows > * { animation: none; }
}
</style>
