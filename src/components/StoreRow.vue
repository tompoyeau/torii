<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { t } from "../i18n";

/**
 * Rangée horizontale de cartes (carrousel) : flèches de défilement, accroche aux cartes.
 * Les cartes restent des éléments focalisables ordinaires : la tabulation fait défiler.
 */
const props = withDefaults(
  defineProps<{
    titre: string;
    sousTitre?: string;
    /** Largeur d'une carte : `portrait` (jaquette) ou `paysage` (bannière). */
    format?: "portrait" | "paysage";
    /** Nombre de cartes, pour réévaluer les flèches quand la liste change. */
    nombre: number;
  }>(),
  { format: "portrait", sousTitre: undefined },
);
defineSlots<{ default(): unknown; actions?(): unknown; avant?(): unknown }>();

const piste = ref<HTMLElement | null>(null);
const debut = ref(true);
const fin = ref(true);

function mesurer() {
  const el = piste.value;
  if (!el) return;
  debut.value = el.scrollLeft <= 2;
  fin.value = el.scrollLeft + el.clientWidth >= el.scrollWidth - 2;
}

function defiler(sens: 1 | -1) {
  const el = piste.value;
  if (!el) return;
  el.scrollBy({ left: sens * el.clientWidth * 0.9, behavior: "smooth" });
}

let obs: ResizeObserver | null = null;
onMounted(() => {
  mesurer();
  if (piste.value && typeof ResizeObserver !== "undefined") {
    obs = new ResizeObserver(mesurer);
    obs.observe(piste.value);
  }
});
onBeforeUnmount(() => obs?.disconnect());
watch(
  () => props.nombre,
  () => nextTick(mesurer),
);
</script>

<template>
  <section class="rangee">
    <div class="rayon-tete">
      <div>
        <h2>{{ titre }}</h2>
        <p v-if="sousTitre" class="sous">{{ sousTitre }}</p>
      </div>
      <div class="actions">
        <slot name="actions" />
        <button class="fleche" :disabled="debut" :aria-label="t('boutique.accueil.precedent')" @click="defiler(-1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
        </button>
        <button class="fleche" :disabled="fin" :aria-label="t('boutique.accueil.suivant')" @click="defiler(1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
        </button>
      </div>
    </div>
    <slot name="avant" />
    <div ref="piste" class="piste" :class="format" @scroll.passive="mesurer">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.piste {
  display: grid; grid-auto-flow: column; gap: 20px; overflow-x: auto; overscroll-behavior-x: contain;
  scroll-snap-type: x mandatory; scrollbar-width: none;
  /* Marge interne : l'ombre et le soulèvement des cartes au survol ne sont pas rognés. */
  padding: 8px 4px 18px; margin: -8px -4px -18px;
}
.piste::-webkit-scrollbar { display: none; }
.piste.portrait { grid-auto-columns: 172px; }
.piste.paysage { grid-auto-columns: 280px; }
.piste > :deep(*) { scroll-snap-align: start; }
</style>
