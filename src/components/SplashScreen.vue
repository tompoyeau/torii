<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { usePreferences } from "../composables/usePreferences";

const { booted } = useLibrary();
const { prefs } = usePreferences();

/**
 * Durée minimale d'affichage : la bibliothèque du dernier scan s'affiche en quelques
 * dizaines de millisecondes (cache disque), donc sans ce plancher l'ouverture ne serait
 * qu'un clignotement. C'est le temps que la porte met à se construire.
 */
const MIN_MS = 1700;
/** Animations réduites : on ne retient personne, juste le temps d'un fondu. */
const MIN_MS_SOBRE = 350;

const tempsEcoule = ref(false);
onMounted(() => {
  setTimeout(() => (tempsEcoule.value = true), prefs.reduceMotion ? MIN_MS_SOBRE : MIN_MS);
});

// L'écran s'efface quand la bibliothèque est prête ET que la séquence est allée au bout.
const visible = computed(() => !booted.value || !tempsEcoule.value);
</script>

<template>
  <transition name="splash">
    <div v-if="visible" class="splash" :class="{ sobre: prefs.reduceMotion }">
      <!-- Halos d'ambiance : deux masses de couleur qui respirent derrière la porte. -->
      <div class="halo halo-a" />
      <div class="halo halo-b" />

      <div class="scene">
        <div class="mark">
          <div class="glow" />
          <svg viewBox="0 0 24 24" fill="#1a0f0c">
            <!-- Les piliers montent du sol… -->
            <g class="piliers">
              <path d="M6.6 7.3 L8.7 7.3 L9.1 19.6 L6.2 19.6 Z" />
              <path d="M15.3 7.3 L17.4 7.3 L17.8 19.6 L14.9 19.6 Z" />
            </g>
            <!-- …les traverses s'ouvrent depuis le centre… -->
            <g class="traverses">
              <rect x="11.2" y="8.9" width="1.6" height="2.6" />
              <rect x="4.5" y="11.1" width="15" height="2.1" rx="0.4" />
            </g>
            <!-- …et le linteau se pose en dernier, comme sur un vrai chantier. -->
            <g class="linteau">
              <path d="M2.5 5 Q12 7.5 21.5 5 L21.5 7.4 Q12 9.9 2.5 7.4 Z" />
              <path d="M2.5 5 L1.3 4 L0.9 5.7 L2.5 7.4 Z" />
              <path d="M21.5 5 L22.7 4 L23.1 5.7 L21.5 7.4 Z" />
            </g>
          </svg>
          <!-- Éclat qui traverse la tuile une fois la porte dressée. -->
          <div class="sweep" />
        </div>

        <div class="name">Torii</div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.splash {
  position: fixed; inset: 0; z-index: 400;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  background: var(--bg); background-image: var(--bg-grad);
  overflow: hidden;
}
/* Vignette : referme l'image sur la porte, comme un rideau de scène. */
.splash::after {
  content: ""; position: absolute; inset: 0; pointer-events: none;
  background: radial-gradient(120% 90% at 50% 45%, transparent 35%, rgba(0, 0, 0, 0.42) 100%);
}

.halo {
  position: absolute; border-radius: 50%; filter: blur(70px); opacity: 0;
  animation: halo-entre 1.1s cubic-bezier(0.2, 0.7, 0.3, 1) forwards, derive 9s ease-in-out 1.1s infinite alternate;
}
.halo-a {
  width: 460px; height: 460px; top: 12%; left: 14%;
  background: radial-gradient(circle, var(--accent), transparent 68%);
}
.halo-b {
  width: 400px; height: 400px; bottom: 8%; right: 12%;
  background: radial-gradient(circle, #7b45f0, transparent 68%);
  animation-delay: 0.15s, 1.25s;
}

.scene { position: relative; display: flex; flex-direction: column; align-items: center; gap: 22px; }

.mark {
  position: relative; width: 108px; height: 108px; border-radius: 30px;
  background: linear-gradient(140deg, var(--accent), #ff9a6b);
  display: grid; place-items: center; overflow: hidden;
  box-shadow: 0 22px 60px -18px var(--accent);
  animation: tuile 0.75s cubic-bezier(0.16, 1.16, 0.3, 1) both;
}
.mark svg { width: 62px; height: 62px; grid-area: 1 / 1; }

/* Lueur qui bat derrière la porte, une fois celle-ci dressée. */
.glow {
  position: absolute; inset: -30%; border-radius: 50%;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.55), transparent 62%);
  opacity: 0; animation: souffle 2.6s ease-in-out 0.85s infinite;
}

