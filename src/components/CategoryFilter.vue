<script setup lang="ts">
import { computed } from "vue";
import { useUi } from "../composables/useUi";
import type { Game } from "../types";
import { etiquetteIntl, t } from "../i18n";
import MenuCategories, { type OptionCategorie } from "./MenuCategories.vue";

/**
 * Filtre « Catégories » de la bibliothèque : les tags Steam des jeux, plusieurs à la fois.
 *
 * 🔑 Les compteurs sont « à facettes » : à côté de chaque catégorie, le nombre de jeux
 * qu'on obtiendrait en la cochant EN PLUS de celles déjà cochées. Avec plus de 250
 * catégories, en cocher deux au hasard menait sinon presque toujours à une grille vide ;
 * celles qui y mèneraient ne sont tout simplement pas proposées.
 */
const props = defineProps<{
  /** Les jeux affichés AVANT ce filtre (plateforme, recherche, « installés »). */
  games: Game[];
}>();

const { categories, toggleCategorie, clearCategories } = useUi();

/** Pas de bouton tant qu'aucun jeu n'a de catégorie (premier lancement, démo vide). */
const disponible = computed(() => categories.value.length > 0 || props.games.some((g) => g.tags?.length));

/** Les jeux qui ont toutes les catégories cochées. */
const retenus = computed(() =>
  props.games.filter((g) => categories.value.every((c) => g.tags?.includes(c))),
);

const options = computed<OptionCategorie[]>(() => {
  const compte = new Map<string, number>();
  for (const g of retenus.value) {
    for (const tag of g.tags ?? []) compte.set(tag, (compte.get(tag) ?? 0) + 1);
  }
  // Une catégorie cochée reste dans la liste même si la vue courante n'a plus aucun jeu
  // qui la porte : sinon on ne pourrait plus la décocher.
  for (const c of categories.value) if (!compte.has(c)) compte.set(c, 0);
  return [...compte.entries()]
    .map(([nom, n]) => ({ cle: nom, nom, n, on: categories.value.includes(nom) }))
    .sort(
      (a, b) =>
        Number(b.on) - Number(a.on) || b.n - a.n || a.nom.localeCompare(b.nom, etiquetteIntl.value),
    );
});

const libelle = computed(() => {
  const c = categories.value;
  if (!c.length) return t("commun.categories.bouton");
  return c.length === 1 ? c[0] : `${c[0]} +${c.length - 1}`;
});
</script>

<template>
  <MenuCategories
    v-if="disponible"
    :options="options"
    :libelle="libelle"
    :actif="categories.length > 0"
    :titre="categories.length > 1 ? categories.join(' + ') : undefined"
    @basculer="(cle) => toggleCategorie(String(cle))"
    @effacer="clearCategories()"
  />
</template>
