<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { t } from "../i18n";

/**
 * Menu « Catégories » : un bouton et sa liste à cocher, avec recherche. Le même dans la
 * bibliothèque (`CategoryFilter`) et la Boutique (`StoreView`) ; chacun lui fournit ses
 * options, déjà triées, et réagit aux cases cochées.
 */
export interface OptionCategorie {
  cle: string | number;
  nom: string;
  /** Nombre de jeux, quand on le connaît (bibliothèque). */
  n?: number;
  on: boolean;
}

const props = defineProps<{
  options: OptionCategorie[];
  /** Texte du bouton. */
  libelle: string;
  /** Au moins une catégorie cochée : le bouton passe en couleur d'accent. */
  actif: boolean;
  /** Infobulle du bouton (la liste complète quand le libellé la tronque). */
  titre?: string;
}>();

const emit = defineEmits<{
  basculer: [cle: string | number];
  effacer: [];
}>();

const ouvert = ref(false);
const recherche = ref("");
const racine = ref<HTMLElement | null>(null);
const bouton = ref<HTMLElement | null>(null);
const champ = ref<HTMLInputElement | null>(null);
const liste = ref<HTMLElement | null>(null);

/** Minuscules sans accents : « metroid » trouve « Metroidvania », « reflexion » « Réflexion ». */
function plier(s: string): string {
  return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

const visibles = computed(() => {
  const q = plier(recherche.value.trim());
  return q ? props.options.filter((o) => plier(o.nom).includes(q)) : props.options;
});

async function basculerMenu() {
  ouvert.value = !ouvert.value;
  if (ouvert.value) {
    recherche.value = "";
    await nextTick();
    champ.value?.focus();
  }
}

function fermer(rendreFocus: boolean) {
  ouvert.value = false;
  if (rendreFocus) bouton.value?.focus();
}

/** Entrée dans le champ : coche la première catégorie trouvée, et vide le champ pour la suivante. */
function valider() {
  const premiere = visibles.value.find((o) => !o.on);
  if (!premiere) return;
  emit("basculer", premiere.cle);
  recherche.value = "";
}

/** Flèches haut/bas : passent d'une catégorie à l'autre, et du champ à la liste. */
function deplacer(e: KeyboardEvent, sens: 1 | -1) {
  const items = [...(liste.value?.querySelectorAll<HTMLElement>(".opt") ?? [])];
  if (!items.length) return;
  e.preventDefault();
  const i = items.indexOf(document.activeElement as HTMLElement);
  if (i === -1) {
    if (sens === 1) items[0].focus();
    return;
  }
  if (i === 0 && sens === -1) champ.value?.focus();
  else items[Math.min(items.length - 1, i + sens)].focus();
}

function surClic(e: MouseEvent) {
  if (ouvert.value && !racine.value?.contains(e.target as Node)) fermer(false);
}
/** Échap ferme le menu et rend le focus au bouton : sinon il laisse le clavier nulle part. */
function surTouche(e: KeyboardEvent) {
  if (e.key === "Escape" && ouvert.value) {
    // Le menu d'abord : `useRaccourcis` ignore un Échap déjà pris, sans quoi le même
    // appui refermerait aussi l'écran du dessous.
    e.preventDefault();
    fermer(true);
  }
}
onMounted(() => {
  document.addEventListener("click", surClic);
  document.addEventListener("keydown", surTouche);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", surClic);
  document.removeEventListener("keydown", surTouche);
});
</script>

<template>
  <div ref="racine" class="cat-wrap">
    <button
      ref="bouton"
      class="chip cat-btn"
      :class="{ active: actif }"
      aria-haspopup="dialog"
      :aria-expanded="ouvert"
      :title="titre"
      @click.stop="basculerMenu"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 5h18M6 12h12M10 19h4" /></svg>
      <span class="cat-libelle">{{ libelle }}</span>
      <svg class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
    </button>

    <div v-if="ouvert" class="cat-menu" role="dialog" :aria-label="t('commun.categories.bouton')">
      <div class="cat-search">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></svg>
        <input
          ref="champ"
          v-model="recherche"
          type="search"
          :placeholder="t('commun.categories.chercher')"
          :aria-label="t('commun.categories.chercher')"
          @keydown.enter.prevent="valider"
          @keydown.down="deplacer($event, 1)"
        />
      </div>

      <div ref="liste" class="cat-list" @keydown.down="deplacer($event, 1)" @keydown.up="deplacer($event, -1)">
        <button
          v-for="o in visibles"
          :key="o.cle"
          class="opt"
          :class="{ on: o.on }"
          :aria-pressed="o.on"
          @click="emit('basculer', o.cle)"
        >
          <span class="check" aria-hidden="true">
            <svg v-if="o.on" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
          </span>
          <span class="nom">{{ o.nom }}</span>
          <span v-if="o.n !== undefined" class="n">{{ o.n }}</span>
        </button>
        <p v-if="!visibles.length" class="vide">{{ t("commun.categories.aucune") }}</p>
      </div>

      <div v-if="actif" class="cat-pied">
        <span class="aide">{{ t("commun.categories.toutes") }}</span>
        <button class="effacer" @click="emit('effacer')">{{ t("commun.categories.effacer") }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cat-wrap { position: relative; }
.cat-btn { display: inline-flex; align-items: center; gap: 6px; max-width: 260px; }
.cat-btn svg { width: 14px; height: 14px; flex: none; }
.cat-btn .caret { width: 13px; height: 13px; margin-left: -1px; opacity: 0.7; }
.cat-libelle { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cat-btn.active {
  background: var(--accent-soft); color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent); font-weight: 600;
}

.cat-menu {
  position: absolute; top: calc(100% + 6px); right: 0; z-index: 40; width: 290px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px;
  box-shadow: var(--shadow-hero); display: flex; flex-direction: column; overflow: hidden;
}
.cat-search {
  display: flex; align-items: center; gap: 8px; margin: 8px 8px 4px; padding: 0 10px;
  background: var(--surface-2); border: 1px solid var(--border); border-radius: 9px;
}
.cat-search svg { width: 14px; height: 14px; color: var(--text-faint); flex: none; }
.cat-search input {
  flex: 1; min-width: 0; border: none; outline: none; background: none;
  color: var(--text); font: inherit; font-size: 13px; padding: 8px 0;
}
.cat-search:focus-within { border-color: color-mix(in srgb, var(--accent) 55%, transparent); }

.cat-list { max-height: 340px; overflow-y: auto; padding: 4px 6px 6px; display: flex; flex-direction: column; gap: 1px; }
.opt {
  display: flex; align-items: center; gap: 10px; padding: 7px 8px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13px; text-align: left;
  width: 100%; cursor: pointer;
}
.opt:hover, .opt:focus-visible { background: var(--surface-2); outline: none; }
.opt.on { color: var(--accent); font-weight: 600; }
.check {
  width: 16px; height: 16px; flex: none; border-radius: 5px; border: 1.5px solid var(--border-strong);
  display: grid; place-items: center;
}
.opt.on .check { background: var(--accent); border-color: var(--accent); color: var(--accent-ink); }
.check svg { width: 11px; height: 11px; }
.nom { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.n { font-family: var(--mono); font-size: 11px; color: var(--text-faint); font-variant-numeric: tabular-nums; }
.opt.on .n { color: var(--accent); }
.vide { margin: 10px 8px; font-size: 13px; color: var(--text-faint); }

.cat-pied {
  display: flex; align-items: center; gap: 10px; padding: 8px 12px;
  border-top: 1px solid var(--border);
}
.aide { flex: 1; font-size: 11.5px; line-height: 1.35; color: var(--text-faint); }
.effacer {
  flex: none; background: none; border: none; padding: 4px 2px; cursor: pointer;
  color: var(--accent); font: inherit; font-size: 12.5px; font-weight: 600;
}
.effacer:hover { text-decoration: underline; }
</style>
