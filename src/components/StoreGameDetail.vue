<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useStore } from "../composables/useStore";
import { openExternal } from "../lib/tauri";
import { formatEur } from "../lib/format";

const { product, productLoading, selectedGameId, closeProduct } = useStore();

const open = computed(() => selectedGameId.value != null);

const price = formatEur;

/** Meilleure offre (1re du comparatif, trié par prix croissant). */
const best = computed(() => product.value?.prices[0] ?? null);

function buy(url: string) {
  openExternal(url);
}

function hideBroken(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}

function onKey(e: KeyboardEvent) {
  if (zoom.value != null) {
    // La visionneuse capte les touches avant la fiche.
    if (e.key === "Escape") zoom.value = null;
    else if (e.key === "ArrowRight") stepZoom(1);
    else if (e.key === "ArrowLeft") stepZoom(-1);
    return;
  }
  if (e.key === "Escape" && open.value) closeProduct();
}
onMounted(() => document.addEventListener("keydown", onKey));
onBeforeUnmount(() => document.removeEventListener("keydown", onKey));

/** Visionneuse plein écran des captures (index dans `shots`, sinon fermée). */
const zoom = ref<number | null>(null);
const shots = computed(() => product.value?.screenshots ?? []);
function stepZoom(delta: number) {
  const n = shots.value.length;
  if (zoom.value == null || n === 0) return;
  zoom.value = (zoom.value + delta + n) % n;
}
</script>

<template>
  <div class="sd" :class="{ open }">
    <template v-if="open">
      <div class="sd-banner">
        <div class="sd-banner-art" :style="{ background: 'linear-gradient(160deg,#241a3a,#0e0a18)' }" />
        <img v-if="product?.heroUrl" class="sd-banner-img" :src="product.heroUrl" alt="" @error="hideBroken" />
        <div class="sd-scrim" />
        <button class="sd-back" @click="closeProduct">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>Boutique
        </button>
        <div class="sd-header">
          <span class="sd-tag">Boutique</span>
          <h1 class="sd-title">{{ product?.title ?? "Chargement…" }}</h1>
          <div v-if="product" class="sd-facts">
            <span v-if="product.genre">{{ product.genre }}</span>
            <span v-if="product.developer">· {{ product.developer }}</span>
            <span v-if="product.year">· {{ product.year }}</span>
          </div>
        </div>
      </div>

      <div class="sd-body">
        <div class="sd-main">
          <div v-if="productLoading && !product" class="sd-loading">
            <span class="spin" /> Chargement de la fiche…
          </div>
          <template v-else-if="product">
            <section class="sd-sec">
              <h4>À propos</h4>
              <p v-if="product.description" class="desc">{{ product.description }}</p>
              <p v-else class="desc dim">Aucune description disponible pour ce jeu.</p>
            </section>
            <section v-if="shots.length" class="sd-sec">
              <h4>Captures d'écran</h4>
              <div class="shots no-scrollbar">
                <button v-for="(s, i) in shots" :key="i" class="shot" @click="zoom = i">
                  <img :src="s" alt="" loading="lazy" @error="hideBroken" />
                </button>
              </div>
            </section>
          </template>
        </div>

        <aside class="sd-prices">
          <div v-if="best" class="best">
            <div class="best-top">
              <span class="best-label">Meilleur prix</span>
              <span v-if="best.savings > 0" class="best-save">-{{ best.savings }}%</span>
            </div>
            <div class="best-price">{{ price(best.price) }}</div>
            <div v-if="best.savings > 0" class="best-was">au lieu de {{ price(best.retailPrice) }}</div>
            <button class="buy big" @click="buy(best.buyUrl)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6h15l-1.5 9h-12z" /><circle cx="9" cy="20" r="1.4" /><circle cx="18" cy="20" r="1.4" /><path d="M6 6 5 3H2" /></svg>
              Acheter chez {{ best.storeName }}
            </button>
            <div v-if="product?.cheapestEver != null" class="ever">
              Plus bas historique : <b>{{ price(product.cheapestEver) }}</b>
            </div>
          </div>

          <div v-if="product && product.prices.length > 1" class="compare">
            <div class="compare-label">Comparer ({{ product.prices.length }} boutiques)</div>
            <div v-for="(p, i) in product.prices" :key="p.storeName + p.buyUrl + i" class="row">
              <span class="row-store">{{ p.storeName }}</span>
              <span v-if="p.savings > 0" class="row-save">-{{ p.savings }}%</span>
              <span class="row-price">{{ price(p.price) }}</span>
              <button class="row-buy" title="Acheter" @click="buy(p.buyUrl)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
              </button>
            </div>
          </div>

          <p class="disclaimer">Prix indicatifs en euros (CheapShark convertis au taux du jour ; Instant Gaming natif). L'achat se fait sur la boutique du marchand.</p>
        </aside>
      </div>

      <div v-if="zoom != null && shots[zoom]" class="lightbox" @click.self="zoom = null">
        <button class="lb-close" aria-label="Fermer" @click="zoom = null">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
        <button v-if="shots.length > 1" class="lb-nav prev" aria-label="Capture précédente" @click.stop="stepZoom(-1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
        </button>
        <img class="lb-img" :src="shots[zoom]" alt="" @error="hideBroken" />
        <button v-if="shots.length > 1" class="lb-nav next" aria-label="Capture suivante" @click.stop="stepZoom(1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
        </button>
        <div v-if="shots.length > 1" class="lb-count">{{ zoom + 1 }} / {{ shots.length }}</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.sd { position: fixed; inset: 0; z-index: 110; background: var(--bg); overflow-y: auto; opacity: 0; visibility: hidden; transition: opacity 0.28s; }
