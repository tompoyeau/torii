<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { usePreferences } from "../composables/usePreferences";
import { useStore } from "../composables/useStore";
import { useToriiWishlist } from "../composables/useToriiWishlist";
import { libelleAvis, pageSteam, tonAvis } from "../lib/boutique";
import { formatPrix } from "../lib/format";
import { useImageCascade } from "../lib/images";
import { openExternal } from "../lib/tauri";
import { t } from "../i18n";
import type { StoreShelfItem } from "../types";

/**
 * Vedette de la Boutique : grands jeux populaires en promo, un à la fois.
 *
 * Défile seule toutes les 8 s, sauf si la souris est dessus, si le focus clavier y est,
 * si la fenêtre est cachée, ou si l'utilisateur a demandé moins d'animations (Windows ou
 * réglage de Torii) : un contenu qui bouge sous les yeux de quelqu'un qui lit est la
 * première chose qu'un lecteur d'écran ou un utilisateur sensible au mouvement subit.
 */
const props = defineProps<{ jeux: StoreShelfItem[] }>();

const DUREE_MS = 8000;

const { openProduct } = useStore();
const { isWishlisted, toggle } = useToriiWishlist();
const { prefs } = usePreferences();

const index = ref(0);
const survol = ref(false);
const focus = ref(false);
const cache = ref(typeof document !== "undefined" && document.hidden);

const mouvementReduit = computed(
  () =>
    prefs.reduceMotion ||
    (typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches),
);
const enPause = computed(() => survol.value || focus.value || cache.value || mouvementReduit.value);

const jeu = computed(() => props.jeux[index.value] ?? props.jeux[0] ?? null);
watch(
  () => props.jeux.length,
  (n) => {
    if (index.value >= n) index.value = 0;
  },
);

const { src, onError } = useImageCascade(() => [jeu.value?.heroUrl, jeu.value?.headerUrl]);
const prix = (montant: number) => formatPrix(montant, jeu.value?.currency);

function aller(i: number) {
  const n = props.jeux.length;
  if (n) index.value = (i + n) % n;
}

// Minuterie : une avance par intervalle, relancée à chaque changement de jeu pour que
// cliquer sur une vignette redonne 8 s pleines au jeu choisi.
let minuterie: ReturnType<typeof setTimeout> | undefined;
function programmer() {
  clearTimeout(minuterie);
  if (enPause.value || props.jeux.length < 2) return;
  minuterie = setTimeout(() => aller(index.value + 1), DUREE_MS);
}
watch([index, enPause, () => props.jeux.length], programmer);

function visibilite() {
  cache.value = document.hidden;
}
onMounted(() => {
  document.addEventListener("visibilitychange", visibilite);
  programmer();
});
onBeforeUnmount(() => {
  clearTimeout(minuterie);
  document.removeEventListener("visibilitychange", visibilite);
});

function ouvrir() {
  const j = jeu.value;
  if (!j) return;
  if (j.gameId) void openProduct(j.gameId);
  else void openExternal(pageSteam(j.appId));
}

function quitterFocus(e: FocusEvent) {
  const vers = e.relatedTarget as Node | null;
  if (!vers || !(e.currentTarget as HTMLElement).contains(vers)) focus.value = false;
}
</script>

<template>
  <section
    v-if="jeu"
    class="vedette"
    :aria-label="t('boutique.accueil.vedette.titre')"
    aria-roledescription="carousel"
    @mouseenter="survol = true"
    @mouseleave="survol = false"
    @focusin="focus = true"
    @focusout="quitterFocus"
  >
    <div class="art" />
    <img v-if="src" :key="src" class="image" :src="src" alt="" @error="onError" />
    <div class="voile" />

    <!-- Annoncé seulement quand le défilement est arrêté (motif « carousel » du W3C) :
         sinon un lecteur d'écran relirait la vedette toutes les 8 s. -->
    <div class="contenu" :aria-live="enPause ? 'polite' : 'off'">
      <div class="surtitre">{{ t("boutique.accueil.vedette.surtitre") }}</div>
      <h2 class="titre">{{ jeu.title }}</h2>
      <p v-if="jeu.description" class="desc">{{ jeu.description }}</p>
      <div class="infos">
        <span v-for="tag in jeu.tags" :key="tag" class="tag">{{ tag }}</span>
        <span
          v-if="jeu.reviewScore"
          class="avis"
          :class="tonAvis(jeu.reviewScore)"
          :title="t('boutique.accueil.carte.avisAide', { n: jeu.reviewCount })"
        >
          {{ libelleAvis(jeu.reviewScore) }}<template v-if="jeu.reviewPercent != null"> · {{ jeu.reviewPercent }} %</template>
        </span>
      </div>
      <div v-if="jeu.price != null" class="offre">
        <span v-if="jeu.savings > 0" class="remise grande">-{{ jeu.savings }}%</span>
        <span v-if="jeu.savings > 0 && jeu.normalPrice" class="avant">{{ prix(jeu.normalPrice) }}</span>
        <span class="maintenant">{{ prix(jeu.price) }}</span>
        <span v-if="jeu.storeName" class="chez">{{ t("boutique.accueil.vedette.chez", { boutique: jeu.storeName }) }}</span>
        <span v-if="jeu.record" class="record sur-image">{{ t("boutique.accueil.carte.record") }}</span>
      </div>
      <div class="boutons">
        <button class="btn-play" @click="ouvrir">
          {{ jeu.gameId ? t("boutique.accueil.vedette.comparer") : t("boutique.accueil.vedette.voirSteam") }}
        </button>
        <button
          v-if="jeu.gameId"
          class="btn-ghost"
          :aria-pressed="isWishlisted(jeu.gameId)"
          @click="toggle({ gameId: jeu.gameId, title: jeu.title, coverUrl: jeu.capsuleUrl })"
        >
          <svg viewBox="0 0 24 24" :fill="isWishlisted(jeu.gameId) ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 20s-7-4.3-7-9.3A3.7 3.7 0 0 1 12 8a3.7 3.7 0 0 1 7 2.7c0 5-7 9.3-7 9.3Z" /></svg>
          {{ isWishlisted(jeu.gameId) ? t("boutique.produit.dansWishlist") : t("boutique.ajouterWishlist") }}
        </button>
      </div>
    </div>

    <div v-if="jeux.length > 1" class="navigation">
      <button class="rond" :aria-label="t('boutique.accueil.vedette.precedent')" @click="aller(index - 1)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
      <button
        v-for="(j, i) in jeux"
        :key="j.appId"
        class="vignette"
        :class="{ active: i === index }"
        :aria-label="t('boutique.accueil.vedette.aller', { titre: j.title })"
        :aria-current="i === index"
        @click="aller(i)"
      >
        <img v-if="j.headerUrl" :src="j.headerUrl" alt="" loading="lazy" />
        <span v-if="i === index" class="progression" :class="{ pause: enPause }" :style="{ animationDuration: `${DUREE_MS}ms` }" />
      </button>
      <button class="rond" :aria-label="t('boutique.accueil.vedette.suivant')" @click="aller(index + 1)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
      </button>
    </div>
  </section>
