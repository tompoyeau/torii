<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { platformName } from "../data/platforms";
import { launchGame } from "../lib/tauri";

const { spotlight, markPlayed } = useLibrary();
const { openGame } = useUi();

const game = computed(() => spotlight.value[0] ?? null);

/** Note le jeu comme joué (dernière session) puis le lance. */
function play() {
  if (!game.value) return;
  markPlayed(game.value.id);
  launchGame(game.value);
}

function hideBrokenCover(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}
</script>

<template>
  <section v-if="game" class="hero">
    <div class="hero-art" :style="{ background: game.cover }" />
    <img v-if="game.heroUrl" class="hero-img" :src="game.heroUrl" alt="" @error="hideBrokenCover" />
    <div class="hero-scrim" />
    <div class="hero-eyebrow"><span class="pulse" />{{ game.recent ? "Reprendre la partie" : "À l'honneur" }}</div>
    <h1 class="hero-title">{{ game.title }}</h1>
    <div class="hero-meta">
      <span>{{ platformName(game.platform) }}</span>
      <template v-if="game.genre"><span class="sep">•</span><span>{{ game.genre }}</span></template>
      <template v-if="game.hoursPlayed != null"><span class="sep">•</span><span>{{ game.hoursPlayed }} h de jeu</span></template>
      <template v-if="game.lastPlayed"><span class="sep">•</span><span>Joué {{ game.lastPlayed }}</span></template>
    </div>
    <div class="hero-actions">
      <button class="btn-play" @click="play()">
        <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>Jouer
      </button>
      <button class="btn-ghost" @click="openGame(game.id)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M12 8v.01M11 12h1v4h1" /></svg>Détails
      </button>
    </div>
  </section>
</template>

<style scoped>
.hero {
  position: relative; border-radius: 22px; overflow: hidden; padding: 30px 34px; margin: 8px 0 34px;
  min-height: 268px; display: flex; flex-direction: column; justify-content: flex-end;
  box-shadow: var(--shadow-hero); border: 1px solid var(--border); isolation: isolate;
}
.hero-art {
  position: absolute; inset: 0; z-index: -2;
  background: radial-gradient(120% 140% at 82% 18%, #ff7a4d 0%, #b0288a 34%, #3a1c6e 66%, #14102a 100%);
}
.hero-art::after {
  content: ""; position: absolute; inset: 0;
  background: repeating-linear-gradient(115deg, rgba(255, 255, 255, 0.05) 0 2px, transparent 2px 9px),
              radial-gradient(80% 60% at 20% 110%, rgba(0, 0, 0, 0.55), transparent 70%);
  mix-blend-mode: overlay;
}
.hero-img {
  position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: -2; display: block;
}
.hero-scrim {
  position: absolute; inset: 0; z-index: -1;
  background: linear-gradient(90deg, rgba(8, 5, 14, 0.82) 0%, rgba(8, 5, 14, 0.5) 40%, transparent 78%);
}
.hero-eyebrow {
  font-family: var(--mono); font-size: 11px; letter-spacing: 0.16em; text-transform: uppercase;
  color: #ffd9c9; margin-bottom: 12px; display: flex; align-items: center; gap: 8px;
}
.pulse {
  width: 7px; height: 7px; border-radius: 50%; background: #6ee7a8; animation: pulse 2.2s infinite;
}
@keyframes pulse {
  0% { box-shadow: 0 0 0 0 rgba(110, 231, 168, 0.6); }
  70% { box-shadow: 0 0 0 9px rgba(110, 231, 168, 0); }
  100% { box-shadow: 0 0 0 0 rgba(110, 231, 168, 0); }
}
.hero-title { font-size: 40px; font-weight: 800; letter-spacing: -0.03em; margin: 0 0 10px; color: #fff; text-wrap: balance; }
.hero-meta {
  font-family: var(--mono); font-size: 12.5px; color: rgba(255, 255, 255, 0.82);
  display: flex; flex-wrap: wrap; align-items: center; gap: 9px; margin-bottom: 20px;
}
.hero-meta .sep { opacity: 0.45; }
.hero-actions { display: flex; gap: 10px; flex-wrap: wrap; }
</style>
