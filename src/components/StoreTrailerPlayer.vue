<script lang="ts">
/** Volume retenu d'une bande-annonce à l'autre pendant la session (commun aux lecteurs). */
const souvenir = { volume: 0.8, muet: false };
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { t } from "../i18n";

/**
 * Lecteur des bandes-annonces de la fiche produit, aux couleurs de Torii (les commandes
 * natives de la WebView sont celles de Windows, grises et étrangères au reste).
 *
 * Lecture, barre de progression (avec ce qui est déjà chargé), temps, volume, plein
 * écran. Au clavier, une fois le lecteur sélectionné : Espace ou K (lecture/pause),
 * ← → (5 s), ↑ ↓ (volume), M (son), F (plein écran).
 *
 * Les commandes s'effacent après 2,5 s sans mouvement pendant la lecture, et restent
 * affichées en pause ou quand le lecteur a le focus clavier.
 */
defineProps<{ src: string; affiche?: string | null; nom: string }>();
const emit = defineEmits<{ echec: [] }>();

const racine = ref<HTMLElement | null>(null);
const video = ref<HTMLVideoElement | null>(null);
const enPause = ref(true);
const attente = ref(true);
const temps = ref(0);
const duree = ref(0);
const charge = ref(0);
const volume = ref(souvenir.volume);
const muet = ref(souvenir.muet);
const pleinEcran = ref(false);
const visibles = ref(true);
const focusDedans = ref(false);

const pctLu = computed(() => (duree.value ? (temps.value / duree.value) * 100 : 0));
const pctCharge = computed(() => (duree.value ? (charge.value / duree.value) * 100 : 0));
const commandesVisibles = computed(() => visibles.value || enPause.value || focusDedans.value);