</template>

<style scoped>
.vedette {
  position: relative; border-radius: 22px; overflow: hidden; min-height: 400px;
  display: flex; flex-direction: column; justify-content: space-between;
  box-shadow: var(--shadow-hero); border: 1px solid var(--border); isolation: isolate;
}
.art {
  position: absolute; inset: 0; z-index: -3;
  background: radial-gradient(120% 140% at 82% 18%, #ff7a4d 0%, #b0288a 34%, #3a1c6e 66%, #14102a 100%);
}
.image { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: 70% center; z-index: -2; display: block; }
.voile {
  position: absolute; inset: 0; z-index: -1;
  background:
    linear-gradient(90deg, rgba(8, 5, 14, 0.94) 0%, rgba(8, 5, 14, 0.78) 34%, rgba(8, 5, 14, 0.1) 68%, transparent 100%),
    linear-gradient(0deg, rgba(8, 5, 14, 0.75) 0%, transparent 35%);
}
.contenu { padding: 34px 38px 0; max-width: 520px; display: flex; flex-direction: column; gap: 12px; color: #fff; }
.surtitre {
  font-family: var(--mono); font-size: 11px; letter-spacing: 0.16em; text-transform: uppercase; color: #ffd9c9;
}
.titre { margin: 0; font-size: 38px; font-weight: 800; letter-spacing: -0.03em; line-height: 1.05; text-wrap: balance; }
.desc {
  margin: 0; font-size: 14px; line-height: 1.5; color: rgba(255, 255, 255, 0.82);
  display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;
}
.infos { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font-size: 12px; }
.tag { padding: 3px 9px; border-radius: 6px; background: rgba(255, 255, 255, 0.12); color: rgba(255, 255, 255, 0.92); }
.infos .avis { color: rgba(255, 255, 255, 0.82); margin-left: 4px; }
.offre { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-top: 4px; }
.remise.grande { font-size: 15px; padding: 5px 10px; }
.avant { font-family: var(--mono); font-size: 13px; color: rgba(255, 255, 255, 0.6); text-decoration: line-through; }
.maintenant { font-family: var(--mono); font-size: 26px; font-weight: 700; font-variant-numeric: tabular-nums; }
.chez { font-size: 13px; color: rgba(255, 255, 255, 0.75); }
.record.sur-image { color: #ffb4a6; }
.boutons { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 6px; }

.navigation {
  display: flex; align-items: center; justify-content: flex-end; gap: 10px; flex-wrap: wrap;
  padding: 20px 24px 22px;
}
.rond {
  width: 36px; height: 36px; border-radius: 50%; display: grid; place-items: center; flex: none;
  border: 1px solid rgba(255, 255, 255, 0.22); background: rgba(14, 10, 20, 0.7); color: #fff;
}
.rond:hover { background: rgba(28, 20, 38, 0.92); }
.rond svg { width: 16px; height: 16px; }
.vignette {
  position: relative; width: 120px; aspect-ratio: 460 / 215; padding: 0; border-radius: 8px; overflow: hidden;
  border: 2px solid transparent; background: rgba(14, 10, 20, 0.6); opacity: 0.6; transition: opacity 0.15s, border-color 0.15s;
}
.vignette img { width: 100%; height: 100%; object-fit: cover; display: block; }
.vignette:hover { opacity: 0.9; }
.vignette.active { opacity: 1; border-color: var(--accent); }
.progression {
  position: absolute; left: 0; bottom: 0; height: 3px; width: 100%; background: var(--accent);
  transform-origin: left; animation: progression linear forwards;
}
.progression.pause { animation-play-state: paused; }
@keyframes progression { from { transform: scaleX(0); } to { transform: scaleX(1); } }
/* Les règles globales « réduire les animations » ne coupent pas une barre déjà pleine : on la cache. */
@media (prefers-reduced-motion: reduce) { .progression { display: none; } }
:root[data-reduce-motion] .progression { display: none; }

@media (max-width: 760px) {
  .contenu { padding: 24px 22px 0; }
  .titre { font-size: 30px; }
  .vignette { width: 84px; }
}
</style>