.sd.open { opacity: 1; visibility: visible; }
.sd-banner { position: relative; height: 40vh; min-height: 300px; overflow: hidden; }
.sd-banner-art { position: absolute; inset: 0; }
.sd-banner-img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center 25%; display: block; }
.sd-scrim { position: absolute; inset: 0; background: linear-gradient(0deg, var(--bg) 1%, rgba(10, 7, 16, 0.25) 45%, rgba(10, 7, 16, 0.55) 100%); }
.sd-back {
  position: absolute; top: 22px; left: 24px; z-index: 5; display: inline-flex; align-items: center; gap: 8px;
  padding: 9px 16px 9px 12px; border-radius: 11px; border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(12, 8, 18, 0.5); backdrop-filter: blur(8px); color: #fff; font-weight: 600; font-size: 13.5px;
}
.sd-back svg { width: 17px; height: 17px; }
.sd-header { position: absolute; bottom: 0; left: 0; right: 0; padding: 0 56px 26px; }
.sd-tag {
  display: inline-block; font-family: var(--mono); font-size: 11px; letter-spacing: 0.1em; text-transform: uppercase;
  color: #fff; padding: 4px 11px; border-radius: 99px; background: var(--accent); margin-bottom: 12px;
}
.sd-title { font-size: clamp(30px, 4.5vw, 52px); font-weight: 800; letter-spacing: -0.035em; margin: 0; color: #fff; line-height: 1; text-shadow: 0 3px 24px rgba(0, 0, 0, 0.4); }
.sd-facts { margin-top: 12px; font-family: var(--mono); font-size: 13px; color: rgba(255, 255, 255, 0.85); display: flex; gap: 6px; }

.sd-body { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 40px; padding: 34px 56px 70px; align-items: start; }
.sd-main { min-width: 0; }
.sd-loading { display: flex; align-items: center; gap: 10px; color: var(--text-faint); padding: 30px 0; }
.sd-sec { margin-bottom: 34px; }
.sd-sec h4 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint); font-weight: 700; margin: 0 0 16px; }
.desc { font-size: 15.5px; line-height: 1.7; color: var(--text-dim); max-width: 90ch; }
.desc.dim { color: var(--text-faint); font-style: italic; }
.shots { display: flex; gap: 14px; overflow-x: auto; padding-bottom: 8px; }
.shot { flex: none; width: 300px; aspect-ratio: 16 / 9; border-radius: 13px; overflow: hidden; border: 1px solid var(--border); padding: 0; cursor: zoom-in; background: none; }
.shot img { width: 100%; height: 100%; object-fit: cover; display: block; }

