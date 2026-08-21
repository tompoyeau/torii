<script setup lang="ts">
import { computed, ref } from "vue";
import type { Game } from "../types";
import { platformName } from "../data/platforms";
import { useLibrary } from "../composables/useLibrary";
import { useContextMenu } from "../composables/useContextMenu";
import PlatformIcon from "./PlatformIcon.vue";

const props = defineProps<{ game: Game }>();
defineEmits<{ (e: "open"): void }>();

const { setHidden, setFavorite } = useLibrary();
const { openContext } = useContextMenu();

/**
 * Image de la carte, en cascade : jaquette portrait (`coverUrl`) → à défaut visuel
 * paysage (`heroUrl`, recadré) → à défaut rien (le dégradé reprend la main). Certains
 * jeux Steam n'ont pas de jaquette 600x900 mais ont un hero → on l'utilise plutôt qu'un dégradé.
 */
const failed = ref(new Set<string>());
const coverSrc = computed(() => {
  for (const url of [props.game.coverUrl, props.game.heroUrl]) {
    if (url && !failed.value.has(url)) return url;
  }
  return null;
});
function onCoverError() {
  if (coverSrc.value) failed.value = new Set(failed.value).add(coverSrc.value);
}

/** Nombre de copies du jeu dans le groupe familial Steam (≥2 = plusieurs copies). */
const familyCopies = computed(() => props.game.familyOwners?.length ?? 0);

/** Bascule le jeu dans/hors la liste d'exclusion (sans ouvrir le détail). */
function toggleHidden() {
  setHidden(props.game.id, !props.game.hidden);
}

/** Épingle/retire le jeu des favoris (sans ouvrir le détail). */
function toggleFavorite() {
  setFavorite(props.game.id, !props.game.favorite);
}
</script>

<template>
  <button class="card cover-card" @click="$emit('open')" @contextmenu="openContext($event, game)">
    <div class="cover" :class="{ uninstalled: !game.installed }" :style="{ background: game.cover }">
      <img
        v-if="coverSrc"
        :key="coverSrc"
        class="cover-img"
        :src="coverSrc"
        alt=""
        loading="lazy"
        @error="onCoverError"
      />
      <span class="cover-plat"><PlatformIcon :platform="game.platform" /></span>
      <div class="cover-actions">
        <button
          class="cover-act fav"
          :class="{ on: game.favorite }"
          :title="game.favorite ? 'Retirer des favoris' : 'Ajouter aux favoris'"
          :aria-label="game.favorite ? 'Retirer des favoris' : 'Ajouter aux favoris'"
          :aria-pressed="game.favorite"
          @click.stop="toggleFavorite"
        >
          <svg viewBox="0 0 24 24" :fill="game.favorite ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 4.5l2.3 4.7 5.2.8-3.8 3.7.9 5.1L12 16.9l-4.6 2.4.9-5.1L4.5 10l5.2-.8z" /></svg>
        </button>
        <button
          class="cover-act"
          :title="game.hidden ? 'Réafficher' : 'Masquer ce jeu'"
          :aria-label="game.hidden ? 'Réafficher' : 'Masquer ce jeu'"
          @click.stop="toggleHidden"
        >
          <svg v-if="game.hidden" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" /><circle cx="12" cy="12" r="3" /></svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 3l18 18M10.6 10.7a2 2 0 0 0 2.8 2.8" /><path d="M9.4 5.2A9.3 9.3 0 0 1 12 5c5 0 9 4.5 9 7a12 12 0 0 1-2.2 3M6.1 6.2A12.7 12.7 0 0 0 3 12c0 2.5 4 7 9 7a9.4 9.4 0 0 0 3.6-.7" /></svg>
        </button>
      </div>
      <span class="cover-scrim" />
      <span v-if="familyCopies >= 2" class="cover-fam" :title="`${familyCopies} copies dans ta famille Steam`">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="8" r="3" /><path d="M3.5 19a5.5 5.5 0 0 1 11 0" /><path d="M16 6a3 3 0 0 1 0 5.6M17.5 19a5.5 5.5 0 0 0-2.5-4.3" /></svg>
        {{ familyCopies }}
      </span>
      <span class="cover-title">{{ game.title }}</span>
      <span class="cover-hover">
        <span class="cover-play">
          <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
        </span>
      </span>
    </div>
    <div class="card-meta">
      <span class="card-name">{{ game.title }}</span>
      <span class="card-sub">
        <span>{{ game.sources && game.sources.length > 1
          ? game.sources.map((s) => platformName(s.platform)).join(" · ")
          : platformName(game.platform) }}</span>
        <template v-if="game.hoursPlayed != null"><span>·</span><span>{{ game.hoursPlayed }} h</span></template>
        <span>·</span>
        <span :class="game.installed ? 'installed' : 'not-installed'">
          {{ game.installed ? "Installé" : "Non installé" }}
        </span>
      </span>
    </div>
  </button>
