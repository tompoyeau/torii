<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { usePreferences } from "../composables/usePreferences";
import { hlsLisible, nomBandeAnnonce } from "../lib/boutique";
import { t } from "../i18n";
import type { StoreTrailer } from "../types";
import StoreTrailerPlayer from "./StoreTrailerPlayer.vue";

/**
 * Médias de la fiche produit : une grande vue et la bande des miniatures, bandes-annonces
 * d'abord, puis captures. Cliquer une grande capture ouvre la visionneuse du parent.
 *
 * Une bande-annonce au repos montre son aperçu muet en boucle (ou son affiche, si
 * l'utilisateur a demandé moins d'animations) sous un bouton de lecture. Le clic lance
 * la vraie bande-annonce, AVEC le son et les commandes du lecteur : jamais de son sans
 * geste de l'utilisateur.
 *
 * Repli : si la WebView ne lit pas le HLS, ou si le flux échoue, l'aperçu muet est joué à
 * la place, avec un mot d'explication et le lien vers Steam.
 */
const props = defineProps<{
  bandesAnnonces: StoreTrailer[];
  captures: { mini: string; grande: string }[];
}>();
const emit = defineEmits<{ agrandir: [index: number]; steam: [] }>();

const { prefs } = usePreferences();
const reduit = computed(
  () =>
    prefs.reduceMotion ||
    (typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches),
);
const fluxLisible = hlsLisible();

const nbBandes = computed(() => props.bandesAnnonces.length);
const total = computed(() => nbBandes.value + props.captures.length);
const choisi = ref(0);
const bande = computed(() => (choisi.value < nbBandes.value ? props.bandesAnnonces[choisi.value] : null));
const capture = computed(() => (bande.value ? null : props.captures[choisi.value - nbBandes.value] ?? null));
const aApercu = (b: StoreTrailer) => !!(b.mp4 || b.webm);

/** `repos` : affiche ou aperçu muet ; `lecture` : le flux avec le son ; `repli` : l'aperçu, faute de flux. */
const etat = ref<"repos" | "lecture" | "repli">("repos");
const video = ref<HTMLVideoElement | null>(null);

function lire() {
  const b = bande.value;
  if (!b) return;
  etat.value = b.hls && fluxLisible ? "lecture" : "repli";
  void nextTick(() => video.value?.play().catch(() => undefined));
}
function echecFlux() {
  if (etat.value === "lecture") etat.value = "repli";
}

function choisir(i: number) {
  choisi.value = i;
  etat.value = "repos";
}

watch(
  () => [props.bandesAnnonces, props.captures],
  () => {
    choisir(0);
    void nextTick(mesurer);
  },
);

/* Bande de miniatures défilante. */
const bandeEl = ref<HTMLElement | null>(null);
const debut = ref(true);
const fin = ref(true);
function mesurer() {
  const el = bandeEl.value;
  if (!el) return;
  debut.value = el.scrollLeft <= 2;
  fin.value = el.scrollLeft + el.clientWidth >= el.scrollWidth - 2;
}
function defiler(sens: number) {
  bandeEl.value?.scrollBy({ left: sens * bandeEl.value.clientWidth * 0.85, behavior: "smooth" });
}
</script>

