<script setup lang="ts">
import { computed, ref } from "vue";
import { usePreferences } from "../composables/usePreferences";
import { t } from "../i18n";
import type { StoreBloc } from "../types";

/**
 * Description complète d'une page Steam, repliée derrière un lien. Partagée par la fiche
 * de la Boutique et celle d'un jeu de la bibliothèque.
 *
 * Les blocs sont déjà nettoyés côté Rust (`metadata/fiche.rs`) : du texte et des adresses
 * d'images Steam, jamais de HTML.
 */
defineProps<{ blocs: StoreBloc[] }>();

const { prefs } = usePreferences();
const reduit = computed(
  () =>
    prefs.reduceMotion ||
    (typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches),
);
const ouverte = ref(false);

function hideBroken(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}
</script>

<template>
  <template v-if="blocs.length">
    <button class="lien-texte" :aria-expanded="ouverte" @click="ouverte = !ouverte">
      {{ ouverte ? t("boutique.produit.masquerDescription") : t("boutique.produit.descriptionComplete") }}
      {{ ouverte ? "▴" : "▾" }}
    </button>
    <div v-if="ouverte" class="blocs">
      <template v-for="(b, i) in blocs" :key="i">
        <h3 v-if="b.genre === 'h'">{{ b.texte }}</h3>
        <p v-else-if="b.genre === 'li'" class="puce">{{ b.texte }}</p>
        <p v-else-if="b.genre === 'p'">{{ b.texte }}</p>
        <img v-else-if="b.genre === 'img' && b.url" :src="b.url" alt="" loading="lazy" @error="hideBroken" />
        <template v-else-if="b.genre === 'video' && b.url">
          <img v-if="reduit && b.affiche" :src="b.affiche" alt="" loading="lazy" @error="hideBroken" />
          <video v-else-if="!reduit" :src="b.url" :poster="b.affiche ?? undefined" muted loop autoplay playsinline preload="none" />
        </template>
      </template>
    </div>
  </template>
</template>

<style scoped>
.lien-texte { display: block; background: none; border: none; padding: 0; margin-top: 10px; color: var(--accent); font-size: 13px; font-weight: 600; }
.lien-texte:hover { text-decoration: underline; }
.blocs { display: flex; flex-direction: column; gap: 12px; margin-top: 16px; max-width: 90ch; }
.blocs h3 { margin: 10px 0 0; font-size: 15px; font-weight: 800; letter-spacing: 0.02em; color: var(--text); }
.blocs p { margin: 0; font-size: 15px; line-height: 1.7; color: var(--text-dim); white-space: pre-line; }
.blocs p.puce { padding-left: 18px; position: relative; }
.blocs p.puce::before { content: "•"; position: absolute; left: 4px; color: var(--accent); }
.blocs img, .blocs video { max-width: 100%; border-radius: 10px; display: block; }
</style>