</template>

<style scoped>
/* Le fond de carte (rayon, ombre, survol, jaquette, voile, titre) vient de
   `.cover-card` dans style.css — partagé avec la Boutique et la Wishlist. Ici,
   uniquement ce qui est propre à la bibliothèque. */
.cover::before {
  content: ""; position: absolute; inset: 0; z-index: 1;
  background: repeating-linear-gradient(125deg, rgba(255, 255, 255, 0.06) 0 1px, transparent 1px 7px);
  mix-blend-mode: overlay; opacity: 0.6;
}
.cover-plat {
  position: absolute; top: 11px; left: 11px; z-index: 2; width: 26px; height: 26px;
  border-radius: 8px; display: grid; place-items: center;
  background: rgba(12, 10, 18, 0.55); backdrop-filter: blur(6px); border: 1px solid rgba(255, 255, 255, 0.16);
}
.cover-plat :deep(.platform-icon) { width: 15px; height: 15px; }
.cover-actions {
  position: absolute; top: 9px; right: 9px; z-index: 4; display: flex; gap: 6px;
}
.cover-act {
  width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; cursor: pointer;
  background: rgba(12, 10, 18, 0.6); backdrop-filter: blur(6px);
  border: 1px solid rgba(255, 255, 255, 0.16); color: #fff;
  opacity: 0; transform: scale(0.9); transition: opacity 0.18s, transform 0.18s, background 0.15s, color 0.15s;
}
.cover-act svg { width: 15px; height: 15px; }
.cover-act:hover { background: rgba(40, 30, 55, 0.85); border-color: rgba(255, 255, 255, 0.35); }
.card:hover .cover-act { opacity: 1; transform: scale(1); }
/* Une étoile épinglée reste visible même hors survol, en couleur d'accent. */
.cover-act.fav.on { opacity: 1; transform: scale(1); color: var(--accent); border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
.cover-act.fav.on:hover { background: color-mix(in srgb, var(--accent) 22%, rgba(12, 10, 18, 0.6)); }
.cover-hover {
  position: absolute; inset: 0; z-index: 3; display: grid; place-items: center;
  background: rgba(12, 8, 20, 0.42); backdrop-filter: blur(2px); opacity: 0; transition: opacity 0.2s;
}
.cover-play {
  width: 54px; height: 54px; border-radius: 50%; background: var(--accent); color: var(--accent-ink);
  display: grid; place-items: center; transform: scale(0.8); transition: transform 0.2s;
  box-shadow: 0 10px 24px -8px rgba(0, 0, 0, 0.6);
}
.cover-play svg { width: 22px; height: 22px; margin-left: 2px; }
.card:hover .cover-hover { opacity: 1; }
.card:hover .cover-play { transform: scale(1); }
.card-meta { display: flex; flex-direction: column; gap: 3px; padding: 0 2px; }
.card-name { font-size: 13.5px; font-weight: 600; letter-spacing: -0.01em; }
.card-sub {
  font-family: var(--mono); font-size: 11px; color: var(--text-faint);
  display: flex; align-items: center; gap: 6px; font-variant-numeric: tabular-nums;
}
.card-sub .installed { color: var(--manual); }
.card-sub .not-installed { color: var(--text-faint); }
.cover.uninstalled { filter: saturate(0.85) brightness(0.9); }
.card:hover .cover.uninstalled { filter: none; }
/* Badge « copies famille » (bas-droite de la cover) */
.cover-fam {
  position: absolute; right: 10px; bottom: 12px; z-index: 2; display: inline-flex; align-items: center; gap: 4px;
  padding: 2px 7px 2px 6px; border-radius: 99px; font-family: var(--mono); font-size: 11px; font-weight: 700; color: #fff;
  background: rgba(12, 10, 18, 0.62); backdrop-filter: blur(6px); border: 1px solid rgba(255, 255, 255, 0.18);
}
.cover-fam svg { width: 13px; height: 13px; }
</style>
