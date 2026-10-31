<script setup lang="ts">
import { computed } from "vue";
import { usePossession } from "../composables/usePossession";
import { useStore } from "../composables/useStore";
import { useToriiWishlist } from "../composables/useToriiWishlist";
import { libelleAvis, libelleSortie, pageSteam, tonAvis } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { useImageCascade } from "../lib/images";
import { t } from "../i18n";
import type { StoreShelfItem } from "../types";
import StorePossede from "./StorePossede.vue";
import { ouvrirPage } from "../composables/usePageWeb";

/** Carte d'un jeu choisi par Steam : jaquette (portrait) ou bandeau (paysage). */
const props = withDefaults(defineProps<{ jeu: StoreShelfItem; format?: "portrait" | "paysage" }>(), {
  format: "portrait",
});

const { openProduct } = useStore();
const { possession } = usePossession();
const { isWishlisted, toggle } = useToriiWishlist();

const etat = computed(() => possession(props.jeu.title, props.jeu.appId));
const enPromo = computed(() => props.jeu.price != null && props.jeu.savings > 0);
/**
 * Note Steam sur la jaquette : la part d'évaluations positives, l'appréciation et leur
 * nombre au survol. Rien tant que Steam lui-même ne note pas le jeu (trop peu d'avis).
 */
const note = computed(() => {
  const j = props.jeu;
  if (!j.reviewScore || j.reviewPercent == null) return null;
  return {
    ton: tonAvis(j.reviewScore),
    aide: `${libelleAvis(j.reviewScore)} · ${t("boutique.accueil.carte.avisAide", { n: j.reviewCount })}`,
  };
});
/** Démo gratuite sur Steam — sans intérêt pour un jeu qu'on a déjà. */
const aUneDemo = computed(() => props.jeu.hasDemo && !etat.value);
const prix = (montant: number) => formatPrix(montant, props.jeu.currency);

const { src, onError } = useImageCascade(() =>
  props.format === "portrait"
    ? [props.jeu.capsuleUrl, props.jeu.headerUrl]
    : [props.jeu.headerUrl, props.jeu.capsuleUrl],
);

/** Fiche produit si le comparateur connaît le jeu, sinon sa page Steam. */
function ouvrir() {
  if (props.jeu.gameId) void openProduct(props.jeu.gameId);
  else ouvrirPage(pageSteam(props.jeu.appId));
}

function basculerWishlist() {
  void toggle({ gameId: props.jeu.gameId, title: props.jeu.title, coverUrl: props.jeu.capsuleUrl });
}
</script>

