<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useStore, type StoreSort } from "../composables/useStore";
import { gradientFor } from "../lib/covers";
import { formatEur } from "../lib/format";
import type { StoreItem, StoreSuggestion } from "../types";

const {
  items, loading, sort, query, activeQuery, suggestions,
  setSort, runSearch, openProduct, fetchSuggestions, clearSuggestions,
} = useStore();

const SORTS: { key: StoreSort; label: string }[] = [
  { key: "featured", label: "Mises en avant" },
  { key: "savings", label: "Meilleures remises" },
  { key: "price", label: "Prix croissant" },
  { key: "recent", label: "Récents" },
  { key: "rating", label: "Mieux notés" },
];

// --- Autocomplétion ---
const suggestOpen = ref(false);
const activeIndex = ref(-1);
let debounce: ReturnType<typeof setTimeout> | undefined;

/** Saisie : ouvre le menu et rafraîchit les suggestions (débruité). */
function onInput() {
  suggestOpen.value = true;
  activeIndex.value = -1;
  clearTimeout(debounce);
  debounce = setTimeout(() => fetchSuggestions(query.value), 220);
}

/** Sélectionne une suggestion → ouvre directement sa fiche produit. */
function pick(s: StoreSuggestion) {
  suggestOpen.value = false;
  openProduct(s.gameId);
}

/** Lance la recherche complète (grille) et ferme le menu. */
function submitSearch() {
  suggestOpen.value = false;
  runSearch();
}

function closeSuggest() {
  suggestOpen.value = false;
  activeIndex.value = -1;
}

function onSearchKey(e: KeyboardEvent) {
  const list = suggestions.value;
  if (e.key === "ArrowDown" && suggestOpen.value && list.length) {
    e.preventDefault();
    activeIndex.value = (activeIndex.value + 1) % list.length;
  } else if (e.key === "ArrowUp" && suggestOpen.value && list.length) {
    e.preventDefault();
    activeIndex.value = activeIndex.value <= 0 ? list.length - 1 : activeIndex.value - 1;
  } else if (e.key === "Enter") {
    if (suggestOpen.value && activeIndex.value >= 0 && list[activeIndex.value]) {
      pick(list[activeIndex.value]);
    } else {
      submitSearch();
    }
  } else if (e.key === "Escape") {
    closeSuggest();
  }
}
function clearSearch() {
  query.value = "";
  clearSuggestions();
  suggestOpen.value = false;
  runSearch();
}

// Fermeture au clic en dehors de la barre de recherche.
function onDocClick(e: MouseEvent) {
  if (!(e.target as HTMLElement).closest(".store-search-wrap")) closeSuggest();
}
onMounted(() => document.addEventListener("click", onDocClick));
onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
  clearTimeout(debounce);
});

const price = formatEur;

/** Jaquette ITAD si fournie et non cassée, sinon dégradé (géré côté template). */
const failed = ref(new Set<string>());
function coverSrc(it: StoreItem): string | null {
  const url = it.coverUrl || null;
  return url && !failed.value.has(url) ? url : null;
}
function onCoverError(url: string | null) {
  if (url) failed.value = new Set(failed.value).add(url);
}
</script>

