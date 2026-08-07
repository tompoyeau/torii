<script setup lang="ts">
import type { Game } from "../types";
import { platformName } from "../data/platforms";
import { useContextMenu } from "../composables/useContextMenu";
import PlatformIcon from "./PlatformIcon.vue";

const props = defineProps<{ game: Game }>();
defineEmits<{ (e: "open"): void }>();

const { openContext } = useContextMenu();

function hideBrokenCover(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}
</script>

<template>
  <button class="tile" @click="$emit('open')" @contextmenu="openContext($event, props.game)">
    <div class="tile-art" :style="{ background: game.cover }">
      <img
        v-if="game.heroUrl"
        class="tile-img"
        :src="game.heroUrl"
        alt=""
        loading="lazy"
        @error="hideBrokenCover"
      />
      <span class="tile-plat"><PlatformIcon :platform="game.platform" /></span>
      <span class="tile-scrim" />
      <span class="tile-play">
        <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
      </span>
      <span class="tile-title">{{ game.title }}</span>
    </div>
    <div class="tile-sub">
      <span>{{ platformName(game.platform) }}</span>
      <template v-if="game.hoursPlayed != null"><span>·</span><span>{{ game.hoursPlayed }} h</span></template>
      <template v-else-if="game.sizeGb"><span>·</span><span>{{ game.sizeGb }} Go</span></template>
      <template v-if="game.genre"><span>·</span><span>{{ game.genre }}</span></template>
    </div>
  </button>
</template>

<style scoped>
.tile {
  flex: none; width: 340px; scroll-snap-align: start;
  background: none; border: none; padding: 0; text-align: left; color: inherit;
}
.tile-art {
  position: relative; aspect-ratio: 16 / 9; border-radius: 18px; overflow: hidden;
  border: 1px solid var(--border); box-shadow: var(--shadow-card);
  transition: transform 0.24s cubic-bezier(0.2, 0.7, 0.3, 1), box-shadow 0.24s, outline-color 0.2s;
  outline: 2px solid transparent; outline-offset: 3px; isolation: isolate;
}
.tile-art::before {
  content: ""; position: absolute; inset: 0; z-index: 1;
  background: repeating-linear-gradient(120deg, rgba(255, 255, 255, 0.05) 0 1px, transparent 1px 8px);
  mix-blend-mode: overlay;
}
.tile-img {
  position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 0; display: block;
}
.tile-scrim { position: absolute; inset: 0; background: linear-gradient(0deg, rgba(0, 0, 0, 0.62), transparent 55%); }
.tile-plat {
  position: absolute; top: 12px; left: 12px; width: 28px; height: 28px; border-radius: 9px;
  display: grid; place-items: center; background: rgba(12, 10, 18, 0.55); backdrop-filter: blur(6px);
  border: 1px solid rgba(255, 255, 255, 0.16); z-index: 2;
}
.tile-plat :deep(.platform-icon) { width: 16px; height: 16px; }
.tile-title {
  position: absolute; left: 16px; right: 16px; bottom: 14px; z-index: 2; color: #fff;
  font-weight: 800; font-size: 20px; letter-spacing: -0.02em; text-shadow: 0 2px 12px rgba(0, 0, 0, 0.5);
}
.tile-play {
  position: absolute; top: 12px; right: 12px; z-index: 2; width: 40px; height: 40px; border-radius: 50%;
  background: var(--accent); color: var(--accent-ink); display: grid; place-items: center;
  opacity: 0; transform: scale(0.7); transition: all 0.2s;
}
.tile-play svg { width: 18px; height: 18px; margin-left: 2px; }
.tile:hover .tile-art { transform: scale(1.045); box-shadow: 0 30px 60px -22px rgba(0, 0, 0, 0.8); outline-color: var(--accent); }
.tile:hover .tile-play { opacity: 1; transform: scale(1); }
.tile-sub {
  font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); margin-top: 10px;
  display: flex; gap: 7px;
}
</style>