<template>
  <button v-if="format === 'portrait'" class="cover-card carte" @click="ouvrir">
    <div class="cover" :style="{ background: gradientFor(String(jeu.appId)) }">
      <img v-if="src" :key="src" class="cover-img" :src="src" alt="" loading="lazy" @error="onError" />
      <span v-if="enPromo || note" class="pile" :class="{ 'sous-coin': etat }">
        <span v-if="enPromo" class="badge">-{{ jeu.savings }}%</span>
        <span v-if="note" class="note" :class="note.ton" :title="note.aide">
          <svg v-if="note.ton === 'neg'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" aria-hidden="true"><path d="M17 14V2M9 18.1 10 14H4.2a2 2 0 0 1-1.9-2.5l2.3-8A2 2 0 0 1 6.5 2H20a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2.8a2 2 0 0 0-1.8 1.1L12 22a3.1 3.1 0 0 1-3-3.9Z" /></svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" aria-hidden="true"><path d="M7 10v12M15 5.9 14 10h5.8a2 2 0 0 1 1.9 2.5l-2.3 8a2 2 0 0 1-1.9 1.5H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2.8a2 2 0 0 0 1.8-1.1L12 2a3.1 3.1 0 0 1 3 3.9Z" /></svg>
          {{ jeu.reviewPercent }} %
        </span>
      </span>
      <span v-if="etat" class="coin"><StorePossede :etat="etat" /></span>
      <span
        v-else-if="jeu.gameId"
        class="wish-dot"
        :class="{ on: isWishlisted(jeu.gameId) }"
        role="button"
        :aria-label="isWishlisted(jeu.gameId) ? t('boutique.retirerWishlist') : t('boutique.ajouterWishlist')"
        :title="isWishlisted(jeu.gameId) ? t('boutique.retirerWishlist') : t('boutique.ajouterWishlist')"
        @click.stop="basculerWishlist"
      >
        <svg viewBox="0 0 24 24" :fill="isWishlisted(jeu.gameId) ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 20s-7-4.3-7-9.3A3.7 3.7 0 0 1 12 8a3.7 3.7 0 0 1 7 2.7c0 5-7 9.3-7 9.3Z" /></svg>
      </span>
      <span class="cover-scrim" />
      <span class="cover-title">{{ jeu.title }}</span>
    </div>
    <div class="pied">
      <span v-if="jeu.isFree" class="prix"><span class="gratuit">{{ t("boutique.accueil.carte.gratuit") }}</span></span>
      <span v-else-if="jeu.price != null" class="prix">
        <span class="now" :class="{ hot: enPromo }">{{ prix(jeu.price) }}</span>
        <span v-if="enPromo && jeu.normalPrice" class="was">{{ prix(jeu.normalPrice) }}</span>
        <span class="shop">{{ jeu.storeName }}</span>
      </span>
      <span v-else class="ligne">{{ t("boutique.accueil.carte.pasDOffre") }}</span>
      <span v-if="jeu.comingSoon" class="ligne">{{ libelleSortie(jeu) }}</span>
      <span v-else-if="jeu.record" class="record">{{ t("boutique.accueil.carte.record") }}</span>
      <span v-if="aUneDemo" class="demo-dispo" :title="t('boutique.accueil.carte.demoAide')">{{ t("boutique.accueil.carte.demo") }}</span>
    </div>
  </button>

  <button v-else class="carte-paysage carte" @click="ouvrir">
    <span class="visuel" :style="{ background: gradientFor(String(jeu.appId)) }">
      <img v-if="src" :key="src" :src="src" alt="" loading="lazy" @error="onError" />
      <span v-if="etat" class="coin"><StorePossede :etat="etat" /></span>
    </span>
    <span class="titre">{{ jeu.title }}</span>
    <span class="ligne-prix">
      <span v-if="jeu.isFree" class="gratuit">{{ t("boutique.accueil.carte.gratuit") }}</span>
      <span v-else-if="jeu.price != null" class="prix">
        <span v-if="enPromo" class="remise">-{{ jeu.savings }}%</span>
        <span class="now" :class="{ hot: enPromo }">{{ prix(jeu.price) }}</span>
      </span>
      <span v-if="aUneDemo" class="demo-dispo" :title="t('boutique.accueil.carte.demoAide')">{{ t("boutique.accueil.carte.demo") }}</span>
      <span v-if="jeu.reviewScore" class="avis" :class="tonAvis(jeu.reviewScore)" :title="t('boutique.accueil.carte.avisAide', { n: jeu.reviewCount })">
        {{ libelleAvis(jeu.reviewScore) }}<template v-if="jeu.reviewPercent != null"> · {{ jeu.reviewPercent }} %</template>
      </span>
    </span>
  </button>
</template>

<style scoped>
.carte { color: inherit; }
.coin { position: absolute; top: 9px; left: 9px; z-index: 3; }
.pied { display: flex; flex-direction: column; gap: 4px; padding: 0 2px; min-width: 0; }
.ligne { font-size: 12px; color: var(--text-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.pied .demo-dispo { align-self: flex-start; }

/* Remise et note Steam, empilées en haut à droite de la jaquette. Seule, la remise s'y
   place d'elle-même (`.cover-card .badge`, style.css) ; dans la pile, c'est la pile qui se
   place. Sous la pastille « Dans ta bibliothèque », qui prend presque toute la largeur
   d'une jaquette, elle descend d'un cran — même règle que la remise seule. */
.pile {
  position: absolute; top: 10px; right: 10px; z-index: 2;
  display: flex; flex-direction: column; align-items: flex-end; gap: 6px;
}
.pile.sous-coin { top: 40px; }
.pile .badge { position: static; }
/* Posée sur l'image dans les deux thèmes : fond sombre, et les teintes claires des notes
   (celles du thème sombre), lisibles dessus. */
.note {
  display: inline-flex; align-items: center; gap: 5px; padding: 3px 8px; border-radius: 8px;
  font-family: var(--mono); font-weight: 700; font-size: 12px; color: #fff;
  background: rgba(14, 10, 20, 0.62); border: 1px solid rgba(255, 255, 255, 0.18); backdrop-filter: blur(6px);
}
.note svg { width: 13px; height: 13px; flex: none; }
.note.pos svg { color: #56d6a6; }
.note.mixte svg { color: #f0b73f; }
.note.neg svg { color: #ff4655; }

.carte-paysage {
  display: flex; flex-direction: column; gap: 8px; min-width: 0; width: 100%;
  background: none; border: none; padding: 0; text-align: left; cursor: pointer;
}
.visuel {
  position: relative; display: block; aspect-ratio: 460 / 215; border-radius: 12px; overflow: hidden;
  border: 1px solid var(--border); box-shadow: var(--shadow-card);
  transition: transform 0.22s cubic-bezier(0.2, 0.7, 0.3, 1);
}
.visuel img { width: 100%; height: 100%; object-fit: cover; display: block; }
.carte-paysage:hover .visuel { transform: translateY(-4px); }
.titre { font-size: 13.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ligne-prix { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-faint); min-width: 0; }
</style>
