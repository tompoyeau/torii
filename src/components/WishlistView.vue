<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useWishlist } from "../composables/useWishlist";
import { useStore } from "../composables/useStore";
import { useUi } from "../composables/useUi";
import { openExternal } from "../lib/tauri";
import { formatEur } from "../lib/format";
import { gradientFor } from "../lib/covers";
import type { WishlistItem } from "../types";

const { items, loading, loaded, steamConnected, onSaleCount, refresh } = useWishlist();
const { openProduct } = useStore();
const { openSettings } = useUi();

onMounted(() => {
  if (!loaded.value) void refresh();
});

const failed = ref(new Set<string>());
function onImgError(url: string) {
  failed.value = new Set(failed.value).add(url);
}
function coverOk(it: WishlistItem): boolean {
  return !!it.coverUrl && !failed.value.has(it.coverUrl);
}

/** Clic sur un jeu : fiche Boutique de Torii si connue, sinon page Steam. */
function open(it: WishlistItem) {
  if (it.gameId) openProduct(it.gameId);
  else openExternal(`https://store.steampowered.com/app/${it.appId}`);
}
/** Le prix actuel touche-t-il (à ~1 %) le plus bas historique ? */
function atLow(it: WishlistItem): boolean {
  return it.price != null && it.historyLow != null && it.price <= it.historyLow * 1.01;
}
</script>

<template>
  <div class="wl">
    <div class="sec-head">
      <h2>Wishlist</h2>
      <span class="n">{{ items.length }} jeu{{ items.length > 1 ? "x" : "" }}</span>
      <span v-if="onSaleCount" class="n sale">· {{ onSaleCount }} en promo</span>
      <span v-if="loading" class="spin" title="Actualisation…" />
      <span class="spacer" />
      <button class="chip refresh" :disabled="loading" title="Actualiser" @click="refresh()">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
        Actualiser
      </button>
    </div>

    <!-- Steam non connecté -->
    <div v-if="loaded && !steamConnected" class="empty">
      <p>Connecte ton compte Steam pour suivre les prix de ta wishlist.</p>
      <button class="btn-connect" @click="openSettings()">Ouvrir les réglages</button>
    </div>

    <!-- Chargement initial -->
    <div v-else-if="!loaded && loading" class="empty">
      <span class="spin big" />
      <p>Récupération des prix de ta wishlist…</p>
      <p class="dim">Quelques secondes la première fois.</p>
    </div>

    <!-- Vide -->
    <div v-else-if="loaded && !items.length" class="empty">
      <p>Ta wishlist Steam est vide.</p>
    </div>

    <div v-else class="grid">
      <button v-for="it in items" :key="it.appId || it.gameId" class="card" @click="open(it)">
        <div class="cover" :style="{ background: gradientFor(String(it.appId)) }">
          <img v-if="coverOk(it)" :src="it.coverUrl" alt="" loading="lazy" @error="onImgError(it.coverUrl)" />
          <span v-else class="cover-title">{{ it.title || "Jeu Steam" }}</span>
          <span v-if="it.savings > 0" class="badge-sale">-{{ it.savings }}%</span>
          <span v-if="atLow(it)" class="badge-low" title="Prix au plus bas historique">★ bas historique</span>
        </div>
        <div class="meta">
          <span class="title">{{ it.title || "Jeu Steam" }}</span>
          <div v-if="it.price != null" class="price-row">
            <span class="price">{{ formatEur(it.price) }}</span>
            <span v-if="it.savings > 0 && it.normalPrice" class="was">{{ formatEur(it.normalPrice) }}</span>
            <span class="store">{{ it.storeName }}</span>
          </div>
          <div v-else class="price-row"><span class="soon">Pas encore d'offre</span></div>
          <div v-if="it.historyLow != null" class="low">Plus bas : {{ formatEur(it.historyLow) }}</div>
        </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.wl { min-width: 0; }
.sec-head { display: flex; align-items: center; gap: 10px; margin-bottom: 20px; }
.sec-head h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.sec-head .n { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
.sec-head .n.sale { color: #3ad07f; }
.sec-head .spacer { flex: 1; }
.chip.refresh {
  display: inline-flex; align-items: center; gap: 6px; padding: 6px 13px; border-radius: 99px; font-size: 12.5px;
  color: var(--text-dim); background: var(--surface); border: 1px solid var(--border);
}
.chip.refresh svg { width: 14px; height: 14px; }
.chip.refresh:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); }
.chip.refresh:disabled { opacity: 0.6; }
.spin { width: 14px; height: 14px; border-radius: 50%; border: 2px solid var(--border-strong); border-top-color: var(--accent); animation: spin 0.7s linear infinite; display: inline-block; }
.spin.big { width: 26px; height: 26px; border-width: 3px; margin-bottom: 14px; }
@keyframes spin { to { transform: rotate(360deg); } }

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(155px, 1fr)); gap: 22px 16px; }
.card { display: flex; flex-direction: column; gap: 9px; background: none; border: none; padding: 0; cursor: pointer; text-align: left; color: inherit; min-width: 0; }
.cover {
  position: relative; aspect-ratio: 3 / 4; border-radius: 12px; overflow: hidden; border: 1px solid var(--border);
  display: grid; place-items: center; transition: transform 0.15s, box-shadow 0.15s;
}
.card:hover .cover { transform: translateY(-3px); box-shadow: 0 12px 28px rgba(0, 0, 0, 0.4); }
.cover img { width: 100%; height: 100%; object-fit: cover; }
.cover-title { padding: 10px; font-size: 14px; font-weight: 700; text-align: center; color: #fff; text-shadow: 0 1px 4px rgba(0,0,0,0.5); }
.badge-sale {
  position: absolute; top: 8px; left: 8px; font-family: var(--mono); font-size: 12px; font-weight: 700; color: #fff;
  background: linear-gradient(135deg, #3ad07f, #1fa862); padding: 3px 8px; border-radius: 8px;
}
.badge-low {
  position: absolute; bottom: 8px; left: 8px; right: 8px; text-align: center; font-size: 10.5px; font-weight: 700; color: #fff;
  background: rgba(12, 10, 18, 0.72); backdrop-filter: blur(4px); padding: 3px 6px; border-radius: 7px;
}
.meta { display: flex; flex-direction: column; gap: 4px; }
.title { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.price-row { display: flex; align-items: baseline; gap: 7px; flex-wrap: wrap; }
.price { font-family: var(--mono); font-size: 14px; font-weight: 700; color: var(--text); }
.was { font-family: var(--mono); font-size: 11px; color: var(--text-faint); text-decoration: line-through; }
.store { font-size: 11px; color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.soon { font-size: 12px; color: var(--text-faint); font-style: italic; }
.low { font-family: var(--mono); font-size: 10.5px; color: var(--text-faint); }

.empty { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 70px 0; text-align: center; color: var(--text-faint); font-size: 14px; }
.empty p { margin: 3px 0; }
.empty .dim { font-size: 12.5px; opacity: 0.8; }
.btn-connect { margin-top: 14px; padding: 9px 18px; border-radius: 11px; background: var(--accent); color: var(--accent-ink); border: none; font-weight: 600; font-size: 13.5px; }
.btn-connect:hover { background: var(--accent-hover); }
</style>
