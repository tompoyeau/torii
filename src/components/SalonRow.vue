<script setup lang="ts">
import { ref } from "vue";
import type { Game } from "../types";
import { useUi } from "../composables/useUi";
import SalonTile from "./SalonTile.vue";

/** `activeCol` = index de la tuile focus clavier/manette dans cette rangée (-1 = aucune). */
const props = withDefaults(defineProps<{ title: string; games: Game[]; activeCol?: number }>(), {
  activeCol: -1,
});

const { openGame } = useUi();
const scroller = ref<HTMLElement | null>(null);

function scroll(dir: number) {
  scroller.value?.scrollBy({ left: 720 * dir, behavior: "smooth" });
}
</script>

<template>
  <div class="row">
    <div class="row-head">
      <h3>{{ title }}</h3>
      <span class="n">{{ games.length }}</span>
      <div class="arrows">
        <button class="row-arrow" aria-label="Précédent" @click="scroll(-1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
        </button>
        <button class="row-arrow" aria-label="Suivant" @click="scroll(1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
        </button>
      </div>
    </div>
    <div ref="scroller" class="row-scroller no-scrollbar">
      <SalonTile
        v-for="(g, ci) in games"
        :key="g.id"
        :game="g"
        :focused="ci === props.activeCol"
        @open="openGame(g.id)"
      />
    </div>
  </div>
</template>

<style scoped>
.row-head { display: flex; align-items: center; gap: 12px; padding: 0 56px 14px; }
.row-head h3 { font-size: 22px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.row-head .n { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
.row-head .arrows { margin-left: auto; display: flex; gap: 8px; }
.row-arrow {
  width: 36px; height: 36px; border-radius: 50%; border: 1px solid var(--border);
  background: var(--surface); color: var(--text-dim); display: grid; place-items: center; transition: all 0.15s;
}
.row-arrow:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-2); }
.row-arrow svg { width: 18px; height: 18px; }
.row-scroller { display: flex; gap: 20px; overflow-x: auto; scroll-behavior: smooth; padding: 8px 56px 20px; }

@media (max-width: 820px) {
  .row-head { padding-left: 20px; padding-right: 20px; }
  .row-scroller { padding-left: 20px; padding-right: 20px; }
}
</style>
