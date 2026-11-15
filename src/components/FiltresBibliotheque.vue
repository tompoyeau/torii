<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useFiltresBibliotheque } from "../composables/useFiltresBibliotheque";
import { useUi } from "../composables/useUi";
import { getSettings } from "../lib/tauri";
import { MODE_WEB } from "../lib/mode";
import type { LibraryFilter, Settings } from "../types";
import PlatformIcon from "./PlatformIcon.vue";
import { t } from "../i18n";

/**
 * Les filtres de la bibliothèque, en puces au-dessus de la grille : les vues (tous, favoris,
 * installés…) sur une rangée, les plateformes sur la suivante. Un seul filtre actif à la
 * fois, sur les deux rangées. Le contenu du slot (catégories, tris) se range au bout de la
 * première.
 *
 * Sur téléphone, les deux rangées n'en font plus qu'une, qui défile de côté.
 */
const props = defineProps<{
  /** Le nombre de jeux affichés, une fois la recherche et les catégories appliquées. */
  affiches: number;
}>();

const { filter, setFilter, openSettings, settingsOpen } = useUi();
const { count, platforms } = useFiltresBibliotheque();

/**
 * Le nombre de jeux affichés, seulement quand la recherche, les catégories ou « installés
 * uniquement » en écartent : sinon, c'est déjà celui de la puce active.
 */
const resteint = computed(() => props.affiches !== count(filter.value).value);

const vues = computed(() => {
  const liste: { id: LibraryFilter; label: string }[] = [{ id: "all", label: t("bibliotheque.laterale.tous") }];
  if (count("family").value) {
    liste.push(
      { id: "mine", label: t("bibliotheque.laterale.mesJeux") },
      { id: "family", label: t("bibliotheque.laterale.famille") },
    );
  }
  // Favoris et installés n'ont de sens que sur le PC où sont les jeux.
  if (!MODE_WEB) {
    liste.push(
      { id: "favorite", label: t("bibliotheque.laterale.favoris") },
      { id: "installed", label: t("bibliotheque.laterale.installes") },
    );
  }
  return liste.map((v) => ({ ...v, n: count(v.id).value }));
});

/**
 * Une plateforme sans aucun jeu n'a pas de puce : elle n'aurait rien à filtrer. Sauf si
 * c'est le filtre en cours (un retour d'historique peut y ramener), pour qu'on voie où l'on est.
 */
const plateformes = computed(() =>
  platforms.value
    .map((p) => ({ ...p, n: count(p.id).value }))
    .filter((p) => p.n > 0 || filter.value === p.id),
);

// État de connexion des launchers, pour le libellé de la dernière puce. Rafraîchi au
// montage et à chaque fermeture des Paramètres (où les connexions changent).
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
</script>

<template>
  <div class="filtres no-scrollbar">
    <div class="rang">
      <nav class="puces" :aria-label="t('bibliotheque.laterale.bibliotheque')">
        <button
          v-for="v in vues"
          :key="v.id"
          class="chip puce"
          :class="{ active: filter === v.id }"
          :aria-pressed="filter === v.id"
          @click="setFilter(v.id)"
        >
          {{ v.label }} <span class="n">{{ v.n }}</span>
        </button>
      </nav>
      <span class="espace" />
      <div class="outils"><slot /></div>
    </div>
    <div class="rang">
      <nav v-if="plateformes.length" class="puces" :aria-label="t('bibliotheque.laterale.plateformes')">
        <button
          v-for="p in plateformes"
          :key="p.id"
          class="chip puce plateforme"
          :class="{ active: filter === p.id }"
          :aria-pressed="filter === p.id"
          :aria-label="`${p.label} (${p.n})`"
          :title="p.label"
          @click="setFilter(p.id)"
        >
          <PlatformIcon :platform="p.icon" />
          <span class="nom">{{ p.label }}</span> <span class="n">{{ p.n }}</span>
        </button>
      </nav>
      <button v-if="!MODE_WEB" class="chip connexions" @click="openSettings('accounts')">
        <svg v-if="tousConnectes" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 6h16M4 12h16M4 18h16" /><circle cx="9" cy="6" r="2" fill="var(--bg)" /><circle cx="15" cy="12" r="2" fill="var(--bg)" /><circle cx="8" cy="18" r="2" fill="var(--bg)" /></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
        {{ tousConnectes ? t("bibliotheque.laterale.gererConnexions") : t("bibliotheque.laterale.connecterLauncher") }}
      </button>
      <span class="espace" />
      <span v-if="resteint" class="total" aria-live="polite">{{ t("bibliotheque.jeux", { n: affiches }) }}</span>
    </div>
  </div>
</template>

<style scoped>
.filtres { display: flex; flex-direction: column; gap: 10px; margin-bottom: 20px; }
.rang { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.puces { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.outils { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.espace { flex: 1; }

.puce { display: inline-flex; align-items: center; gap: 7px; }
.n { font-family: var(--mono); font-size: 10.5px; color: var(--text-faint); font-variant-numeric: tabular-nums; }
.plateforme :deep(.platform-icon) { width: 14px; height: 14px; }
/* La puce active en couleur d'accent, comme l'onglet de la section : elle se distingue des
   puces de tri (fond inversé), et les logos colorés des plateformes restent lisibles. */
.chip.puce.active {
  background: var(--accent-soft); color: var(--text);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent);
}
.chip.puce.active .n { color: var(--accent); }

.connexions {
  display: inline-flex; align-items: center; gap: 6px;
  border-style: dashed; border-color: var(--border-strong); background: none;
}
.connexions svg { width: 14px; height: 14px; flex: none; }
.total { font-family: var(--mono); font-size: 12px; color: var(--text-faint); font-variant-numeric: tabular-nums; }

/* Fenêtre étroite, comme pour les onglets : le logo et le compteur, le nom en infobulle.
   Avec les noms, les huit plateformes prenaient trois lignes à 940 px. */
@media (min-width: 821px) and (max-width: 1180px) {
  .plateforme .nom { display: none; }
  .plateforme { gap: 6px; }
}

/* Téléphone : une seule rangée qui défile de côté, sans le compteur. */
@media (max-width: 820px) {
  .filtres { flex-direction: row; align-items: center; gap: 8px; overflow-x: auto; margin: 0 -18px 16px; padding: 0 18px; }
  .rang, .puces, .outils { display: contents; }
  .filtres :deep(.chip) { flex: none; white-space: nowrap; }
  .espace, .total { display: none; }
}
</style>
