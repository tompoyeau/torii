<script setup lang="ts">
import { etiquetteIntl, t } from "../i18n";
import type { GameNews } from "../types";
import { ouvrirPage } from "../composables/usePageWeb";

/**
 * Actualités d'un jeu de la bibliothèque : les dernières annonces de l'éditeur sur Steam
 * (mises à jour, événements, notes de version). Les articles de presse que Steam mêle à
 * ce fil sont écartés côté Rust (`metadata/possede.rs`).
 */
defineProps<{ actualites: GameNews[]; appId: number; studio?: string }>();

const date = (s: number) =>
  new Intl.DateTimeFormat(etiquetteIntl.value, { day: "numeric", month: "short", year: "numeric" }).format(new Date(s * 1000));

function cacher(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}
</script>

<template>
  <section class="news">
    <div class="tete">
      <div>
        <h3>{{ t("fiche.actualites.titre") }}</h3>
        <p>{{ studio ? t("fiche.actualites.sousTitre", { studio }) : t("fiche.actualites.sousTitreEditeur") }}</p>
      </div>
      <button class="lien" @click="ouvrirPage(`https://store.steampowered.com/news/app/${appId}`)">
        {{ t("fiche.actualites.toutes") }} ↗
      </button>
    </div>
    <div class="grille">
      <button v-for="n in actualites" :key="n.url" class="carte" @click="ouvrirPage(n.url)">
        <img v-if="n.image" :src="n.image" alt="" loading="lazy" @error="cacher" />
        <span class="texte">
          <span class="date">{{ date(n.date) }}</span>
          <span class="titre">{{ n.titre }}</span>
          <span v-if="n.extrait" class="extrait">{{ n.extrait }}</span>
        </span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.news { display: flex; flex-direction: column; gap: 14px; }
.tete { display: flex; align-items: flex-end; gap: 12px; }
.tete > div { flex: 1; min-width: 0; }
h3 { margin: 0; font-size: 20px; font-weight: 800; letter-spacing: -0.02em; color: var(--text); }
.tete p { margin: 3px 0 0; font-size: 13px; color: var(--text-faint); }
.lien { flex: none; background: none; border: none; padding: 0; color: var(--accent); font-size: 13px; font-weight: 600; }
.lien:hover { text-decoration: underline; }
/* Deux colonnes au plus : une annonce a besoin de largeur pour son titre et son extrait. */
.grille { display: grid; grid-template-columns: repeat(auto-fill, minmax(max(340px, calc(50% - 7px)), 1fr)); gap: 14px; }
.carte {
  display: flex; gap: 14px; padding: 10px; border-radius: 14px; min-width: 0; text-align: left; cursor: pointer;
  background: var(--surface); border: 1px solid var(--border); color: var(--text); transition: border-color 0.15s;
}
.carte:hover { border-color: var(--border-strong); }
.carte img { width: 150px; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 9px; flex: none; display: block; background: var(--surface-2); }
.texte { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
.date { font-family: var(--mono); font-size: 11px; color: var(--text-faint); }
.titre { font-size: 14px; font-weight: 700; line-height: 1.25; }
.extrait {
  font-size: 12.5px; color: var(--text-dim); line-height: 1.45;
  display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;
}
</style>