.sd-prices { position: sticky; top: 24px; display: flex; flex-direction: column; gap: 16px; }
.best { background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 20px; }
.best-top { display: flex; align-items: center; justify-content: space-between; }
.best-label { font-size: 12px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; }
.best-save { font-family: var(--mono); font-weight: 700; font-size: 13px; color: #fff; background: linear-gradient(135deg, #3ad07f, #1fa862); padding: 3px 9px; border-radius: 8px; }
.best-price { font-family: var(--mono); font-size: 40px; font-weight: 700; letter-spacing: -0.03em; color: var(--text); margin-top: 8px; line-height: 1; }
.best-was { font-size: 13px; color: var(--text-faint); margin-top: 6px; }
.buy { margin-top: 16px; width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 9px; padding: 12px; border-radius: 12px; font-size: 14.5px; font-weight: 700; background: var(--accent); color: var(--accent-ink); border: none; }
.buy:hover { background: var(--accent-hover); }
.buy svg { width: 17px; height: 17px; }
.ever { margin-top: 14px; font-size: 12.5px; color: var(--text-faint); text-align: center; }
.ever b { color: var(--text-dim); font-family: var(--mono); }

.compare { background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 14px 16px; }
.compare-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; padding: 4px 0 10px; }
.row { display: flex; align-items: center; gap: 10px; padding: 10px 0; border-top: 1px solid var(--border); }
.row-store { font-size: 13.5px; color: var(--text); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.row-save { font-family: var(--mono); font-size: 10.5px; color: #1fa862; }
.row-price { font-family: var(--mono); font-size: 14px; font-weight: 600; color: var(--text); font-variant-numeric: tabular-nums; }
.row-buy { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim); flex: none; }
.row-buy:hover { background: var(--accent); color: var(--accent-ink); border-color: transparent; }
.row-buy svg { width: 15px; height: 15px; }
.disclaimer { font-size: 11px; color: var(--text-faint); line-height: 1.5; margin: 4px 2px 0; }

.spin { width: 18px; height: 18px; border-radius: 50%; border: 2px solid var(--border-strong); border-top-color: var(--accent); animation: spin 0.7s linear infinite; display: inline-block; }
@keyframes spin { to { transform: rotate(360deg); } }

.lightbox { position: fixed; inset: 0; z-index: 300; display: grid; place-items: center; padding: 4vh 5vw; background: rgba(6, 4, 10, 0.86); backdrop-filter: blur(6px); cursor: zoom-out; }
.lb-img { max-width: 100%; max-height: 92vh; border-radius: 12px; object-fit: contain; box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6); }
.lb-close { position: fixed; top: 20px; right: 22px; width: 42px; height: 42px; display: grid; place-items: center; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.18); background: rgba(14, 10, 20, 0.55); color: #fff; }
.lb-close svg { width: 20px; height: 20px; }
.lb-nav {
  position: fixed; top: 50%; transform: translateY(-50%); width: 48px; height: 48px; display: grid; place-items: center;
  border-radius: 50%; border: 1px solid rgba(255, 255, 255, 0.18); background: rgba(14, 10, 20, 0.55); color: #fff; cursor: pointer;
}
.lb-nav:hover { background: rgba(40, 30, 55, 0.85); border-color: rgba(255, 255, 255, 0.35); }
.lb-nav.prev { left: 20px; }
.lb-nav.next { right: 20px; }
.lb-nav svg { width: 24px; height: 24px; }
.lb-count {
  position: fixed; bottom: 22px; left: 50%; transform: translateX(-50%); font-family: var(--mono); font-size: 13px; color: #fff;
  background: rgba(14, 10, 20, 0.6); padding: 6px 12px; border-radius: 99px; border: 1px solid rgba(255, 255, 255, 0.14);
}

@media (max-width: 980px) {
  .sd-body { grid-template-columns: minmax(0, 1fr); }
  .sd-prices { position: static; }
}
@media (max-width: 820px) {
  .sd-header, .sd-body { padding-left: 22px; padding-right: 22px; }
}
</style>
