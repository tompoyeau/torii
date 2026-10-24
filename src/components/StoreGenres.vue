<script setup lang="ts">
import { computed, ref } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useQuandVisible } from "../composables/useQuandVisible";
import { useStoreCategories } from "../composables/useStoreCategories";
import { useStoreHome } from "../composables/useStoreHome";
import { t, type Cle } from "../i18n";
import StoreRow from "./StoreRow.vue";
import StoreShelfCard from "./StoreShelfCard.vue";

/**
 * Rayons par genre : un tag Steam par onglet.
 *
 * 🔑 L'ORDRE DES ONGLETS VIENT DE LA BIBLIOTHÈQUE. Les genres les plus présents parmi
 * les jeux de l'utilisateur passent devant, et le premier est ouvert d'office. Calcul
 * local, sur les genres IGDB déjà connus : rien ne quitte la machine.
 *
 * ⚠️ Les genres de la bibliothèque sont des libellés IGDB **traduits** (« Jeu de rôle »
 * ou « Role-playing » selon la langue, cf. `igdb::genre_fr` / `genre_en`) : chaque genre
 * liste donc ses libellés dans les deux langues.
 */
interface Genre {
  cle: string;
  /** Tag Steam (`tagdata/populartags`). */
  tag: number;
  /** Genres IGDB de la bibliothèque qui comptent pour lui, en français et en anglais. */
  igdb: string[];
}
const GENRES: Genre[] = [
  { cle: "action", tag: 19, igdb: ["Beat'em up", "Beat 'em up", "Combat", "Fighting", "Arcade"] },
  { cle: "aventure", tag: 21, igdb: ["Aventure", "Adventure", "Pointer-cliquer", "Point-and-click", "Roman visuel", "Visual Novel"] },
  { cle: "rpg", tag: 122, igdb: ["Jeu de rôle", "Role-playing", "RPG"] },
  { cle: "strategie", tag: 9, igdb: ["Stratégie", "Strategy", "Stratégie temps réel", "Real-time strategy", "Stratégie au tour par tour", "Turn-based strategy", "Tactique", "Tactical", "MOBA"] },
  { cle: "simulation", tag: 599, igdb: ["Simulation"] },
  { cle: "tir", tag: 1774, igdb: ["Tir", "Shooter"] },
  { cle: "course", tag: 699, igdb: ["Course", "Racing"] },
  { cle: "sport", tag: 701, igdb: ["Sport"] },
  { cle: "reflexion", tag: 1664, igdb: ["Réflexion", "Puzzle"] },
  { cle: "independant", tag: 492, igdb: ["Indépendant", "Indie"] },
];

const { games } = useLibrary();
const { chargerRayon, rayon } = useStoreHome();

/** Les genres, du plus présent dans la bibliothèque au moins présent (ordre stable). */
const ordre = computed(() => {
  const compte = new Map<string, number>();
  for (const g of games.value) {
    if (g.hidden || !g.genre) continue;
    const genre = GENRES.find((x) => x.igdb.includes(g.genre!));
    if (genre) compte.set(genre.cle, (compte.get(genre.cle) ?? 0) + 1);
  }
  return GENRES.map((g, i) => ({ g, i, n: compte.get(g.cle) ?? 0 }))
    .sort((a, b) => b.n - a.n || a.i - b.i)
    .map((x) => x.g);
});

/** Choix explicite de l'utilisateur ; sinon le genre le plus présent. */
const choisi = ref<string | null>(null);
const actif = computed(() => ordre.value.find((g) => g.cle === choisi.value) ?? ordre.value[0]);
const etat = computed(() => rayon("genre", actif.value.tag));

const section = ref<HTMLElement | null>(null);
const visible = ref(false);
useQuandVisible(section, () => {
  visible.value = true;
  void chargerRayon("genre", actif.value.tag);
});

function choisir(g: Genre) {
  choisi.value = g.cle;
  void chargerRayon("genre", g.tag);
}

const nom = (g: Genre) => t(`boutique.accueil.genres.${g.cle}` as Cle);

/** « Tout voir » : tous les jeux du genre, par la recherche par catégories (même tag Steam). */
const { ouvrir } = useStoreCategories();
const voirTout = () => ouvrir({ id: actif.value.tag, nom: nom(actif.value) });
</script>

<template>
  <div ref="section" class="genres">
    <StoreRow
      :titre="t('boutique.accueil.genres.titre')"
      :sous-titre="t('boutique.accueil.genres.sousTitre')"
      :nombre="etat.items.length"
    >
      <template #actions>
        <button class="lien" @click="voirTout">{{ t("boutique.accueil.toutVoir") }}</button>
      </template>
      <template #avant>
        <div class="pastilles" role="group" :aria-label="t('boutique.accueil.genres.titre')">
          <button
            v-for="g in ordre"
            :key="g.cle"
            class="chip"
            :class="{ active: g.cle === actif.cle }"
            :aria-pressed="g.cle === actif.cle"
            @click="choisir(g)"
          >
            {{ nom(g) }}
          </button>
        </div>
      </template>
      <StoreShelfCard v-for="j in etat.items" :key="j.appId" :jeu="j" />
      <template v-if="!etat.items.length && (etat.etat === 'chargement' || !visible)">
        <div v-for="n in 6" :key="n" class="fantome" aria-hidden="true" />
      </template>
    </StoreRow>
    <div v-if="etat.etat === 'echec' && !etat.items.length" class="echec">
      {{ t("boutique.accueil.echec") }}
      <button class="chip" @click="chargerRayon('genre', actif.tag, true)">{{ t("boutique.accueil.reessayer") }}</button>
    </div>
  </div>
</template>

<style scoped>
.pastilles { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 16px; }
.lien { background: none; border: none; padding: 0 6px; color: var(--accent); font-size: 13px; font-weight: 600; }
.lien:hover { text-decoration: underline; }
.fantome { aspect-ratio: 3 / 4; border-radius: var(--radius); background: var(--surface); border: 1px solid var(--border); }
.echec { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--text-faint); }
</style>