<template>
  <section v-if="total" class="medias" :aria-label="t('boutique.produit.medias')">
    <div class="vue">
      <template v-if="bande">
        <!-- La vraie bande-annonce, avec le son et les commandes de Torii. -->
        <StoreTrailerPlayer
          v-if="etat === 'lecture' && bande.hls"
          :key="`flux-${choisi}`"
          :src="bande.hls"
          :affiche="bande.affiche"
          :nom="nomBandeAnnonce(bande.nom, choisi + 1)"
          @echec="echecFlux"
        />
        <!-- Aperçu muet : en boucle au repos (sauf animations réduites), ou en repli. -->
        <video
          v-else-if="aApercu(bande) && (etat === 'repli' || !reduit)"
          ref="video"
          :key="`apercu-${choisi}`"
          :poster="bande.affiche ?? undefined"
          muted
          loop
          autoplay
          playsinline
          preload="metadata"
        >
          <source v-if="bande.webm" :src="bande.webm" type="video/webm" />
          <source v-if="bande.mp4" :src="bande.mp4" type="video/mp4" />
        </video>
        <img v-else-if="bande.affiche" :src="bande.affiche" alt="" />

        <button
          v-if="etat === 'repos'"
          class="lecture"
          :aria-label="t('boutique.produit.lireBandeAnnonce', { nom: nomBandeAnnonce(bande.nom, choisi + 1) })"
          @click="lire"
        >
          <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
        </button>
        <div v-if="etat !== 'lecture'" class="legende">
          <span class="nom">{{ etat === "repli" ? t("boutique.produit.lectureImpossible") : nomBandeAnnonce(bande.nom, choisi + 1) }}</span>
          <button class="btn-ghost" @click="emit('steam')">
            <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>{{ t("boutique.produit.bandeAnnonce") }}
          </button>
        </div>
      </template>
      <button v-else-if="capture" class="grande" @click="emit('agrandir', choisi - nbBandes)">
        <img :src="capture.grande" alt="" />
      </button>
    </div>

    <div v-if="total > 1" class="bande-wrap">
      <button v-if="!debut" class="defile prec" :aria-label="t('fiche.capturesPrecedentes')" @click="defiler(-1)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
      <div ref="bandeEl" class="bande no-scrollbar" @scroll.passive="mesurer">
        <button
          v-for="(b, i) in bandesAnnonces"
          :key="`b${i}`"
          class="mini"
          :class="{ actif: choisi === i }"
          :aria-label="nomBandeAnnonce(b.nom, i + 1)"
          :aria-current="choisi === i"
          :title="nomBandeAnnonce(b.nom, i + 1)"
          @click="choisir(i)"
        >
          <img v-if="b.affiche" :src="b.affiche" alt="" loading="lazy" @load="mesurer" />
          <span class="lecture petite"><svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg></span>
        </button>
        <button
          v-for="(c, i) in captures"
          :key="c.mini"
          class="mini"
          :class="{ actif: choisi === i + nbBandes }"
          :aria-label="t('boutique.produit.afficherMedia', { n: i + 1 })"
          :aria-current="choisi === i + nbBandes"
          @click="choisir(i + nbBandes)"
        >
          <img :src="c.mini" alt="" loading="lazy" @load="mesurer" />
        </button>
      </div>
      <button v-if="!fin" class="defile suiv" :aria-label="t('fiche.capturesSuivantes')" @click="defiler(1)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
      </button>
    </div>
  </section>
</template>

<style scoped>
.medias { display: flex; flex-direction: column; gap: 12px; }
.vue {
  position: relative; aspect-ratio: 16 / 9; border-radius: 16px; overflow: hidden;
  border: 1px solid var(--border); background: #0e0a14;
}
.vue > video, .vue > img { width: 100%; height: 100%; object-fit: cover; display: block; }
.grande { display: block; width: 100%; height: 100%; padding: 0; border: none; background: none; cursor: zoom-in; }
.grande img { width: 100%; height: 100%; object-fit: cover; display: block; }
.lecture {
  position: absolute; inset: 0; margin: auto; width: 72px; height: 72px; border-radius: 50%; z-index: 2;
  display: grid; place-items: center; background: rgba(14, 10, 20, 0.72); color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.35); transition: transform 0.15s, background 0.15s;
}
button.lecture:hover { transform: scale(1.06); background: var(--accent); color: var(--accent-ink); border-color: transparent; }
.lecture svg { width: 30px; height: 30px; margin-left: 3px; }
.lecture.petite { width: 30px; height: 30px; pointer-events: none; }
.lecture.petite svg { width: 14px; height: 14px; margin-left: 2px; }
.legende {
  position: absolute; inset: auto 0 0 0; padding: 18px; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;
  background: linear-gradient(0deg, rgba(8, 5, 14, 0.88), rgba(8, 5, 14, 0)); color: #fff; z-index: 1;
}
.nom { font-size: 13.5px; font-weight: 600; color: rgba(255, 255, 255, 0.92); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.legende .btn-ghost { margin-left: auto; }

.bande-wrap { position: relative; }
.bande { display: flex; gap: 10px; overflow-x: auto; padding: 3px; }
.mini {
  position: relative; flex: none; width: 150px; aspect-ratio: 16 / 9; padding: 0; border-radius: 9px; overflow: hidden;
  border: 2px solid transparent; background: var(--surface-2); opacity: 0.7; transition: opacity 0.15s, border-color 0.15s;
}
.mini img { width: 100%; height: 100%; object-fit: cover; display: block; }
.mini:hover { opacity: 1; }
.mini.actif { opacity: 1; border-color: var(--accent); }
.defile {
  position: absolute; top: 50%; transform: translateY(-50%); z-index: 2; width: 34px; height: 34px; border-radius: 50%;
  display: grid; place-items: center; background: rgba(14, 10, 20, 0.7); border: 1px solid rgba(255, 255, 255, 0.25); color: #fff;
}
.defile svg { width: 17px; height: 17px; }
.defile.prec { left: 4px; }
.defile.suiv { right: 4px; }
</style>
