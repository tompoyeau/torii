<script setup lang="ts">
import { computed, ref } from "vue";
import { usePossession } from "../composables/usePossession";
import { useStore } from "../composables/useStore";
import { useStoreHome, type Etat } from "../composables/useStoreHome";
import { libelleAvis, libelleSortie, pageSteam, tonAvis } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { openExternal } from "../lib/tauri";
import { t } from "../i18n";
import type { StoreShelfItem, StoreShelfKind } from "../types";
import StorePossede from "./StorePossede.vue";

/**
 * Classements façon Steam, en onglets : meilleures ventes (livrées avec l'accueil),
 * nouveautés populaires et jeux les plus attendus (demandés au premier clic).
 */
const props = defineProps<{ ventes: StoreShelfItem[]; etatVentes: Etat }>();

type Onglet = "ventes" | "nouveautes" | "aVenir";
const ONGLETS: { cle: Onglet; rayon: StoreShelfKind | null }[] = [
  { cle: "ventes", rayon: null },
  { cle: "nouveautes", rayon: "newReleases" },
  { cle: "aVenir", rayon: "upcoming" },
];

const { openProduct } = useStore();
const { chargerRayon, rayon } = useStoreHome();
const { possession } = usePossession();

const onglet = ref<Onglet>("ventes");
const courant = computed(() => ONGLETS.find((o) => o.cle === onglet.value)!);

const liste = computed(() =>
  courant.value.rayon ? rayon(courant.value.rayon).items : props.ventes,
);
const etat = computed<Etat>(() =>
  courant.value.rayon ? rayon(courant.value.rayon).etat : props.etatVentes,
);

function choisir(o: Onglet) {
  onglet.value = o;
  const r = ONGLETS.find((x) => x.cle === o)?.rayon;
  if (r) void chargerRayon(r);
}

/** Flèches gauche/droite entre onglets (motif WAI-ARIA « tabs »). */
const boutons = ref<HTMLButtonElement[]>([]);
function clavier(e: KeyboardEvent, i: number) {
  if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
  e.preventDefault();
  const j = (i + (e.key === "ArrowRight" ? 1 : -1) + ONGLETS.length) % ONGLETS.length;
  choisir(ONGLETS[j].cle);
  boutons.value[j]?.focus();
}

function reessayer() {
  if (courant.value.rayon) void chargerRayon(courant.value.rayon, undefined, true);
  else void useStoreHome().chargerAccueil(true);
}

function ouvrir(j: StoreShelfItem) {
  if (j.gameId) void openProduct(j.gameId);
  else void openExternal(pageSteam(j.appId));
}

const prix = (montant: number, j: StoreShelfItem) => formatPrix(montant, j.currency);
/** Ce qui suit l'appréciation : la date pour un jeu à venir ou récent, sinon les tags. */
const detail = (j: StoreShelfItem) =>
  j.comingSoon || onglet.value === "nouveautes" ? libelleSortie(j) : j.tags.join(", ");
</script>