<template>
  <div class="store">
    <div class="store-head">
      <div class="store-title">
        <h2>Boutique</h2>
        <p class="sub">Découvre et compare les prix sur toutes les boutiques PC.</p>
      </div>
      <div class="store-search-wrap">
        <div class="store-search">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" /></svg>
          <input
            v-model="query"
            type="text"
            placeholder="Rechercher un jeu à acheter…"
            autocomplete="off"
            role="combobox"
            aria-autocomplete="list"
            :aria-expanded="suggestOpen && suggestions.length > 0"
            @input="onInput"
            @keydown="onSearchKey"
            @focus="query && (suggestOpen = true)"
          />
          <button v-if="query" class="clear" aria-label="Effacer" @click="clearSearch">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
          <button class="go" @click="submitSearch">Rechercher</button>
        </div>

        <ul v-if="suggestOpen && suggestions.length" class="suggest" role="listbox">
          <li
            v-for="(s, i) in suggestions"
            :key="s.gameId"
            class="suggest-item"
            :class="{ active: i === activeIndex }"
            role="option"
            :aria-selected="i === activeIndex"
            @mousedown.prevent="pick(s)"
            @mouseenter="activeIndex = i"
          >
            <span class="suggest-cover">
              <img v-if="s.coverUrl" :src="s.coverUrl" alt="" loading="lazy" />
            </span>
            <span class="suggest-title">{{ s.title }}</span>
            <svg class="suggest-go" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
          </li>
        </ul>
      </div>
    </div>

    <div class="bar">
      <span class="ctx">
        {{ activeQuery ? `Résultats pour « ${activeQuery} »` : "Sélection du moment" }}
        <span class="n">· {{ items.length }}</span>
      </span>
      <span class="spacer" />
      <template v-if="!activeQuery">
        <button
          v-for="s in SORTS"
          :key="s.key"
          class="chip"
          :class="{ active: sort === s.key }"
          @click="setSort(s.key)"
        >
          {{ s.label }}
        </button>
      </template>
    </div>

    <div v-if="loading && !items.length" class="empty">
      <span class="spin" />
      <p>Chargement de la boutique…</p>
    </div>
    <div v-else-if="!items.length" class="empty">
      <p>Aucun jeu trouvé.</p>
      <p v-if="activeQuery" class="dim">Essaie un autre titre.</p>
    </div>

    <div v-else class="grid" :class="{ dim: loading }">
      <button
        v-for="it in items"
        :key="it.gameId"
        class="card"
        @click="openProduct(it.gameId)"
      >
        <div class="cover" :style="{ background: gradientFor(it.gameId) }">
          <img
            v-if="coverSrc(it)"
            :key="coverSrc(it)!"
            class="cover-img"
            :src="coverSrc(it)!"
            alt=""
            loading="lazy"
            @error="onCoverError(coverSrc(it))"
          />
          <span v-if="it.savings > 0" class="badge">-{{ it.savings }}%</span>
          <span class="cover-scrim" />
          <span class="cover-title">{{ it.title }}</span>
        </div>
        <div class="meta">
          <div class="price">
            <span class="now" :class="{ hot: it.savings > 0 }">{{ price(it.price) }}</span>
            <span v-if="it.savings > 0" class="was">{{ price(it.normalPrice) }}</span>
          </div>
          <span v-if="it.storeName" class="store-name">{{ it.storeName }}</span>
        </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.store { min-width: 0; }
.store-head { display: flex; align-items: flex-end; gap: 20px; flex-wrap: wrap; margin-bottom: 20px; }
.store-title h2 { font-size: 26px; font-weight: 800; letter-spacing: -0.03em; margin: 0; }
.store-title .sub { margin: 4px 0 0; font-size: 13.5px; color: var(--text-faint); }
.store-search-wrap { position: relative; margin-left: auto; flex: 1; min-width: 320px; max-width: 520px; }
.store-search {
  display: flex; align-items: center; gap: 8px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 4px 4px 4px 12px;
}
.store-search:focus-within { border-color: var(--accent); }
.store-search > svg { width: 17px; height: 17px; color: var(--text-faint); flex: none; }
.store-search input { flex: 1; min-width: 0; background: none; border: none; outline: none; color: var(--text); font-size: 14px; padding: 8px 0; }
.store-search .clear { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; background: none; border: none; color: var(--text-faint); }
.store-search .clear svg { width: 15px; height: 15px; }
.store-search .clear:hover { background: var(--surface-2); color: var(--text); }
.store-search .go { flex: none; padding: 8px 14px; border-radius: 9px; background: var(--accent); color: var(--accent-ink); border: none; font-weight: 600; font-size: 13px; }
.store-search .go:hover { background: var(--accent-hover); }

