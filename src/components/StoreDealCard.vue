<script setup lang="ts">
import { computed } from "vue";
import { usePossession } from "../composables/usePossession";
import { useStore } from "../composables/useStore";
import { useToriiWishlist } from "../composables/useToriiWishlist";
import { libelleFin } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { useImageCascade } from "../lib/images";
import { t } from "../i18n";
import type { StoreItem } from "../types";
import StorePossede from "./StorePossede.vue";

/** Carte paysage d'une offre du comparateur (carrousel « Promos du moment »). */
const props = defineProps<{ offre: StoreItem }>();

const { openProduct } = useStore();
const { possession } = usePossession();
const { isWishlisted, toggle } = useToriiWishlist();

const etat = computed(() => possession(props.offre.title));
const fin = computed(() => libelleFin(props.offre.expiry));
const prix = (montant: number) => formatPrix(montant, props.offre.currency);
const { src, onError } = useImageCascade(() => [props.offre.bannerUrl, props.offre.coverUrl]);
</script>

<template>
  <button class="carte-paysage" @click="openProduct(offre.gameId)">
    <span class="visuel" :style="{ background: gradientFor(offre.gameId) }">
      <img v-if="src" :key="src" :src="src" alt="" loading="lazy" @error="onError" />
      <span v-if="offre.savings > 0" class="remise coin-droit">-{{ offre.savings }}%</span>
      <span v-if="etat" class="coin"><StorePossede :etat="etat" /></span>
      <span
        v-else
        class="wish-dot"
        :class="{ on: isWishlisted(offre.gameId) }"
        role="button"
        :aria-label="isWishlisted(offre.gameId) ? t('boutique.retirerWishlist') : t('boutique.ajouterWishlist')"
        :title="isWishlisted(offre.gameId) ? t('boutique.retirerWishlist') : t('boutique.ajouterWishlist')"
        @click.stop="toggle({ gameId: offre.gameId, title: offre.title, coverUrl: offre.coverUrl })"
      >
        <svg viewBox="0 0 24 24" :fill="isWishlisted(offre.gameId) ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 20s-7-4.3-7-9.3A3.7 3.7 0 0 1 12 8a3.7 3.7 0 0 1 7 2.7c0 5-7 9.3-7 9.3Z" /></svg>
      </span>
      <span v-if="offre.record" class="bandeau">{{ t("boutique.accueil.carte.record") }}</span>
      <span v-else-if="fin" class="bandeau">{{ fin }}</span>
    </span>
    <span class="titre">{{ offre.title }}</span>
    <span class="prix">
      <span class="now" :class="{ hot: offre.savings > 0 }">{{ prix(offre.price) }}</span>
      <span v-if="offre.savings > 0" class="was">{{ prix(offre.normalPrice) }}</span>
      <span class="shop">{{ offre.storeName }}</span>
    </span>
  </button>
</template>

<style scoped>
.carte-paysage {
  display: flex; flex-direction: column; gap: 8px; min-width: 0; width: 100%; color: inherit;
  background: none; border: none; padding: 0; text-align: left; cursor: pointer;
}
.visuel {
  position: relative; display: block; aspect-ratio: 600 / 344; border-radius: 14px; overflow: hidden;
  border: 1px solid var(--border); box-shadow: var(--shadow-card);
  transition: transform 0.22s cubic-bezier(0.2, 0.7, 0.3, 1);
}
.visuel img { width: 100%; height: 100%; object-fit: cover; display: block; }
.carte-paysage:hover .visuel { transform: translateY(-4px); }
.coin { position: absolute; top: 9px; left: 9px; z-index: 3; }
.coin-droit { position: absolute; top: 9px; right: 9px; z-index: 2; }
.bandeau {
  position: absolute; left: 9px; bottom: 9px; z-index: 2; font-size: 11.5px; font-weight: 600;
  padding: 3px 8px; border-radius: 7px; background: rgba(14, 10, 20, 0.82); color: #fff;
}
.titre { font-size: 14px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