/** « 1:05 », « 1:02:09 ». */
function horloge(s: number): string {
  if (!Number.isFinite(s) || s < 0) s = 0;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60).toString().padStart(2, "0");
  return h ? `${h}:${m.toString().padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}

function basculer() {
  const v = video.value;
  if (!v) return;
  if (v.paused || v.ended) void v.play().catch(() => undefined);
  else v.pause();
}
function chercher(s: number) {
  const v = video.value;
  if (v && Number.isFinite(v.duration)) v.currentTime = Math.min(Math.max(0, s), v.duration);
}
function regler(niveau: number) {
  const v = video.value;
  if (!v) return;
  v.volume = Math.min(Math.max(0, niveau), 1);
  v.muted = v.volume === 0;
}
function couperSon() {
  const v = video.value;
  if (!v) return;
  v.muted = !v.muted;
  if (!v.muted && v.volume === 0) v.volume = 0.5;
}
async function basculerPleinEcran() {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await racine.value?.requestFullscreen();
  } catch {
    // Plein écran refusé par la fenêtre : on reste dans la fiche, rien de cassé.
  }
}

// --- Événements de la vidéo ---
function majTemps() {
  const v = video.value;
  if (!v) return;
  temps.value = v.currentTime;
  duree.value = Number.isFinite(v.duration) ? v.duration : 0;
  const b = v.buffered;
  charge.value = b.length ? b.end(b.length - 1) : 0;
}
function majVolume() {
  const v = video.value;
  if (!v) return;
  volume.value = v.volume;
  muet.value = v.muted;
  souvenir.volume = v.volume;
  souvenir.muet = v.muted;
}

// --- Effacement des commandes ---
let minuterie: ReturnType<typeof setTimeout> | undefined;
function reveiller() {
  visibles.value = true;
  clearTimeout(minuterie);
  minuterie = setTimeout(() => (visibles.value = false), 2500);
}

function clavier(e: KeyboardEvent) {
  const v = video.value;
  if (!v || e.ctrlKey || e.metaKey || e.altKey) return;
  const actions: Record<string, () => void> = {
    " ": basculer,
    k: basculer,
    ArrowLeft: () => chercher(v.currentTime - 5),
    ArrowRight: () => chercher(v.currentTime + 5),
    ArrowUp: () => regler(v.volume + 0.1),
    ArrowDown: () => regler(v.volume - 0.1),
    m: couperSon,
    f: () => void basculerPleinEcran(),
  };
  const action = actions[e.key.length === 1 ? e.key.toLowerCase() : e.key];
  // Les curseurs (progression, volume) gèrent eux-mêmes leurs flèches.
  if (!action || ((e.target as HTMLElement).tagName === "INPUT" && e.key.startsWith("Arrow"))) return;
  e.preventDefault();
  e.stopPropagation();
  action();
  reveiller();
}

function majPleinEcran() {
  pleinEcran.value = document.fullscreenElement === racine.value;
}

onMounted(() => {
  const v = video.value;
  if (v) {
    v.volume = volume.value;
    v.muted = muet.value;
    // Le clic sur « lecture » vient d'avoir lieu : le navigateur autorise le son.
    void v.play().catch(() => undefined);
  }
  document.addEventListener("fullscreenchange", majPleinEcran);
  reveiller();
});
onBeforeUnmount(() => {
  clearTimeout(minuterie);
  document.removeEventListener("fullscreenchange", majPleinEcran);
  if (document.fullscreenElement === racine.value) void document.exitFullscreen().catch(() => undefined);
});
</script>

<template>
  <div
    ref="racine"
    class="lecteur"
    :class="{ cache: !commandesVisibles, plein: pleinEcran }"
    tabindex="0"
    role="group"
    :aria-label="nom"
    @mousemove="reveiller"
    @focusin="focusDedans = true"
    @focusout="focusDedans = false"
    @keydown="clavier"
  >
    <video
      ref="video"
      :src="src"
      :poster="affiche ?? undefined"
      playsinline
      @click="basculer"
      @dblclick="basculerPleinEcran"
      @play="enPause = false"
      @pause="enPause = true"
      @ended="enPause = true"
      @waiting="attente = true"
      @playing="attente = false"
      @canplay="attente = false"
      @timeupdate="majTemps"
      @progress="majTemps"
      @durationchange="majTemps"
      @volumechange="majVolume"
      @error="emit('echec')"
    />

    <span v-if="attente && !enPause" class="tourne" aria-hidden="true" />
    <button v-else-if="enPause" class="centre" :aria-label="t('boutique.produit.lecteur.lire')" @click="basculer">
      <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
    </button>

    <div class="commandes" @click.stop>
      <div class="progression">
        <span class="piste" />
        <span class="charge" :style="{ width: `${pctCharge}%` }" />
        <span class="lu" :style="{ width: `${pctLu}%` }" />
        <input
          type="range"
          min="0"
          :max="duree || 0"
          step="0.1"
          :value="temps"
          :aria-label="t('boutique.produit.lecteur.position')"
          :aria-valuetext="`${horloge(temps)} / ${horloge(duree)}`"
          @input="chercher(Number(($event.target as HTMLInputElement).value))"
        />
      </div>
      <div class="rangee">
        <button class="bouton" :aria-label="enPause ? t('boutique.produit.lecteur.lire') : t('boutique.produit.lecteur.pause')" @click="basculer">
          <svg v-if="enPause" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
          <svg v-else viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1" /><rect x="14" y="5" width="4" height="14" rx="1" /></svg>
        </button>
        <div class="son">
          <button class="bouton" :aria-label="muet || volume === 0 ? t('boutique.produit.lecteur.activerSon') : t('boutique.produit.lecteur.couperSon')" @click="couperSon">
            <svg v-if="muet || volume === 0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" stroke="none" /><path d="M17 9l5 6M22 9l-5 6" /></svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" stroke="none" /><path d="M16.5 8.5a5 5 0 0 1 0 7" /><path v-if="volume > 0.5" d="M19 6a8.5 8.5 0 0 1 0 12" /></svg>
          </button>
          <input
            class="volume"
            type="range"
            min="0"
            max="1"
            step="0.05"
            :value="muet ? 0 : volume"
            :style="{ '--niveau': `${(muet ? 0 : volume) * 100}%` }"
            :aria-label="t('boutique.produit.lecteur.volume')"
            @input="regler(Number(($event.target as HTMLInputElement).value))"
          />
        </div>
        <span class="temps">{{ horloge(temps) }} <span class="sep">/</span> {{ horloge(duree) }}</span>
        <span class="titre">{{ nom }}</span>
        <button class="bouton" :aria-label="pleinEcran ? t('boutique.produit.lecteur.quitterPleinEcran') : t('boutique.produit.lecteur.pleinEcran')" @click="basculerPleinEcran">
          <svg v-if="pleinEcran" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" /></svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* `user-select: none` : un double-clic (plein écran) sélectionnait le titre de la barre. */
.lecteur { position: absolute; inset: 0; background: #000; outline: none; user-select: none; }
.lecteur:focus-visible { box-shadow: inset 0 0 0 2px var(--accent); }
.lecteur.cache { cursor: none; }
.lecteur video { width: 100%; height: 100%; object-fit: contain; display: block; }

.centre {
  position: absolute; inset: 0; margin: auto; width: 72px; height: 72px; border-radius: 50%;
  display: grid; place-items: center; background: rgba(14, 10, 20, 0.72); color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.35); transition: transform 0.15s, background 0.15s;
}
.centre:hover { transform: scale(1.06); background: var(--accent); color: var(--accent-ink); border-color: transparent; }
.centre svg { width: 30px; height: 30px; margin-left: 3px; }
.tourne {
  position: absolute; inset: 0; margin: auto; width: 42px; height: 42px; border-radius: 50%;
  border: 3px solid rgba(255, 255, 255, 0.25); border-top-color: var(--accent); animation: tourne 0.8s linear infinite;
}
@keyframes tourne { to { transform: rotate(360deg); } }

/* --- Barre de commandes --- */
.commandes {
  position: absolute; inset: auto 0 0 0; padding: 26px 14px 10px; color: #fff;
  background: linear-gradient(0deg, rgba(8, 5, 14, 0.92), rgba(8, 5, 14, 0));
  transition: opacity 0.25s;
}
.cache .commandes { opacity: 0; pointer-events: none; }
.rangee { display: flex; align-items: center; gap: 6px; }
.bouton {
  width: 36px; height: 36px; flex: none; display: grid; place-items: center; border-radius: 10px;
  background: none; border: none; color: #fff; opacity: 0.9;
}
.bouton:hover { opacity: 1; background: rgba(255, 255, 255, 0.12); }
.bouton svg { width: 20px; height: 20px; }
.son { display: flex; align-items: center; }
.temps { font-family: var(--mono); font-size: 12.5px; font-variant-numeric: tabular-nums; margin-left: 4px; white-space: nowrap; }
.temps .sep { opacity: 0.5; }
.titre { flex: 1; min-width: 0; margin: 0 8px; font-size: 12.5px; color: rgba(255, 255, 255, 0.7); text-align: right; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

/* --- Barre de progression : piste, déjà chargé, déjà lu, et un curseur natif par-dessus
   (accessible au clavier et aux lecteurs d'écran sans rien réécrire). --- */
.progression { position: relative; height: 16px; margin: 0 4px 4px; }
.piste, .charge, .lu {
  position: absolute; left: 0; top: 50%; height: 4px; transform: translateY(-50%); border-radius: 99px;
  transition: height 0.12s; pointer-events: none;
}
.piste { right: 0; background: rgba(255, 255, 255, 0.22); }
.charge { background: rgba(255, 255, 255, 0.38); }
.lu { background: var(--accent); }
.progression:hover .piste, .progression:hover .charge, .progression:hover .lu { height: 6px; }

input[type="range"] {
  -webkit-appearance: none; appearance: none; margin: 0; background: transparent; cursor: pointer;
}
.progression input { position: absolute; inset: 0; width: 100%; height: 100%; }
/* Piste invisible (le dessin est fait en dessous), seul le curseur se voit. */
input[type="range"]::-webkit-slider-runnable-track { height: 100%; background: transparent; }
input[type="range"]::-moz-range-track { height: 100%; background: transparent; }
input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none; width: 14px; height: 14px; border-radius: 50%; background: var(--accent);
  border: 2px solid #fff; margin-top: 1px; transform: scale(0); transition: transform 0.12s;
}
input[type="range"]::-moz-range-thumb {
  width: 12px; height: 12px; border-radius: 50%; background: var(--accent); border: 2px solid #fff; transform: scale(0); transition: transform 0.12s;
}
.progression:hover input::-webkit-slider-thumb, .progression input:focus-visible::-webkit-slider-thumb { transform: scale(1); }
.progression:hover input::-moz-range-thumb, .progression input:focus-visible::-moz-range-thumb { transform: scale(1); }

/* Volume : la piste porte elle-même le niveau (dégradé), le curseur reste visible.
   ⚠️ Sélecteurs `.son input.volume` : il faut battre `input[type="range"]` ci-dessus,
   plus spécifique qu'une simple classe. */
.son input.volume {
  width: 0; height: 16px; opacity: 0; transition: width 0.18s, opacity 0.18s, margin 0.18s;
}
.son:hover input.volume, .son input.volume:focus-visible { width: 76px; opacity: 1; margin: 0 8px 0 2px; }
.son input.volume::-webkit-slider-runnable-track {
  height: 4px; border-radius: 99px;
  background: linear-gradient(90deg, var(--accent) var(--niveau), rgba(255, 255, 255, 0.25) var(--niveau));
}
.son input.volume::-moz-range-track {
  height: 4px; border-radius: 99px;
  background: linear-gradient(90deg, var(--accent) var(--niveau), rgba(255, 255, 255, 0.25) var(--niveau));
}
.son input.volume::-webkit-slider-thumb { transform: scale(1); width: 12px; height: 12px; margin-top: -4px; }
.son input.volume::-moz-range-thumb { transform: scale(1); width: 10px; height: 10px; }

/* Plein écran : un peu plus grand, et l'image prend toute la place. */
.plein .commandes { padding: 40px 28px 18px; }
.plein .bouton { width: 44px; height: 44px; }
.plein .bouton svg { width: 24px; height: 24px; }
</style>
