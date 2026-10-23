<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useContenusAdditionnels } from "../composables/useContenusAdditionnels";
import { useQuandVisible } from "../composables/useQuandVisible";
import { useStore } from "../composables/useStore";
import { pageSteam } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { openExternal } from "../lib/tauri";
import { t } from "../i18n";
import type { StoreShelfItem } from "../types";
import StorePages from "./StorePages.vue";

/**
 * Contenus additionnels d'un jeu possédé sur Steam : ceux qu'on a (marqués), puis ceux
 * qui manquent avec leur meilleur prix. Par pages de cinq, en onglets.
 *
 * Les prix passent par le comparateur, dont le quota est partagé par tous les
 * utilisateurs : rien n'est demandé tant que la section n'arrive pas à l'écran, et
 * seulement pour la page affichée.
 */
const props = defineProps<{ appId: number; dlc: number[]; possedes: number[] }>();

const PAR_PAGE = 5;

const { charger: chargerDlc, entree: entreeDlc } = useContenusAdditionnels();
const { openProduct } = useStore();

const etat = computed(() => entreeDlc(props.appId));
/** Les contenus que Steam décrit encore, dans l'ordre : possédés d'abord. */
const liste = computed(() => props.dlc.filter((id) => !etat.value.absents.includes(id)));
const page = ref(0);
const pages = computed(() => Math.ceil(liste.value.length / PAR_PAGE));
const visibles = computed(() => liste.value.slice(page.value * PAR_PAGE, (page.value + 1) * PAR_PAGE));
/** Les lignes affichées : le contenu une fois reçu, `null` tant que son prix est en route. */
const lignes = computed(() => {
  const parId = new Map(etat.value.jeux.map((j) => [j.appId, j]));
  return visibles.value.map((id) => ({ id, jeu: parId.get(id) ?? null }));
});

const racine = ref<HTMLElement | null>(null);
const aLEcran = ref(false);
useQuandVisible(racine, () => (aLEcran.value = true));
watch([aLEcran, visibles], () => {
  if (aLEcran.value) void chargerDlc(props.appId, visibles.value);
});

const possede = (j: StoreShelfItem) => props.possedes.includes(j.appId);

/** Un contenu qu'on n'a pas s'ouvre dans la Boutique (comparatif de prix) ; sinon, sur Steam. */
function ouvrir(j: StoreShelfItem) {
  if (!possede(j) && j.gameId) void openProduct(j.gameId);
  else void openExternal(pageSteam(j.appId));
}
const prix = (m: number, j: StoreShelfItem) => formatPrix(m, j.currency);
</script>

<template>
  <section ref="racine" class="dlc">
    <div class="tete">
      <h3>{{ t("fiche.dlc.titre") }}</h3>
      <span class="compte">
        {{ liste.length }}<template v-if="possedes.length"> · {{ t("fiche.dlc.possedes", { n: possedes.length }) }}</template>
      </span>
    </div>

    <template v-for="{ id, jeu: j } in lignes" :key="id">
      <button v-if="j" class="ligne" @click="ouvrir(j)">
        <span class="vignette" :style="{ background: gradientFor(String(id)) }">
          <img v-if="j.headerUrl" :src="j.headerUrl" alt="" loading="lazy" />
        </span>
        <span class="nom">{{ j.title }}</span>
        <span v-if="possede(j)" class="possede">✓ {{ t("fiche.dlc.possede") }}</span>
        <span v-else-if="j.isFree" class="gratuit">{{ t("boutique.accueil.carte.gratuit") }}</span>
        <span v-else-if="j.price != null" class="prix">
          <span v-if="j.savings > 0" class="remise">-{{ j.savings }}%</span>
          <span v-if="j.savings > 0 && j.normalPrice" class="was">{{ prix(j.normalPrice, j) }}</span>
          <span class="now">{{ prix(j.price, j) }}</span>
          <span class="shop">{{ j.storeName }}</span>
        </span>
        <span v-else class="faible">{{ t("boutique.accueil.carte.pasDOffre") }}</span>
      </button>
      <div v-else-if="etat.etat !== 'echec'" class="ligne fantome" aria-hidden="true" />
    </template>

    <div v-if="etat.etat === 'echec'" class="echec">
      {{ t("boutique.accueil.echec") }}
      <button class="chip" @click="chargerDlc(appId, visibles)">{{ t("boutique.accueil.reessayer") }}</button>
    </div>

    <StorePages v-model="page" class="suite" :pages="pages" :libelle="t('fiche.dlc.pages')" />
  </section>
</template>

<style scoped>
.dlc { display: flex; flex-direction: column; gap: 10px; }
.tete { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; margin-bottom: 4px; }
h3 { margin: 0; font-size: 20px; font-weight: 800; letter-spacing: -0.02em; color: var(--text); }
.compte { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
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
.possede {
  flex: none; font-size: 11.5px; font-weight: 700; padding: 3px 9px; border-radius: 7px;
  background: color-mix(in srgb, var(--manual) 18%, transparent); color: var(--manual);
}
.faible { font-size: 12px; color: var(--text-faint); }
.fantome { height: 78px; cursor: default; }
.echec { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--text-faint); }
.suite { margin-top: 2px; }
</style>
