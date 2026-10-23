<script setup lang="ts">
import { computed, ref } from "vue";
import { etiquetteIntl, t } from "../i18n";
import type { SteamAchievement, SteamAchievements } from "../types";

/**
 * Succès Steam d'un jeu : les plus rares qu'on a débloqués, les prochains à viser (les
 * plus courants parmi ceux qui manquent : les plus faciles à aller chercher), et la liste
 * complète triée par rareté. La rareté vient de la page des succès elle-même
 * (`accounts/steam.rs`), sans appel de plus.
 */
const props = defineProps<{ succes: SteamAchievements | null; chargement: boolean }>();

const items = computed(() => props.succes?.items ?? []);
const parRarete = (a: SteamAchievement, b: SteamAchievement) => (a.rarete ?? 101) - (b.rarete ?? 101);
const rares = computed(() => items.value.filter((a) => a.unlocked && a.rarete != null).sort(parRarete).slice(0, 3));
const aViser = computed(() =>
  items.value
    .filter((a) => !a.unlocked && !a.cache)
    .sort((a, b) => parRarete(b, a))
    .slice(0, 3),
);
const toutDebloque = computed(() => !!props.succes && props.succes.total > 0 && props.succes.unlocked === props.succes.total);

const tous = ref(false);
const liste = computed(() => items.value.filter((a) => !a.cache).sort(parRarete));
const caches = computed(() => items.value.filter((a) => a.cache).length);

const part = (r: number) =>
  t("fiche.succesFiche.part", {
    pct: new Intl.NumberFormat(etiquetteIntl.value, { style: "percent", maximumFractionDigits: 1 }).format(r / 100),
  });
const debloque = (s: number) =>
  t("fiche.succesFiche.debloqueLe", {
    date: new Intl.DateTimeFormat(etiquetteIntl.value, { day: "numeric", month: "short", year: "numeric" }).format(new Date(s * 1000)),
  });
/** Moins de 10 % des joueurs : un succès qui se remarque. */
const rare = (a: SteamAchievement) => a.rarete != null && a.rarete < 10;

const groupes = computed(() =>
  tous.value
    ? [{ cle: "tous", titre: "", lignes: liste.value }]
    : [
        { cle: "rares", titre: t("fiche.succesFiche.rares"), lignes: rares.value },
        { cle: "aViser", titre: t("fiche.succesFiche.aViser"), lignes: aViser.value },
      ].filter((g) => g.lignes.length),
);

function cacher(e: Event) {
  (e.target as HTMLElement).style.visibility = "hidden";
}
</script>

<template>
  <section class="succes">
    <div class="tete">
      <h3>{{ t("fiche.succes") }}</h3>
      <span v-if="succes" class="compte">{{ succes.unlocked }} / {{ succes.total }}</span>
      <button v-if="succes && liste.length > 1" class="lien" :aria-expanded="tous" @click="tous = !tous">
        {{ tous ? t("fiche.succesFiche.masquer") : t("fiche.succesFiche.tous", { n: succes.total }) }}
      </button>
    </div>

    <p v-if="chargement && !succes" class="attente">{{ t("fiche.chargementSucces") }}</p>
    <template v-else-if="succes">
      <p v-if="!tous && toutDebloque" class="bravo">{{ t("fiche.succesFiche.toutDebloque") }}</p>
      <div :class="tous ? 'liste' : 'cartes'">
        <div v-for="g in groupes" :key="g.cle" :class="{ carte: !tous }">
          <span v-if="g.titre" class="carte-titre">{{ g.titre }}</span>
          <div v-for="(a, i) in g.lignes" :key="`${g.cle}-${i}-${a.name}`" class="ligne" :class="{ verrouille: !a.unlocked }">
            <img :src="a.icon" alt="" loading="lazy" @error="cacher" />
            <span class="infos">
              <span class="nom">{{ a.name }}</span>
              <span v-if="a.description" class="desc">{{ a.description }}</span>
              <span v-if="a.unlocked && a.unlockedAt" class="meta">{{ debloque(a.unlockedAt) }}</span>
              <span v-else-if="a.progression" class="meta progression">
                <span class="barre"><span :style="{ width: `${(a.progression.actuel / a.progression.max) * 100}%` }" /></span>
                {{ a.progression.actuel }} / {{ a.progression.max }}
              </span>
            </span>
            <span v-if="a.rarete != null" class="part" :class="{ rare: rare(a) }">{{ part(a.rarete) }}</span>
          </div>
        </div>
        <div v-if="tous && caches" class="ligne verrouille caches">
          <span class="cadenas" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="11" width="14" height="10" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></svg>
          </span>
          <span class="infos">
            <span class="nom">{{ t("fiche.succesFiche.caches", { n: caches }) }}</span>
            <span class="desc">{{ t("fiche.succesFiche.cachesAide") }}</span>
          </span>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.succes { display: flex; flex-direction: column; gap: 14px; }
.tete { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
h3 { margin: 0; font-size: 20px; font-weight: 800; letter-spacing: -0.02em; color: var(--text); }
.compte { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
.lien { margin-left: auto; background: none; border: none; padding: 0; color: var(--accent); font-size: 13px; font-weight: 600; }
.lien:hover { text-decoration: underline; }
.attente { margin: 0; color: var(--text-faint); font-style: italic; font-size: 14px; }
.bravo { margin: 0; font-size: 14px; color: var(--text-dim); }

.cartes { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; }
.carte {
  display: flex; flex-direction: column; gap: 12px; padding: 14px 16px; border-radius: 14px;
  background: var(--surface); border: 1px solid var(--border); min-width: 0;
}
.carte-titre { font-size: 11.5px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-faint); font-weight: 700; }
.liste { display: flex; flex-direction: column; gap: 12px; }
.liste > div { display: flex; flex-direction: column; gap: 12px; }

.ligne { display: flex; align-items: center; gap: 12px; min-width: 0; }
.ligne img, .cadenas {
  width: 48px; height: 48px; border-radius: 10px; flex: none; display: block; object-fit: cover;
  background: var(--surface-2); border: 1px solid var(--border);
}
.cadenas { display: grid; place-items: center; color: var(--text-faint); }
.cadenas svg { width: 20px; height: 20px; }
.ligne.verrouille img { opacity: 0.7; }
.infos { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.nom { font-size: 14px; font-weight: 700; color: var(--text); }
.verrouille .nom { color: var(--text-dim); }
.desc { font-size: 12.5px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.meta { font-family: var(--mono); font-size: 11px; color: var(--text-faint); margin-top: 2px; }
.progression { display: flex; align-items: center; gap: 8px; }
.barre { width: 90px; height: 5px; border-radius: 99px; background: var(--surface-3); overflow: hidden; }
.barre span { display: block; height: 100%; border-radius: 99px; background: var(--accent); }
.part {
  flex: none; font-size: 11.5px; font-weight: 700; padding: 3px 8px; border-radius: 7px; white-space: nowrap;
  background: var(--surface-2); color: var(--text-dim);
}
.part.rare { background: var(--accent-soft); color: var(--accent); }
</style>
