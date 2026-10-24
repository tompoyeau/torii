<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useContenusAdditionnels } from "../composables/useContenusAdditionnels";
import { useFicheExtras } from "../composables/useFicheExtras";
import { usePossession } from "../composables/usePossession";
import { useQuandVisible } from "../composables/useQuandVisible";
import { useStore } from "../composables/useStore";
import { pageSteam } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { hasTauriRuntime } from "../lib/tauri";
import { t } from "../i18n";
import type { StoreGame, StoreShelfItem } from "../types";
import StorePossede from "./StorePossede.vue";
import StoreRow from "./StoreRow.vue";
import StorePages from "./StorePages.vue";
import StoreShelfCard from "./StoreShelfCard.vue";
import { ouvrirPage } from "../composables/usePageWeb";

/**
 * Bas de la fiche produit : contenus additionnels et jeux similaires. Rien n'est demandé
 * tant que l'utilisateur ne fait pas défiler la fiche jusque-là.
 *
 * Les contenus additionnels sont TOUS là (Hunt: Showdown en a plus de 80), feuilletés par
 * pages de cinq : seuls ceux de la page ouverte passent par le comparateur, dont le quota
 * est partagé par tous les utilisateurs de Torii (`useContenusAdditionnels`, commun avec
 * la fiche d'un jeu possédé). Les jeux similaires ont leur propre appel.
 */
const props = defineProps<{
  jeu: StoreGame;
  /** La fiche place la configuration requise entre les deux : chaque partie est montée à sa place. */
  partie: "dlc" | "similaires";
}>();

const { charger, entree } = useFicheExtras();
const { charger: chargerDlc, entree: entreeDlc } = useContenusAdditionnels();
const { openProduct } = useStore();
const { possession } = usePossession();

const racine = ref<HTMLElement | null>(null);
const aLEcran = ref(false);
useQuandVisible(racine, () => {
  aLEcran.value = true;
  if (props.partie === "similaires") void charger(props.jeu);
});

/* --- Jeux similaires --- */
const etat = computed(() => entree(props.jeu.gameId));
const similaires = computed(() => etat.value.extras?.similar ?? []);
const tags = computed(() => etat.value.extras?.similarTags ?? []);

/* --- Contenus additionnels, par pages de cinq --- */
const PAR_PAGE = 5;
const appId = computed(() => props.jeu.appId ?? 0);
const etatDlc = computed(() => entreeDlc(appId.value));
/** Les contenus que Steam décrit encore (un contenu retiré de la vente est sauté). */
const liste = computed(() => (props.jeu.steam?.dlc ?? []).filter((id) => !etatDlc.value.absents.includes(id)));
const page = ref(0);
const pages = computed(() => Math.ceil(liste.value.length / PAR_PAGE));
const visibles = computed(() => liste.value.slice(page.value * PAR_PAGE, (page.value + 1) * PAR_PAGE));
/** Les lignes affichées : le contenu une fois reçu, `null` tant que son prix est en route. */
const lignes = computed(() => {
  const parId = new Map(etatDlc.value.jeux.map((j) => [j.appId, j]));
  return visibles.value.map((id) => ({ id, jeu: parId.get(id) ?? null }));
});
// Hors Tauri (démo web), pas de comparateur : la section reste vide plutôt qu'en attente.
const actif = props.partie === "dlc" && hasTauriRuntime();
watch([aLEcran, visibles], () => {
  if (actif && aLEcran.value && appId.value) void chargerDlc(appId.value, visibles.value);
});

function ouvrir(j: StoreShelfItem) {
  if (j.gameId) void openProduct(j.gameId);
  else ouvrirPage(pageSteam(j.appId));
}
const prix = (m: number, j: StoreShelfItem) => formatPrix(m, j.currency);
</script>

<template>
  <div ref="racine" class="bas">
    <section v-if="actif && liste.length" class="dlc">
      <h4>{{ t("boutique.produit.contenus") }} · {{ liste.length }}</h4>
      <template v-for="{ id, jeu: j } in lignes" :key="id">
        <button v-if="j" class="ligne" @click="ouvrir(j)">
          <span class="vignette" :style="{ background: gradientFor(String(id)) }">
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
        <div v-else-if="etatDlc.etat !== 'echec'" class="ligne fantome" aria-hidden="true" />
      </template>
      <div v-if="etatDlc.etat === 'echec'" class="echec">
        {{ t("boutique.accueil.echec") }}
        <button class="chip" @click="chargerDlc(appId, visibles)">{{ t("boutique.accueil.reessayer") }}</button>
      </div>
      <StorePages v-model="page" class="suite" :pages="pages" :libelle="t('fiche.dlc.pages')" />
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
.suite { margin-top: 2px; }
.echec { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--text-faint); }
</style>
