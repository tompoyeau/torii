<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { usePreferences } from "../composables/usePreferences";
import { t } from "../i18n";
import type { StoreSteamSheet } from "../types";

/**
 * Médias de la fiche produit : une grande vue (aperçu vidéo ou capture) et la bande des
 * miniatures. Cliquer la grande capture ouvre la visionneuse plein écran du parent.
 *
 * L'aperçu vidéo est muet et ne dure que quelques secondes (voir `metadata/fiche.rs`) :
 * il tourne en boucle, sauf si l'utilisateur a demandé moins d'animations — il reste
 * alors à l'arrêt sur son affiche, avec un bouton pour le lancer.
 */
const props = defineProps<{
  apercu?: StoreSteamSheet["apercu"];
  captures: { mini: string; grande: string }[];
}>();
const emit = defineEmits<{ agrandir: [index: number]; bandeAnnonce: [] }>();

const { prefs } = usePreferences();
const reduit = computed(
  () =>
    prefs.reduceMotion ||
    (typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches),
);

/** Position dans la bande : l'aperçu (s'il existe) d'abord, puis les captures. */
const avecApercu = computed(() => !!props.apercu);
const choisi = ref(0);
const total = computed(() => props.captures.length + (avecApercu.value ? 1 : 0));
const surApercu = computed(() => avecApercu.value && choisi.value === 0);
const capture = computed(() => props.captures[choisi.value - (avecApercu.value ? 1 : 0)] ?? null);

const video = ref<HTMLVideoElement | null>(null);
const enLecture = ref(false);
function lire() {
  enLecture.value = true;
  void nextTick(() => video.value?.play().catch(() => (enLecture.value = false)));
}

watch(
  () => [props.apercu, props.captures],
  () => {
    choisi.value = 0;
    enLecture.value = false;
    void nextTick(mesurer);
  },
);

/* Bande de miniatures défilante. */
const bande = ref<HTMLElement | null>(null);
const debut = ref(true);
const fin = ref(true);
function mesurer() {
  const el = bande.value;
  if (!el) return;
  debut.value = el.scrollLeft <= 2;
  fin.value = el.scrollLeft + el.clientWidth >= el.scrollWidth - 2;
}
function defiler(sens: number) {
  bande.value?.scrollBy({ left: sens * bande.value.clientWidth * 0.85, behavior: "smooth" });
}
</script>

<template>
  <section v-if="total" class="medias" :aria-label="t('boutique.produit.medias')">
    <div class="vue">
      <template v-if="surApercu && apercu">
        <video
          v-if="!reduit || enLecture"
          ref="video"
          :key="apercu.mp4 ?? apercu.webm ?? ''"
          :poster="apercu.affiche ?? undefined"
          muted
          loop
          playsinline
          :autoplay="!reduit"
          preload="metadata"
        >
          <source v-if="apercu.webm" :src="apercu.webm" type="video/webm" />
          <source v-if="apercu.mp4" :src="apercu.mp4" type="video/mp4" />
        </video>
        <button v-else class="affiche" :aria-label="t('boutique.produit.lireApercu')" @click="lire">
          <img v-if="apercu.affiche" :src="apercu.affiche" alt="" />
          <span class="lecture"><svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg></span>
        </button>
        <div class="legende">
          <span class="etiquette">{{ t("boutique.produit.apercu") }}</span>
          <span class="nom">{{ apercu.nom }}</span>
          <button class="btn-ghost" @click="emit('bandeAnnonce')">
            <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>{{ t("boutique.produit.bandeAnnonce") }}
          </button>
        </div>
      </template>
      <button v-else-if="capture" class="grande" @click="emit('agrandir', choisi - (avecApercu ? 1 : 0))">
        <img :src="capture.grande" alt="" />
      </button>
    </div>

    <div v-if="total > 1" class="bande-wrap">
      <button v-if="!debut" class="defile prec" :aria-label="t('fiche.capturesPrecedentes')" @click="defiler(-1)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
      <div ref="bande" class="bande no-scrollbar" @scroll.passive="mesurer">
        <button
          v-if="apercu"
          class="mini"
          :class="{ actif: choisi === 0 }"
          :aria-label="t('boutique.produit.lireApercu')"
          :aria-current="choisi === 0"
          @click="choisi = 0"
        >
          <img v-if="apercu.affiche" :src="apercu.affiche" alt="" loading="lazy" @load="mesurer" />
          <span class="lecture petite"><svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg></span>
        </button>
        <button
          v-for="(c, i) in captures"
          :key="c.mini"
          class="mini"
          :class="{ actif: choisi === i + (avecApercu ? 1 : 0) }"
          :aria-label="t('boutique.produit.afficherMedia', { n: i + 1 })"
          :aria-current="choisi === i + (avecApercu ? 1 : 0)"
          @click="choisi = i + (avecApercu ? 1 : 0)"
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
.vue video, .vue img { width: 100%; height: 100%; object-fit: cover; display: block; }
.grande, .affiche { display: block; width: 100%; height: 100%; padding: 0; border: none; background: none; cursor: zoom-in; }
.affiche { cursor: pointer; position: relative; }
.lecture {
  position: absolute; inset: 0; margin: auto; width: 64px; height: 64px; border-radius: 50%;
  display: grid; place-items: center; background: rgba(14, 10, 20, 0.7); color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.3);
}
.lecture svg { width: 26px; height: 26px; }
.lecture.petite { width: 30px; height: 30px; }
.lecture.petite svg { width: 14px; height: 14px; }
.legende {
  position: absolute; inset: auto 0 0 0; padding: 18px; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;
  background: linear-gradient(0deg, rgba(8, 5, 14, 0.88), rgba(8, 5, 14, 0)); color: #fff;
}
.etiquette { font-family: var(--mono); font-size: 11px; letter-spacing: 0.12em; text-transform: uppercase; color: #ffd9c9; }
.nom { font-size: 13px; color: rgba(255, 255, 255, 0.85); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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
