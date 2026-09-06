<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
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
function onImgError(url: string | null) {
  if (url) failed.value = new Set(failed.value).add(url);
}

/**
 * Jaquette à afficher : capsule Steam d'abord, puis la boxart ITAD de repli.
 * 🔑 Plus de la moitié des jeux d'une wishlist (nouveautés, jeux pas encore sortis)
 * n'ont pas de `library_600x900` sur le CDN Steam : sans ce repli, ils s'affichaient
 * en dégradé alors que leur fiche produit, elle, avait bien un visuel.
 */
function coverSrc(it: WishlistItem): string | null {
  const urls = [it.coverUrl, it.coverFallbackUrl].filter(
    (u): u is string => !!u && !failed.value.has(u),
  );
  return urls[0] ?? null;
}

/** Clic sur un jeu : fiche Boutique de Torii si connue, sinon page Steam. */
function open(it: WishlistItem) {
  if (it.gameId) openProduct(it.gameId);
  else openExternal(`https://store.steampowered.com/app/${it.appId}`);
}
/**
 * Recherche LOCALE, comme dans « En commun ». Celle du bandeau filtre la bibliotheque, or
 * une wishlist ne contient par definition que des jeux qu on ne possede PAS : elle n avait
 * aucune chance d y trouver quoi que ce soit.
 */
const query = ref("");
const visibles = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return items.value;
  return items.value.filter((it) => it.title.toLowerCase().includes(q));
});

/** Le prix actuel touche-t-il (à ~1 %) le plus bas historique ? */
function atLow(it: WishlistItem): boolean {
  return it.price != null && it.historyLow != null && it.price <= it.historyLow * 1.01;
}
</script>

<template>
  <div class="wl">
    <div class="sec-head">
      <h2>Wishlist</h2>
      <span class="n">{{ visibles.length }} jeu{{ visibles.length > 1 ? "x" : "" }}</span>
      <!-- Le compte des promos porte sur la wishlist entiere : on le cache pendant une
           recherche, ou il contredirait le nombre affiche juste a cote. -->
      <span v-if="onSaleCount && !query.trim()" class="n sale">· {{ onSaleCount }} en promo</span>
      <span v-if="loading" class="spin" title="Actualisation…" />
      <span class="spacer" />
      <input
        v-if="items.length"
        v-model="query"
        class="search"
        type="search"
        placeholder="Rechercher…"
        aria-label="Rechercher dans la wishlist"
      />
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

    <div v-else-if="!visibles.length" class="empty">
      <p>Aucun jeu de ta wishlist ne correspond à « {{ query.trim() }} ».</p>
    </div>

    <div v-else class="grid">
      <button
        v-for="it in visibles"
        :key="it.gameId || it.appId"
        class="card cover-card"
        @click="open(it)"
      >
        <div class="cover" :style="{ background: gradientFor(it.gameId || String(it.appId)) }">
          <img
            v-if="coverSrc(it)"
            :key="coverSrc(it)!"
            class="cover-img"
            :src="coverSrc(it)!"
            alt=""
            loading="lazy"
            @error="onImgError(coverSrc(it))"
          />
          <span v-if="it.savings > 0" class="badge">-{{ it.savings }}%</span>
          <span class="cover-scrim" />
          <span class="cover-title">{{ it.title || "Jeu Steam" }}</span>
        </div>
        <div class="meta">
          <div v-if="it.price != null" class="price-row">
            <span class="price" :class="{ low: atLow(it) }">{{ formatEur(it.price) }}</span>
            <span v-if="it.savings > 0 && it.normalPrice" class="was">{{ formatEur(it.normalPrice) }}</span>
            <span v-if="it.storeName" class="store">{{ it.storeName }}</span>
          </div>
          <div v-else class="price-row"><span class="soon">Pas encore d'offre</span></div>
          <div v-if="atLow(it)" class="hist at-low" title="Le prix actuel touche son plus bas historique">
            ★ Plus bas historique
          </div>
          <div v-else-if="it.historyLow != null" class="hist">
            Plus bas : {{ formatEur(it.historyLow) }}
          </div>
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

/* Même grille que la Boutique ; le fond de carte vient de `.cover-card` (style.css). */
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(172px, 1fr)); gap: 22px 20px; }

.meta { display: flex; flex-direction: column; gap: 4px; padding: 0 2px; }
.price-row { display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; }
.price { font-family: var(--mono); font-size: 15px; font-weight: 700; color: var(--text); }
.price.low { color: #3ad07f; }
.was { font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); text-decoration: line-through; }
.store { font-size: 12px; color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.soon { font-size: 12.5px; color: var(--text-faint); font-style: italic; }
.hist { font-family: var(--mono); font-size: 11px; color: var(--text-faint); }
.hist.at-low { color: #3ad07f; }

.empty { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 70px 0; text-align: center; color: var(--text-faint); font-size: 14px; }
.empty p { margin: 3px 0; }
.empty .dim { font-size: 12.5px; opacity: 0.8; }
.btn-connect { margin-top: 14px; padding: 9px 18px; border-radius: 11px; background: var(--accent); color: var(--accent-ink); border: none; font-weight: 600; font-size: 13.5px; }
.btn-connect:hover { background: var(--accent-hover); }

/* Recherche locale : meme dessin que celle de la bibliotheque d un ami — c est le meme
   geste, il doit avoir la meme tete partout. */
.search {
  width: 220px; padding: 7px 12px; border-radius: 10px;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text);
  font-size: 13px;
}
.search:focus { border-color: var(--accent); }
</style>
