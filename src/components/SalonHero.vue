<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { platformName } from "../data/platforms";
import { launchGame } from "../lib/tauri";

const { spotlight, markPlayed } = useLibrary();
const { openGame } = useUi();

const index = ref(0);
const game = computed(() => spotlight.value[index.value] ?? null);

/** Note le jeu comme joué (dernière session) puis le lance. */
function play() {
  if (!game.value) return;
  markPlayed(game.value.id);
  launchGame(game.value);
}

let timer: number | undefined;
function restart() {
  clearInterval(timer);
  timer = window.setInterval(() => {
    if (spotlight.value.length) index.value = (index.value + 1) % spotlight.value.length;
  }, 6000);
}
function select(i: number) {
  index.value = i;
  restart();
}
watch(spotlight, restart, { immediate: true });
onBeforeUnmount(() => clearInterval(timer));

function hideBrokenCover(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}
</script>

<template>
  <section v-if="game" class="salon-hero">
    <div class="salon-hero-art" :style="{ background: game.cover }" />
    <img v-if="game.heroUrl" :key="game.id" class="salon-hero-img" :src="game.heroUrl" alt="" @error="hideBrokenCover" />
    <div class="salon-hero-scrim" />
    <div class="salon-hero-eyebrow">
      <span class="pulse" />{{ game.recent ? "Reprendre" : "À l'honneur" }}
    </div>
    <h1 class="salon-hero-title">{{ game.title }}</h1>
    <div class="salon-hero-meta">
      <span>{{ platformName(game.platform) }}</span>
      <template v-if="game.genre"><span class="sep">•</span><span>{{ game.genre }}</span></template>
      <template v-if="game.hoursPlayed != null"><span class="sep">•</span><span>{{ game.hoursPlayed }} h de jeu</span></template>
      <template v-else-if="game.sizeGb"><span class="sep">•</span><span>{{ game.sizeGb }} Go</span></template>
      <template v-if="game.developer"><span class="sep">•</span><span>{{ game.developer }}</span></template>
    </div>
    <div class="hero-actions">
      <button class="btn-play big" @click="play()">
        <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>Jouer
      </button>
      <button class="btn-ghost solid" @click="openGame(game.id)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M12 8v.01M11 12h1v4h1" /></svg>Détails
      </button>
    </div>
    <div class="salon-dots">
      <button v-for="(g, i) in spotlight" :key="g.id" :class="{ on: i === index }"
              :aria-label="`Mettre en avant ${g.title}`" @click="select(i)" />
    </div>
  </section>
</template>

<style scoped>
.salon-hero {
  position: relative; min-height: 60vh; display: flex; flex-direction: column; justify-content: flex-end;
  padding: 44px 56px 30px; overflow: hidden; margin-top: -74px;
}
.salon-hero-art { position: absolute; inset: 0; z-index: -2; transition: opacity 0.6s; }
.salon-hero-art::after {
  content: ""; position: absolute; inset: 0;
  background: repeating-linear-gradient(115deg, rgba(255, 255, 255, 0.04) 0 2px, transparent 2px 10px);
  mix-blend-mode: overlay;
}
.salon-hero-img {
  position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center 30%; z-index: -2; display: block;
}
.salon-hero-scrim {
  position: absolute; inset: 0; z-index: -1;
  background: linear-gradient(0deg, var(--bg) 2%, rgba(10, 7, 16, 0.35) 40%, rgba(10, 7, 16, 0.55) 100%),
              linear-gradient(90deg, rgba(10, 7, 16, 0.8), transparent 60%);
}
:root[data-theme="light"] .salon-hero-scrim {
  background: linear-gradient(0deg, var(--bg) 2%, rgba(255, 255, 255, 0.1) 45%, transparent 100%),
              linear-gradient(90deg, rgba(255, 255, 255, 0.55), transparent 60%);
}
.salon-hero-eyebrow {
  font-family: var(--mono); font-size: 12px; letter-spacing: 0.16em; text-transform: uppercase;
  color: #ffd9c9; margin-bottom: 14px; display: flex; align-items: center; gap: 9px;
}
.pulse { width: 7px; height: 7px; border-radius: 50%; background: #6ee7a8; animation: pulse 2.2s infinite; }
@keyframes pulse {
  0% { box-shadow: 0 0 0 0 rgba(110, 231, 168, 0.6); }
  70% { box-shadow: 0 0 0 9px rgba(110, 231, 168, 0); }
  100% { box-shadow: 0 0 0 0 rgba(110, 231, 168, 0); }
}
.salon-hero-title {
  font-size: clamp(44px, 6vw, 76px); font-weight: 800; letter-spacing: -0.035em; margin: 0 0 14px;
  color: #fff; text-wrap: balance; line-height: 0.98; text-shadow: 0 4px 30px rgba(0, 0, 0, 0.4);
}
:root[data-theme="light"] .salon-hero-title { color: #1a1220; text-shadow: none; }
.salon-hero-meta {
  font-family: var(--mono); font-size: 14px; color: rgba(255, 255, 255, 0.88);
  display: flex; flex-wrap: wrap; gap: 11px; align-items: center; margin-bottom: 24px;
}
:root[data-theme="light"] .salon-hero-meta { color: var(--text-dim); }
.salon-hero-meta .sep { opacity: 0.4; }
.hero-actions { display: flex; gap: 10px; flex-wrap: wrap; }
:root[data-theme="light"] .salon-hero .btn-ghost {
  background: rgba(20, 15, 30, 0.08); color: var(--text); border-color: var(--border);
}
.salon-dots { display: flex; gap: 8px; margin-top: 22px; }
.salon-dots button {
  width: 26px; height: 5px; border-radius: 99px; border: none; background: rgba(255, 255, 255, 0.3);
  transition: background 0.2s, width 0.2s; padding: 0;
}
.salon-dots button.on { background: var(--accent); width: 40px; }

@media (max-width: 820px) {
  .salon-hero { padding-left: 20px; padding-right: 20px; }
}
</style>