/* Menu déroulant d'autocomplétion */
.suggest {
  position: absolute; top: calc(100% + 6px); left: 0; right: 0; z-index: 40;
  list-style: none; margin: 0; padding: 6px; max-height: 380px; overflow-y: auto;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px;
  box-shadow: var(--shadow-hero); display: flex; flex-direction: column; gap: 1px;
}
.suggest-item {
  display: flex; align-items: center; gap: 11px; padding: 7px 9px; border-radius: 9px; cursor: pointer;
}
.suggest-item.active { background: var(--surface-2); }
.suggest-cover {
  width: 34px; height: 45px; flex: none; border-radius: 6px; overflow: hidden;
  background: var(--surface-3); border: 1px solid var(--border);
}
.suggest-cover img { width: 100%; height: 100%; object-fit: cover; display: block; }
.suggest-title {
  flex: 1; min-width: 0; font-size: 13.5px; font-weight: 600; color: var(--text);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.suggest-go { width: 16px; height: 16px; color: var(--text-faint); flex: none; opacity: 0; }
.suggest-item.active .suggest-go { opacity: 1; }

.bar { display: flex; align-items: center; gap: 8px; margin-bottom: 18px; flex-wrap: wrap; }
.bar .ctx { font-size: 14px; font-weight: 600; color: var(--text); }
.bar .ctx .n { font-family: var(--mono); color: var(--text-faint); font-weight: 400; }
.bar .spacer { flex: 1; }
.chip {
  padding: 6px 13px; border-radius: 99px; font-size: 12.5px; color: var(--text-dim);
  background: var(--surface); border: 1px solid var(--border); transition: all 0.15s;
}
.chip:hover { color: var(--text); border-color: var(--border-strong); }
.chip.active { background: var(--text); color: var(--bg); border-color: var(--text); font-weight: 600; }

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(172px, 1fr)); gap: 22px 20px; transition: opacity 0.15s; }
.grid.dim { opacity: 0.5; }
.card { background: none; border: none; padding: 0; text-align: left; color: inherit; display: flex; flex-direction: column; gap: 10px; cursor: pointer; }
.cover {
  position: relative; aspect-ratio: 3 / 4; border-radius: var(--radius); overflow: hidden;
  box-shadow: var(--shadow-card); border: 1px solid var(--border); isolation: isolate;
  transition: transform 0.22s cubic-bezier(0.2, 0.7, 0.3, 1), box-shadow 0.22s;
}
.cover-img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block; }
.cover-scrim { position: absolute; inset: 0; background: linear-gradient(0deg, rgba(0, 0, 0, 0.6) 0%, transparent 45%); }
.cover-title {
  position: absolute; left: 12px; right: 12px; bottom: 11px; z-index: 2; font-weight: 800; font-size: 16px;
  line-height: 1.08; letter-spacing: -0.02em; color: #fff; text-shadow: 0 2px 12px rgba(0, 0, 0, 0.5); text-wrap: balance;
}
.badge {
  position: absolute; top: 10px; right: 10px; z-index: 2; font-family: var(--mono); font-weight: 700; font-size: 12px;
  padding: 3px 8px; border-radius: 8px; color: #fff;
  background: linear-gradient(135deg, #3ad07f, #1fa862); box-shadow: 0 3px 10px -3px rgba(31, 168, 98, 0.7);
}
.card:hover .cover { transform: translateY(-6px); box-shadow: 0 26px 50px -20px rgba(0, 0, 0, 0.75); }
.meta { display: flex; align-items: baseline; gap: 8px; padding: 0 2px; }
.price { display: flex; align-items: baseline; gap: 7px; }
.price .now { font-family: var(--mono); font-size: 16px; font-weight: 700; color: var(--text); font-variant-numeric: tabular-nums; }
.price .now.hot { color: var(--accent); }
.price .was { font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); text-decoration: line-through; }
.store-name { margin-left: auto; font-family: var(--mono); font-size: 10.5px; color: var(--text-faint); }

.empty { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 80px 0; text-align: center; color: var(--text-faint); font-size: 14px; }
.empty p { margin: 3px 0; }
.empty .dim { font-size: 12.5px; opacity: 0.8; }
.spin { width: 26px; height: 26px; border-radius: 50%; border: 3px solid var(--border-strong); border-top-color: var(--accent); animation: spin 0.7s linear infinite; margin-bottom: 14px; }
@keyframes spin { to { transform: rotate(360deg); } }

@media (max-width: 620px) {
  .store-head { flex-direction: column; align-items: stretch; }
  .store-search { margin-left: 0; max-width: none; }
}
</style>
