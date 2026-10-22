<script setup lang="ts">
import { usePossession } from "../composables/usePossession";
import { useStore } from "../composables/useStore";
import { libelleFin } from "../lib/boutique";
import { gradientFor } from "../lib/covers";
import { formatPrix } from "../lib/format";
import { t } from "../i18n";
import type { StoreItem } from "../types";
import StorePossede from "./StorePossede.vue";

/**
 * Jeux offerts en ce moment (0 € chez un vendeur, pour une durée limitée). Un jeu déjà
 * dans la bibliothèque est signalé : c'est en général qu'on l'a déjà récupéré.
 */
defineProps<{ offres: StoreItem[]; chargement: boolean }>();

const { openProduct } = useStore();
const { possession } = usePossession();
</script>

<template>
  <aside class="offerts">
    <div>
      <h2>{{ t("boutique.accueil.offerts.titre") }}</h2>
      <p class="sous">{{ t("boutique.accueil.offerts.sousTitre") }}</p>
    </div>
    <template v-if="offres.length">
      <button v-for="o in offres.slice(0, 5)" :key="o.gameId" class="cadeau" @click="openProduct(o.gameId)">
        <span class="boite" :style="{ background: gradientFor(o.gameId) }">
          <img v-if="o.coverUrl" :src="o.coverUrl" alt="" loading="lazy" />
        </span>
        <span class="texte">
          <span class="nom">{{ o.title }}</span>
          <span class="prix">
            <span class="gratuit">{{ t("boutique.accueil.carte.gratuit") }}</span>
            <span v-if="o.normalPrice > 0" class="was">{{ formatPrix(o.normalPrice, o.currency) }}</span>
          </span>
          <span class="ou">{{ o.storeName }}<template v-if="libelleFin(o.expiry)"> · {{ libelleFin(o.expiry) }}</template></span>
          <StorePossede v-if="possession(o.title)" :etat="possession(o.title)!" class="deja" />
        </span>
      </button>
    </template>
    <p v-else-if="!chargement" class="aucun">{{ t("boutique.accueil.offerts.aucun") }}</p>
  </aside>
</template>

<style scoped>
.offerts {
  display: flex; flex-direction: column; gap: 10px; padding: 18px; border-radius: 16px;
  background: var(--surface); border: 1px solid var(--border); min-width: 0; align-self: flex-start;
}
h2 { margin: 0; font-size: 16px; font-weight: 800; }
.sous { margin: 3px 0 0; font-size: 12.5px; color: var(--text-faint); }
.cadeau {
  display: flex; gap: 12px; align-items: center; padding: 6px; border-radius: 10px; width: 100%;
  background: none; border: none; color: inherit; text-align: left; cursor: pointer;
}
.cadeau:hover { background: var(--surface-2); }
.boite { width: 52px; aspect-ratio: 2 / 3; flex: none; border-radius: 7px; overflow: hidden; border: 1px solid var(--border); }
.boite img { width: 100%; height: 100%; object-fit: cover; display: block; }
.texte { display: flex; flex-direction: column; align-items: flex-start; gap: 5px; min-width: 0; }
.nom { font-size: 13.5px; font-weight: 600; }
.ou { font-size: 11.5px; color: var(--text-faint); }
.deja { background: var(--surface-3); color: var(--text); border-color: var(--border); }
.aucun { margin: 0; font-size: 13px; color: var(--text-faint); }
</style>
