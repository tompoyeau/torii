<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useUi } from "../composables/useUi";
import { useWishlist } from "../composables/useWishlist";
import { t } from "../i18n";
import type { StoreItem } from "../types";
import StoreDealCard from "./StoreDealCard.vue";
import StoreRow from "./StoreRow.vue";

/**
 * « Ta wishlist en promo » : un rayon de la Boutique comme les autres (mêmes cartes paysage
 * que « Promos du moment », juste en dessous), avec les jeux suivis dont la meilleure offre
 * est en promotion. Mêmes données que la page Wishlist (`useWishlist`), chargées une fois par
 * session. Rien ne s'affiche tant qu'aucun jeu n'est en promo — ni titre vide, ni chargement.
 */
const { items, loaded, loading, refresh } = useWishlist();
const { showWishlist } = useUi();

onMounted(() => {
  if (!loaded.value && !loading.value) void refresh();
});

/**
 * Les jeux en promo, au format des offres de la Boutique. Seuls ceux qu'ITAD connaît
 * (`gameId`) ont une fiche à ouvrir : sans lui, pas de promo suivie de toute façon.
 * La bannière : l'en-tête Steam quand on a l'appId, sinon la jaquette (repli de la carte).
 */
const enPromo = computed<StoreItem[]>(() =>
  items.value
    .filter((i) => i.gameId && i.price != null && i.savings > 0)
    .sort((a, b) => b.savings - a.savings)
    .slice(0, 12)
    .map((i) => ({
      gameId: i.gameId,
      title: i.title,
      // Capsule Steam ; la jaquette ITAD de repli suit d'elle-même (`useImagesWishlist`).
      coverUrl: i.coverUrl || null,
      bannerUrl: i.appId ? `https://cdn.cloudflare.steamstatic.com/steam/apps/${i.appId}/header.jpg` : null,
      price: i.price!,
      normalPrice: i.normalPrice ?? i.price!,
      currency: i.currency,
      savings: i.savings,
      storeName: i.storeName,
      buyUrl: i.buyUrl,
      record: i.historyLow != null && i.price! <= i.historyLow,
    })),
);
</script>

<template>
  <StoreRow
    v-if="enPromo.length"
    :titre="t('boutique.accueil.wishlist.titre')"
    format="paysage"
    :nombre="enPromo.length"
  >
    <template #actions>
      <button class="lien" @click="showWishlist()">{{ t("boutique.accueil.wishlist.voir") }}</button>
    </template>
    <StoreDealCard v-for="d in enPromo" :key="d.gameId" :offre="d" />
  </StoreRow>
</template>

<style scoped>
.lien { background: none; border: none; padding: 0; color: var(--accent); font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
.lien:hover { text-decoration: underline; }
</style>
