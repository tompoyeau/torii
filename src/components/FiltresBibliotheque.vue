<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useFiltresBibliotheque } from "../composables/useFiltresBibliotheque";
import { useUi } from "../composables/useUi";
import { getSettings } from "../lib/tauri";
import { MODE_WEB } from "../lib/mode";
import type { Game, LibraryFilter, PlatformId, Settings, SortKey } from "../types";
import CategoryFilter from "./CategoryFilter.vue";
import MenuChoix, { type OptionChoix } from "./MenuChoix.vue";
import PlatformIcon from "./PlatformIcon.vue";
import { t } from "../i18n";

/**
 * La barre d'outils de la grille, sur une ligne : les vues (tous, favoris, installés…) en
 * commande segmentée, puis la plateforme, les catégories et le tri en menus déroulants.
 *
 * 🔑 UN SEUL FILTRE À LA FOIS (`useUi().filter`) : choisir une plateforme remplace la vue,
 * et aucun segment n'est alors allumé ; « Toutes les plateformes » ramène à « Tous ».
 */
const props = defineProps<{
  /** Le nombre de jeux affichés, une fois la recherche et les catégories appliquées. */
  affiches: number;
  /** Les jeux avant le filtre des catégories : c'est sur eux qu'il compte. */
  avantCategories: Game[];
}>();

const { filter, setFilter, sort, setSort, installedOnly, toggleInstalledOnly, openSettings, settingsOpen } = useUi();
const { count, platforms } = useFiltresBibliotheque();

const vues = computed(() => {
  const liste: { id: LibraryFilter; label: string }[] = [{ id: "all", label: t("bibliotheque.laterale.tousCourt") }];
  if (count("family").value) {
    liste.push(
      { id: "mine", label: t("bibliotheque.laterale.mesJeux") },
      { id: "family", label: t("bibliotheque.laterale.famille") },
    );
  }
  // Les favoris n'ont de sens que sur le PC où sont les jeux. Pas de vue « Installés » :
  // la bascule « Installés uniquement », toujours là, fait ce travail sur n'importe quelle vue.
  if (!MODE_WEB) liste.push({ id: "favorite", label: t("bibliotheque.laterale.favoris") });
  return liste.map((v) => ({ ...v, n: count(v.id).value }));
});

// --- Plateforme -----------------------------------------------------------------

/**
 * Les plateformes du menu. Une plateforme sans aucun jeu n'y figure pas : elle n'aurait rien
 * à filtrer. Sauf si c'est le filtre en cours (un retour d'historique peut y ramener).
 */
const plateformes = computed(() =>
  platforms.value
    .map((p) => ({ ...p, n: count(p.id).value }))
    .filter((p) => p.n > 0 || filter.value === p.id),
);
const iconeDe = computed(() => new Map<string, PlatformId>(plateformes.value.map((p) => [p.id, p.icon])));
/** La plateforme filtrée (« Hors launcher » compris), ou aucune. */
const plateformeActive = computed(() => plateformes.value.find((p) => p.id === filter.value) ?? null);

const optionsPlateforme = computed<OptionChoix[]>(() => [
  { cle: "toutes", nom: t("bibliotheque.laterale.toutesPlateformes") },
  ...plateformes.value.map((p) => ({ cle: p.id, nom: p.label, n: p.n })),
]);

function choisirPlateforme(cle: string) {
  if (cle !== "toutes") setFilter(cle as LibraryFilter);
  // « Toutes » ne touche pas à une vue (Favoris…) : elle ne défait qu'une plateforme.
  else if (plateformeActive.value) setFilter("all");
}

// --- Tri ------------------------------------------------------------------------

const optionsTri = computed<OptionChoix<SortKey>[]>(() => [
  { cle: "recent", nom: t("bibliotheque.grille.tris.recent") },
  { cle: "alpha", nom: t("bibliotheque.grille.tris.alpha") },
  { cle: "playtime", nom: t("bibliotheque.grille.tris.playtime") },
]);
const libelleTri = computed(() => optionsTri.value.find((o) => o.cle === sort.value)?.nom ?? "");

// --- Connexions des launchers (pied du menu Plateforme) ----------------------------

const settings = ref<Settings | null>(null);
async function rafraichirConnexions() {
  // Hors Tauri (aperçu) : état fictif mixte (Steam + Epic connectés) pour la maquette.
  settings.value =
    (await getSettings()) ??
    { steamConnected: true, steamId: "0", epicConnected: true, eaConnected: false, battlenetConnected: false, gogConnected: false };
}
onMounted(() => {
  if (!MODE_WEB) void rafraichirConnexions();
});
watch(settingsOpen, (ouvert) => {
  if (!ouvert && !MODE_WEB) void rafraichirConnexions();
});
/** Les 5 launchers à compte (les seuls qui se « connectent »). */
const tousConnectes = computed(() => {
  const s = settings.value;
  return !!s && s.steamConnected && s.epicConnected && s.eaConnected && s.battlenetConnected && s.gogConnected;
});

/**
 * Le nombre de jeux affichés, seulement quand la recherche, les catégories ou « installés
 * uniquement » en écartent : sinon, c'est déjà celui de la vue ou de la plateforme.
 */