<template>
  <section class="classements">
    <div role="tablist" :aria-label="t('boutique.accueil.classements.titre')" class="onglets no-scrollbar">
      <button
        v-for="(o, i) in ONGLETS"
        :id="`onglet-${o.cle}`"
        :key="o.cle"
        ref="boutons"
        role="tab"
        class="onglet"
        :class="{ actif: onglet === o.cle }"
        :aria-selected="onglet === o.cle"
        :tabindex="onglet === o.cle ? 0 : -1"
        aria-controls="classement-liste"
        @click="choisir(o.cle)"
        @keydown="clavier($event, i)"
      >
        {{ t(`boutique.accueil.classements.${o.cle}`) }}
      </button>
    </div>

    <div id="classement-liste" role="tabpanel" :aria-labelledby="`onglet-${onglet}`" class="liste">
      <template v-if="liste.length">
        <button v-for="(j, i) in liste.slice(0, 10)" :key="j.appId" class="ligne" @click="ouvrir(j)">
          <span class="rang">{{ i + 1 }}</span>
          <span class="vignette" :style="{ background: gradientFor(String(j.appId)) }">
            <img v-if="j.headerUrl" :src="j.headerUrl" alt="" loading="lazy" />
          </span>
          <span class="texte">
            <span class="nom">{{ j.title }}</span>
            <span class="sous">
              <span v-if="j.reviewScore" class="avis" :class="tonAvis(j.reviewScore)" :title="t('boutique.accueil.carte.avisAide', { n: j.reviewCount })">
                {{ libelleAvis(j.reviewScore) }}<template v-if="j.reviewPercent != null"> · {{ j.reviewPercent }} %</template>
              </span>
              <span v-if="detail(j)" class="detail">{{ detail(j) }}</span>
            </span>
          </span>
          <span v-if="possession(j.title, j.appId)" class="etat"><StorePossede :etat="possession(j.title, j.appId)!" /></span>
          <span class="tarif">
            <span v-if="j.isFree" class="gratuit">{{ t("boutique.accueil.carte.gratuit") }}</span>
            <span v-else-if="j.price != null" class="prix">
              <span v-if="j.savings > 0" class="remise">-{{ j.savings }}%</span>
              <span v-if="j.savings > 0 && j.normalPrice" class="was">{{ prix(j.normalPrice, j) }}</span>
              <span class="now">{{ prix(j.price, j) }}</span>
            </span>
            <span v-else class="faible">{{ t("boutique.accueil.carte.pasDOffre") }}</span>
            <span v-if="j.record" class="record">{{ t("boutique.accueil.carte.record") }}</span>
            <span v-else-if="j.storeName && j.price != null" class="faible mono">{{ j.storeName }}</span>
          </span>
        </button>
      </template>
      <div v-else-if="etat === 'echec'" class="vide">
        <p>{{ t("boutique.accueil.echec") }}</p>
        <button class="chip" @click="reessayer">{{ t("boutique.accueil.reessayer") }}</button>
      </div>
      <div v-else-if="etat === 'pret'" class="vide"><p>{{ t("boutique.accueil.vide") }}</p></div>
      <template v-else>
        <div v-for="n in 8" :key="n" class="ligne fantome" aria-hidden="true">
          <span class="rang" /><span class="vignette" /><span class="texte"><span class="barre" /><span class="barre courte" /></span>
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.classements { min-width: 0; display: flex; flex-direction: column; }
.onglets { display: flex; gap: 22px; border-bottom: 1px solid var(--border); margin-bottom: 8px; overflow-x: auto; }
.onglet {
  padding: 0 0 11px; border: none; background: none; color: var(--text-faint); white-space: nowrap;
  font-weight: 700; font-size: 16px; letter-spacing: -0.01em;
}
.onglet:hover { color: var(--text-dim); }
.onglet.actif { color: var(--text); box-shadow: inset 0 -2px 0 var(--accent); }
.liste { display: flex; flex-direction: column; gap: 2px; }
.ligne {
  display: flex; align-items: center; gap: 14px; padding: 8px 10px; border-radius: 12px; width: 100%;
  background: none; border: none; color: inherit; text-align: left; cursor: pointer; min-width: 0;
}
.ligne:hover { background: var(--surface); }
.rang { width: 22px; flex: none; text-align: right; font-family: var(--mono); font-size: 14px; font-weight: 700; color: var(--text-faint); }
.vignette { width: 124px; aspect-ratio: 460 / 215; flex: none; border-radius: 8px; overflow: hidden; border: 1px solid var(--border); }
.vignette img { width: 100%; height: 100%; object-fit: cover; display: block; }
.texte { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.nom { font-size: 14px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.sous { display: flex; gap: 8px; font-size: 12px; color: var(--text-faint); min-width: 0; }
.detail { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.etat { flex: none; }
.tarif { flex: none; display: flex; flex-direction: column; align-items: flex-end; gap: 4px; min-width: 96px; }
.faible { font-size: 12px; color: var(--text-faint); }
.mono { font-family: var(--mono); font-size: 10.5px; }
.vide { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 40px 0; color: var(--text-faint); }
.vide p { margin: 0; }
.fantome { cursor: default; }
.fantome .vignette { background: var(--surface-2); }
.barre { display: block; height: 11px; width: 60%; border-radius: 6px; background: var(--surface-2); }
.barre.courte { width: 35%; }
@media (max-width: 760px) {
  .vignette { width: 92px; }
  .etat { display: none; }
}
</style>
