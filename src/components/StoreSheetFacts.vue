<script setup lang="ts">
import { computed, ref } from "vue";
import { langueCourante, t } from "../i18n";
import { nombre, nomLangue } from "../lib/boutique";
import type { StoreSteamSheet } from "../types";

/**
 * « En bref » : ce qu'on vérifie avant d'acheter, en cartes. Une carte sans donnée
 * n'apparaît pas — mieux vaut quatre cartes justes que six dont deux vides.
 */
const props = defineProps<{
  steam?: StoreSteamSheet | null;
  joueursJour?: number | null;
  joueursRecord?: number | null;
  suivis?: number | null;
}>();

/** Fonctionnalités Steam, par famille (identifiants des catégories du magasin Steam). */
const MODES = [2, 1, 49, 36, 47, 37, 9, 38, 48, 39, 24, 27, 20];
const MANETTE = [28, 18];
const MANETTES_PRECISES = [55, 56, 57, 58];
const ACCES = [64, 65, 66, 67, 68, 71, 72, 73, 74, 75, 76, 78, 79];
const CLOUD = 23;

const categories = computed(() => props.steam?.categories ?? []);
const libelles = (ids: number[]) => {
  const vus = new Set<string>();
  return ids
    .map((id) => categories.value.find((c) => c.id === id)?.libelle)
    .filter((l): l is string => !!l && !vus.has(l) && !!vus.add(l));
};

/** La langue de l'interface parmi celles du jeu. */
const maLangue = computed(() => {
  const code = langueCourante.value;
  return props.steam?.langues.find((l) => l.code === code || l.code.startsWith(`${code}-`)) ?? null;
});
const toutesLangues = ref(false);

interface Carte {
  cle: string;
  titre: string;
  valeur: string;
  detail?: string;
}
const cartes = computed<Carte[]>(() => {
  const s = props.steam;
  const out: Carte[] = [];
  if (s) {
    const modes = libelles(MODES);
    const extras = [s.succes ? t("boutique.produit.succes", { n: s.succes }) : "", ...libelles([CLOUD])].filter(Boolean);
    if (modes.length) out.push({ cle: "modes", titre: t("boutique.produit.modes"), valeur: modes.slice(0, 3).join(" · "), detail: extras.join(" · ") });
    const manette = libelles(MANETTE);
    if (manette.length) out.push({ cle: "manette", titre: t("boutique.produit.manette"), valeur: manette[0], detail: libelles(MANETTES_PRECISES).join(", ") });
    if (s.langues.length) {
      const l = maLangue.value;
      const nom = nomLangue(langueCourante.value);
      out.push({
        cle: "langue",
        titre: t("boutique.produit.taLangue"),
        valeur: !l
          ? t("boutique.produit.langueAbsente")
          : l.voix
            ? t("boutique.produit.langueVoix", { langue: nom })
            : l.sousTitres
              ? t("boutique.produit.langueSousTitres", { langue: nom })
              : t("boutique.produit.langueInterface", { langue: nom }),
      });
    }
  }
  if (props.joueursJour) {
    out.push({
      cle: "joueurs",
      titre: t("boutique.produit.joueurs"),
      valeur: t("boutique.produit.joueursJour", { nombre: nombre(props.joueursJour) }),
      detail: props.joueursRecord ? t("boutique.produit.joueursRecord", { nombre: nombre(props.joueursRecord) }) : "",
    });
  }
  if (s) {
    const acces = libelles(ACCES);
    if (acces.length) {
      out.push({
        cle: "acces",
        titre: t("boutique.produit.accessibilite"),
        valeur: acces.slice(0, 2).join(", "),
        detail: acces.length > 2 ? t("boutique.produit.autresOptions", { n: acces.length - 2 }) : "",
      });
    }
  }
  if (props.suivis) {
    out.push({
      cle: "suivis",
      titre: t("boutique.produit.suiviPar"),
      valeur: t("boutique.produit.suiviParValeur", { n: props.suivis, nombre: nombre(props.suivis) }),
      detail: t("boutique.produit.suiviParAide"),
    });
  }
  return out;
});

const autresLangues = computed(() => Math.max(0, (props.steam?.langues.length ?? 0) - (maLangue.value ? 1 : 0)));
</script>

<template>
  <section v-if="cartes.length" class="bref">
    <h4>{{ t("boutique.produit.enBref") }}</h4>
    <div class="cartes">
      <div v-for="c in cartes" :key="c.cle" class="carte">
        <span class="titre">{{ c.titre }}</span>
        <span class="valeur">{{ c.valeur }}</span>
        <span v-if="c.detail" class="detail">{{ c.detail }}</span>
        <button v-if="c.cle === 'langue' && autresLangues" class="lien" :aria-expanded="toutesLangues" @click="toutesLangues = !toutesLangues">
          {{ toutesLangues ? t("boutique.produit.toutesLangues") : t("boutique.produit.autresLangues", { n: autresLangues }) }}
        </button>
      </div>
    </div>
    <table v-if="toutesLangues && steam" class="langues">
      <thead>
        <tr>
          <th scope="col">{{ t("boutique.produit.toutesLangues") }}</th>
          <th scope="col">{{ t("boutique.produit.voix") }}</th>
          <th scope="col">{{ t("boutique.produit.sousTitres") }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="l in steam.langues" :key="l.code">
          <td>{{ nomLangue(l.code) }}</td>
          <td>{{ l.voix ? "✓" : "—" }}</td>
          <td>{{ l.sousTitres ? "✓" : "—" }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.bref h4 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint); font-weight: 700; margin: 0 0 16px; }
.cartes { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 12px; }
.carte {
  display: flex; flex-direction: column; align-items: flex-start; gap: 5px; padding: 14px 16px; border-radius: 14px;
  background: var(--surface); border: 1px solid var(--border); min-width: 0;
}
.titre { font-size: 11.5px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-faint); font-weight: 700; }
.valeur { font-size: 14.5px; font-weight: 600; color: var(--text); }
.detail { font-size: 12px; color: var(--text-faint); }
.lien { background: none; border: none; padding: 0; color: var(--accent); font-size: 12.5px; font-weight: 600; }
.lien:hover { text-decoration: underline; }
.langues { margin-top: 14px; width: 100%; max-width: 520px; border-collapse: collapse; font-size: 13px; }
.langues th { text-align: left; font-size: 11.5px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-faint); padding: 6px 8px; }
.langues td { padding: 6px 8px; border-top: 1px solid var(--border); color: var(--text-dim); }
.langues td + td, .langues th + th { text-align: center; width: 110px; }
</style>