const resteint = computed(() => props.affiches !== count(filter.value).value);
</script>

<template>
  <div class="outils">
    <div class="segmented vues" role="group" :aria-label="t('bibliotheque.laterale.bibliotheque')">
      <button
        v-for="v in vues"
        :key="v.id"
        :class="{ on: filter === v.id }"
        :aria-pressed="filter === v.id"
        @click="setFilter(v.id)"
      >
        {{ v.label }} <span class="n">{{ v.n }}</span>
      </button>
    </div>

    <MenuChoix
      v-if="plateformes.length"
      :options="optionsPlateforme"
      :choisie="plateformeActive ? plateformeActive.id : 'toutes'"
      :libelle="plateformeActive?.label ?? t('bibliotheque.laterale.plateforme')"
      :n="plateformeActive?.n"
      :actif="!!plateformeActive"
      :nom-menu="t('bibliotheque.laterale.plateformes')"
      @choisir="choisirPlateforme"
    >
      <template #icone>
        <PlatformIcon v-if="plateformeActive" :platform="plateformeActive.icon" />
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3 3 7.5l9 4.5 9-4.5L12 3Z" /><path d="m3 12 9 4.5 9-4.5M3 16.5 12 21l9-4.5" /></svg>
      </template>
      <template #option="{ cle }">
        <PlatformIcon v-if="iconeDe.get(cle)" :platform="iconeDe.get(cle)!" />
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3 3 7.5l9 4.5 9-4.5L12 3Z" /><path d="m3 12 9 4.5 9-4.5M3 16.5 12 21l9-4.5" /></svg>
      </template>
      <template v-if="!MODE_WEB" #pied>
        <button class="pied-action" @click="openSettings('accounts')">
          <svg v-if="tousConnectes" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 6h16M4 12h16M4 18h16" /><circle cx="9" cy="6" r="2" fill="var(--surface)" /><circle cx="15" cy="12" r="2" fill="var(--surface)" /><circle cx="8" cy="18" r="2" fill="var(--surface)" /></svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
          {{ tousConnectes ? t("bibliotheque.laterale.gererConnexions") : t("bibliotheque.laterale.connecterLauncher") }}
        </button>
      </template>
    </MenuChoix>

    <CategoryFilter v-if="!MODE_WEB" :games="avantCategories" />

    <span class="espace" />
    <span v-if="resteint" class="total" aria-live="polite">{{ t("bibliotheque.jeux", { n: affiches }) }}</span>

    <!-- À droite, avec le tri : ce qui règle l'affichage plutôt que ce qu'on regarde. -->
    <button
      v-if="!MODE_WEB"
      class="chip bascule"
      :class="{ active: installedOnly }"
      :aria-pressed="installedOnly"
      :title="t('bibliotheque.grille.installesSeulementAide')"
      @click="toggleInstalledOnly()"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v11m0 0l-4-4m4 4l4-4" /><path d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2" /></svg>
      {{ t("bibliotheque.grille.installesSeulement") }}
    </button>

    <MenuChoix
      v-if="!MODE_WEB"
      class="a-droite"
      :options="optionsTri"
      :choisie="sort"
      :libelle="libelleTri"
      :nom-menu="t('bibliotheque.laterale.trier')"
      @choisir="setSort"
    >
      <template #icone>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M7 4v16m0 0-3-3m3 3 3-3M17 20V4m0 0-3 3m3-3 3 3" /></svg>
      </template>
    </MenuChoix>
  </div>
</template>

<style scoped>
.outils { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 20px; }
.espace { flex: 1; }

/* Les vues : la commande segmentée commune (`style.css`), un peu plus basse que les puces
   pour s'aligner sur elles. */
.vues button { padding: 5px 12px; font-size: 12.5px; }
.vues .n { font-family: var(--mono); font-size: 10.5px; font-weight: 400; color: var(--text-faint); font-variant-numeric: tabular-nums; }
.vues button.on .n { color: var(--accent); }
.vues button:not(.on):hover { color: var(--text); }

.bascule { display: inline-flex; align-items: center; gap: 6px; }
.bascule svg { width: 14px; height: 14px; }
.bascule.active {
  background: var(--accent-soft); color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent); font-weight: 600;
}

/* L'action au pied du menu Plateforme : même allure qu'une option, en couleur d'accent. */
.pied-action {
  display: flex; align-items: center; gap: 10px; width: 100%; padding: 8px 9px; border-radius: 9px;
  background: none; border: none; color: var(--accent); font: inherit; font-size: 13px; font-weight: 600;
  text-align: left; cursor: pointer;
}
.pied-action svg { width: 16px; height: 16px; flex: none; }
.pied-action:hover, .pied-action:focus-visible { background: var(--surface-2); outline: none; }

.total { font-family: var(--mono); font-size: 12px; color: var(--text-faint); font-variant-numeric: tabular-nums; }

/* Téléphone (version web) : la ligne passe à la ligne. Pas de défilement de côté, qui
   couperait les menus déroulants. */
@media (max-width: 820px) {
  .outils { margin-bottom: 16px; }
}
</style>