/* --- Le chantier : chaque pièce arrive à son tour ------------------------- */
.piliers path, .traverses rect, .linteau path {
  transform-box: fill-box;
}
.piliers path {
  transform-origin: bottom center;
  animation: monte 0.55s cubic-bezier(0.2, 0.9, 0.25, 1) both;
}
.piliers path:nth-child(2) { animation-delay: 0.27s; }
.piliers path:nth-child(1) { animation-delay: 0.20s; }

.traverses rect {
  transform-origin: center;
  animation: ouvre 0.42s cubic-bezier(0.2, 0.9, 0.25, 1) both;
}
.traverses rect:nth-child(1) { animation-delay: 0.46s; }
.traverses rect:nth-child(2) { animation-delay: 0.52s; }

.linteau path {
  transform-origin: center;
  animation: pose 0.5s cubic-bezier(0.16, 1.3, 0.4, 1) 0.66s both;
}

/* Éclat diagonal : la lumière passe sous la porte une fois qu'elle tient debout. */
.sweep {
  position: absolute; inset: -60% -120%;
  background: linear-gradient(
    68deg,
    transparent 42%,
    rgba(255, 255, 255, 0.6) 50%,
    transparent 58%
  );
  transform: translateX(-100%);
  animation: balaye 0.7s cubic-bezier(0.4, 0, 0.2, 1) 0.92s both;
}

.name {
  font-size: 26px; font-weight: 700; color: var(--text);
  animation: signature 0.7s cubic-bezier(0.2, 0.7, 0.3, 1) 0.85s both;
}


@keyframes tuile {
  from { transform: scale(0.62) translateY(14px); opacity: 0; }
  to { transform: scale(1) translateY(0); opacity: 1; }
}
@keyframes monte {
  from { transform: translateY(9px) scaleY(0.2); opacity: 0; }
  to { transform: translateY(0) scaleY(1); opacity: 1; }
}
@keyframes ouvre {
  from { transform: scaleX(0.05); opacity: 0; }
  to { transform: scaleX(1); opacity: 1; }
}
@keyframes pose {
  from { transform: translateY(-11px) scale(0.9); opacity: 0; }
  to { transform: translateY(0) scale(1); opacity: 1; }
}
@keyframes balaye {
  from { transform: translateX(-100%); }
  to { transform: translateX(100%); }
}
@keyframes signature {
  from { opacity: 0; letter-spacing: 0.42em; transform: translateY(6px); }
  to { opacity: 1; letter-spacing: -0.02em; transform: translateY(0); }
}
@keyframes souffle {
  0%, 100% { opacity: 0.16; transform: scale(0.92); }
  50% { opacity: 0.36; transform: scale(1.06); }
}
@keyframes halo-entre { to { opacity: 0.5; } }
@keyframes derive {
  from { transform: translate3d(0, 0, 0) scale(1); }
  to { transform: translate3d(26px, -18px, 0) scale(1.12); }
}

/* --- Animations réduites : la même image, sans le chantier ---------------- */
.splash.sobre .halo,
.splash.sobre .glow,
.splash.sobre .sweep { animation: none; opacity: 0.28; }
.splash.sobre .glow, .splash.sobre .sweep { opacity: 0; }
.splash.sobre .mark,
.splash.sobre .piliers path,
.splash.sobre .traverses rect,
.splash.sobre .linteau path,
.splash.sobre .name { animation: none; opacity: 1; transform: none; }

/* Sortie : la porte s'ouvre — un pas en avant, et l'écran s'efface. */
.splash-leave-active { transition: opacity 0.45s ease, transform 0.45s cubic-bezier(0.4, 0, 0.2, 1); }
.splash-leave-to { opacity: 0; transform: scale(1.06); }
.splash.sobre.splash-leave-to { transform: none; }
</style>
