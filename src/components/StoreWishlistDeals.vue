<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useStore } from "../composables/useStore";
import { useUi } from "../composables/useUi";
import { useWishlist } from "../composables/useWishlist";
import { formatPrix } from "../lib/format";
import { t } from "../i18n";
import type { WishlistItem } from "../types";
import { ouvrirPage } from "../composables/usePageWeb";

/**
 * « Ta wishlist en promo », en haut de la Boutique : les jeux suivis dont la meilleure
 * offre est en promotion. Mêmes données que la page Wishlist (`useWishlist`), chargées
 * une fois par session : rien n'est demandé de plus si la page a déjà été ouverte.
 * Rien ne s'affiche tant qu'aucun jeu n'est en promo — ni titre vide, ni chargement.
 */
const { items, loaded, loading, refresh } = useWishlist();
const { openProduct } = useStore();
const { showWishlist } = useUi();

onMounted(() => {
  if (!loaded.value && !loading.value) void refresh();
});

const enPromo = computed(() =>
  items.value
    .filter((i) => i.price != null && i.savings > 0)
    .sort((a, b) => b.savings - a.savings)
    .slice(0, 8),
);

function ouvrir(i: WishlistItem) {
  if (i.gameId) void openProduct(i.gameId);
  else if (i.appId) ouvrirPage(`https://store.steampowered.com/app/${i.appId}`);
}

/** Capsule Steam, sinon jaquette de repli (beaucoup de jeux récents n'ont pas la première). */
function onErreur(e: Event, i: WishlistItem) {
  const img = e.target as HTMLImageElement;
  if (i.coverFallbackUrl && img.src !== i.coverFallbackUrl) img.src = i.coverFallbackUrl;
  else img.style.visibility = "hidden";
}
</script>

<template>
  <section v-if="enPromo.length" class="wl">
    <div class="tete">
      <h2>{{ t("boutique.accueil.wishlist.titre") }} <span class="n">· {{ enPromo.length }}</span></h2>
      <button class="lien" @click="showWishlist()">{{ t("boutique.accueil.wishlist.voir") }}</button>
    </div>
    <div class="bande">
      <button v-for="i in enPromo" :key="i.gameId || i.appId" class="jeu" @click="ouvrir(i)">
        <span class="mini"><img :src="i.coverUrl || i.coverFallbackUrl || ''" alt="" loading="lazy" @error="onErreur($event, i)" /></span>
        <span class="texte">
          <span class="nom">{{ i.title }}</span>
          <span class="prix">
            <span class="remise">-{{ i.savings }}%</span>
            <span class="now hot">{{ formatPrix(i.price!, i.currency) }}</span>
          </span>
        </span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.wl {
  padding: 14px 16px 16px; border-radius: 16px; background: var(--accent-soft);
  border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
}
.tete { display: flex; align-items: baseline; gap: 12px; margin-bottom: 10px; }
h2 { margin: 0; font-size: 15px; font-weight: 800; }
.n { font-family: var(--mono); font-weight: 400; color: var(--text-faint); }
.lien { margin-left: auto; background: none; border: none; color: var(--accent); font-size: 13px; font-weight: 600; padding: 0; }
.lien:hover { text-decoration: underline; }
/* Barre de défilement : celle de Torii (`style.css`). Surtout pas de `scrollbar-width` ici,
   qui ferait revenir la barre grise de Windows (voir le commentaire de `style.css`). */
.bande { display: grid; grid-auto-flow: column; grid-auto-columns: minmax(220px, 1fr); gap: 10px; overflow-x: auto; padding-bottom: 4px; }
.jeu {
  display: flex; align-items: center; gap: 10px; padding: 6px; border-radius: 10px; min-width: 0;
  background: var(--surface); border: 1px solid var(--border); color: inherit; text-align: left; cursor: pointer;
}
.jeu:hover { border-color: var(--border-strong); }
.mini { width: 40px; aspect-ratio: 2 / 3; flex: none; border-radius: 6px; overflow: hidden; background: var(--surface-3); }
.mini img { width: 100%; height: 100%; object-fit: cover; display: block; }
.texte { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
.nom { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
