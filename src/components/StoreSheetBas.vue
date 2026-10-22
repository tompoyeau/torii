<script setup lang="ts">
import { computed, ref } from "vue";
import { useFicheExtras } from "../composables/useFicheExtras";
import { usePossession } from "../composables/usePossession";
import { useQuandVisible } from "../composables/useQuandVisible";
import { useStore } from "../composables/useStore";
import { pageSteam } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { openExternal } from "../lib/tauri";
import { t } from "../i18n";
import type { StoreGame, StoreShelfItem } from "../types";
import StorePossede from "./StorePossede.vue";
import StoreRow from "./StoreRow.vue";
import StoreShelfCard from "./StoreShelfCard.vue";

/**
 * Bas de la fiche produit : contenus additionnels et jeux similaires. Rien n'est demandé
 * tant que l'utilisateur ne fait pas défiler la fiche jusque-là (un appel au comparateur
 * pour les deux, partagé par tous les utilisateurs de Torii).
 */
const props = defineProps<{
  jeu: StoreGame;
  /** La fiche place la configuration requise entre les deux : chaque partie est montée à sa place. */
  partie: "dlc" | "similaires";
}>();

const { charger, entree } = useFicheExtras();
const { openProduct } = useStore();
const { possession } = usePossession();

const racine = ref<HTMLElement | null>(null);
useQuandVisible(racine, () => void charger(props.jeu));

const etat = computed(() => entree(props.jeu.gameId));
const dlc = computed(() => etat.value.extras?.dlc ?? []);
const similaires = computed(() => etat.value.extras?.similar ?? []);
const tags = computed(() => etat.value.extras?.similarTags ?? []);
const attendus = computed(() => props.jeu.steam?.dlc.length ?? 0);

function ouvrir(j: StoreShelfItem) {
  if (j.gameId) void openProduct(j.gameId);
  else void openExternal(pageSteam(j.appId));
}
const prix = (m: number, j: StoreShelfItem) => formatPrix(m, j.currency);
</script>

<template>
  <div ref="racine" class="bas">
    <section v-if="partie === 'dlc' && (dlc.length || (attendus && etat.etat === 'chargement'))" class="dlc">
      <h4>{{ t("boutique.produit.contenus") }} · {{ dlc.length || attendus }}</h4>
      <template v-if="dlc.length">
        <button v-for="j in dlc" :key="j.appId" class="ligne" @click="ouvrir(j)">
          <span class="vignette" :style="{ background: gradientFor(String(j.appId)) }">
            <img v-if="j.headerUrl" :src="j.headerUrl" alt="" loading="lazy" />
          </span>
          <span class="nom">{{ j.title }}</span>
          <StorePossede v-if="possession(j.title, j.appId)" :etat="possession(j.title, j.appId)!" class="deja" />
          <span v-if="j.isFree" class="gratuit">{{ t("boutique.accueil.carte.gratuit") }}</span>
          <span v-else-if="j.price != null" class="prix">
            <span v-if="j.savings > 0" class="remise">-{{ j.savings }}%</span>
            <span v-if="j.savings > 0 && j.normalPrice" class="was">{{ prix(j.normalPrice, j) }}</span>
            <span class="now">{{ prix(j.price, j) }}</span>
            <span class="shop">{{ j.storeName }}</span>
          </span>
          <span v-else class="faible">{{ t("boutique.accueil.carte.pasDOffre") }}</span>
        </button>
      </template>
      <template v-else>
        <div v-for="n in Math.min(attendus, 3)" :key="n" class="ligne fantome" aria-hidden="true" />
      </template>
    </section>

    <StoreRow
      v-if="partie === 'similaires' && similaires.length"
      :titre="t('boutique.produit.similaires')"
      :sous-titre="tags.length ? t('boutique.produit.similairesSous', { tags: tags.join(', ') }) : undefined"
      :nombre="similaires.length"
    >
      <StoreShelfCard v-for="j in similaires" :key="j.appId" :jeu="j" />
    </StoreRow>

    <div v-if="partie === 'similaires' && etat.etat === 'echec'" class="echec">
      {{ t("boutique.accueil.echec") }}
      <button class="chip" @click="charger(jeu, true)">{{ t("boutique.accueil.reessayer") }}</button>
    </div>
  </div>
</template>

<style scoped>
.bas { display: flex; flex-direction: column; gap: 34px; min-height: 1px; }
h4 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint); font-weight: 700; margin: 0 0 14px; }
.dlc { display: flex; flex-direction: column; gap: 8px; }
.dlc h4 { margin-bottom: 6px; }
.ligne {
  display: flex; align-items: center; gap: 14px; padding: 8px 10px; border-radius: 12px; width: 100%;
  background: var(--surface); border: 1px solid var(--border); color: inherit; text-align: left; cursor: pointer; min-width: 0;
}
.ligne:hover { border-color: var(--border-strong); }
.vignette { width: 132px; aspect-ratio: 460 / 215; flex: none; border-radius: 8px; overflow: hidden; }
.vignette img { width: 100%; height: 100%; object-fit: cover; display: block; }
.nom { flex: 1; min-width: 0; font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ligne .prix { flex: none; }
.ligne .prix .shop { margin-left: 4px; }
.faible { font-size: 12px; color: var(--text-faint); }
.deja { background: var(--surface-3); color: var(--text); border-color: var(--border); }
.fantome { height: 78px; cursor: default; }
.echec { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--text-faint); }
</style>
