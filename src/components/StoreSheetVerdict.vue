<script setup lang="ts">
import { computed } from "vue";
import { t, type Cle } from "../i18n";
import { verdict } from "../lib/boutique";
import { formatPrix } from "../lib/format";

/**
 * « Bon moment pour acheter ? » : la meilleure offre comparée aux plus bas des 3
 * derniers mois, de l'année et de tous les temps (données déjà reçues avec les prix).
 */
const props = defineProps<{
  prix: number;
  devise?: string;
  bas3m?: number | null;
  bas1a?: number | null;
  basTout?: number | null;
}>();

const v = computed(() => verdict(props.prix, props.bas3m, props.bas1a, props.basTout));
const ton = computed(() => (v.value === "cher" ? "cher" : v.value === "annee" ? "neutre" : "bon"));
const prix = (m: number) => formatPrix(m, props.devise);

const detail = computed(() => {
  if (v.value === "proche" && props.basTout != null) {
    return t("boutique.produit.ecart", { ecart: prix(Math.round((props.prix - props.basTout) * 100) / 100) });
  }
  if (v.value === "record" && props.bas3m != null && props.basTout != null && props.bas3m <= props.basTout + 0.01) {
    return t("boutique.produit.recordAtteint");
  }
  return "";
});

const bas = computed(() =>
  [
    { cle: "bas3m", valeur: props.bas3m },
    { cle: "bas1a", valeur: props.bas1a },
    { cle: "basTout", valeur: props.basTout },
  ].filter((b): b is { cle: string; valeur: number } => b.valeur != null),
);
</script>

<template>
  <div v-if="v" class="verdict" :class="ton">
    <span class="titre">{{ t("boutique.produit.verdictTitre") }}</span>
    <span class="ligne">
      <svg v-if="ton === 'bon'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 7.5v5M12 16v.01" /></svg>
      {{ t(`boutique.produit.verdict.${v}` as Cle) }}
    </span>
    <span v-if="detail" class="detail">{{ detail }}</span>
    <div class="bas">
      <span v-for="b in bas" :key="b.cle" class="case">
        <span class="nom">{{ t(`boutique.produit.${b.cle}` as Cle) }}</span>
        <b>{{ prix(b.valeur) }}</b>
      </span>
    </div>
  </div>
</template>

<style scoped>
.verdict {
  display: flex; flex-direction: column; gap: 8px; padding: 16px 18px; border-radius: 18px;
  background: var(--surface); border: 1px solid var(--border);
}
.verdict.bon { background: color-mix(in srgb, var(--manual) 9%, var(--surface)); border-color: color-mix(in srgb, var(--manual) 40%, var(--border)); }
.verdict.cher { background: color-mix(in srgb, var(--detected) 9%, var(--surface)); border-color: color-mix(in srgb, var(--detected) 40%, var(--border)); }
.titre { font-size: 12px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; }
.ligne { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px; color: var(--text); }
.ligne svg { width: 18px; height: 18px; flex: none; }
.bon .ligne svg { color: var(--manual); }
.cher .ligne svg { color: var(--detected); }
.detail { font-size: 12.5px; color: var(--text-dim); }
.bas { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; margin-top: 2px; }
.case { display: flex; flex-direction: column; gap: 2px; padding: 8px; border-radius: 10px; background: var(--bg); }
.case .nom { font-size: 11px; color: var(--text-faint); }
.case b { font-family: var(--mono); font-size: 13.5px; color: var(--text); }
</style>
