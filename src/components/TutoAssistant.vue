<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { useFocusTrap } from "../composables/useFocusTrap";
import { useLangueRegion } from "../composables/useLangueRegion";
import { useLibrary } from "../composables/useLibrary";
import { useModaleDeNavigation } from "../composables/useNavigation";
import { useTorii } from "../composables/useTorii";
import { useTuto } from "../composables/useTuto";
import { LANGUES, langueCourante, t } from "../i18n";
import { deviseAttendue, nomDeRegion, regionDuSysteme } from "../i18n/regions";
import AccountsSettings from "./AccountsSettings.vue";

/**
 * L'assistant du premier lancement : on y règle vraiment les choses, et rien n'y est obligatoire.
 *
 * 🔑 LANGUE EN PREMIER. Une installation neuve démarre en anglais : quelqu'un qui ne le lit pas
 * doit pouvoir s'en sortir avant toute autre chose, d'où les noms de langue écrits dans leur langue.
 */
const { terminer, versVisite } = useTuto();
const { games, loading, reload } = useLibrary();
const { connected, account, openSignIn, signInOpen } = useTorii();
const { langueChoisie, choisirLangue, regionChoisie, choisirRegion, regionsTriees } = useLangueRegion();

/** Par le chemin de base : la démo n'est pas servie à la racine de son site. */
const logo = `${import.meta.env.BASE_URL}torii.svg`;

const TOTAL = 3;
const etape = ref(1);
const corps = ref<HTMLElement | null>(null);

/** Le compte s'ouvre par-dessus : c'est sa fenêtre qui garde alors le focus et le retour. */
const racine = ref<HTMLElement | null>(null);
useFocusTrap(racine, computed(() => !signInOpen.value));
useModaleDeNavigation({ ouverte: () => !signInOpen.value, fermer: terminer });

const jeux = computed(() => games.value.filter((g) => !g.hidden).length);

const langueAuDepart = langueCourante.value;

function allerA(n: number) {
  // Ce qui est déjà chargé l'a été dans l'ancienne langue. Sur une bibliothèque toute neuve, il
  // suffit de relire : pas besoin du redémarrage que proposent les Paramètres.
  if (etape.value === 1 && langueCourante.value !== langueAuDepart && !loading.value) reload();
  etape.value = n;
  void nextTick(() => {
    if (corps.value) corps.value.scrollTop = 0;
  });
}

const AVANTAGES = computed(() => [
  {
    cle: "partout",
    titre: t("tuto.compte.avantages.partout.titre"),
    texte: t("tuto.compte.avantages.partout.texte"),
    icone: "M8 2.5h8A1.5 1.5 0 0 1 17.5 4v16a1.5 1.5 0 0 1-1.5 1.5H8A1.5 1.5 0 0 1 6.5 20V4A1.5 1.5 0 0 1 8 2.5ZM11 18.5h2",
  },
  {
    cle: "amis",
    titre: t("tuto.compte.avantages.amis.titre"),
    texte: t("tuto.compte.avantages.amis.texte"),
    icone: "M9 11.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4ZM3.5 20a5.5 5.5 0 0 1 11 0M16 5.2a3 3 0 0 1 0 5.6M17.5 20a5.5 5.5 0 0 0-3-4.9",
  },
  {
    cle: "alertes",
    titre: t("tuto.compte.avantages.alertes.titre"),
    texte: t("tuto.compte.avantages.alertes.texte"),
    icone: "M6 10a6 6 0 1 1 12 0c0 5 2 6 2 7H4c0-1 2-2 2-7ZM10 20.5a2.2 2.2 0 0 0 4 0",
  },
  {
    cle: "suivi",
    titre: t("tuto.compte.avantages.suivi.titre"),
    texte: t("tuto.compte.avantages.suivi.texte"),
    icone: "M20.5 12a8.5 8.5 0 0 1-14.5 6M3.5 12A8.5 8.5 0 0 1 18 6M18 2.5V6h-3.5M6 21.5V18h3.5",
  },
]);
</script>

<template>
  <div ref="racine" class="fond" role="dialog" aria-modal="true" tabindex="-1" :aria-label="t('tuto.accueil.titre')" @keydown.esc.stop="terminer">
    <div class="assistant">
      <header class="tete">
        <span class="progression" aria-hidden="true">
          <span v-for="n in TOTAL" :key="n" class="point" :class="{ fait: n <= etape }" />
        </span>
        <span class="numero">{{ t("tuto.etape", { n: etape, total: TOTAL }) }}</span>
      </header>

      <section ref="corps" class="corps">
        <!-- Langue et région -->
        <template v-if="etape === 1">
          <img class="logo" :src="logo" alt="" />
          <h2>{{ t("tuto.accueil.titre") }}</h2>
          <p class="intro">{{ t("tuto.accueil.texte") }}</p>

          <div class="champ">
            <span class="libelle">{{ t("reglages.langue.titre") }}</span>
            <div class="segmented">
              <button :class="{ on: langueChoisie === 'system' }" @click="choisirLangue('system')">{{ t("reglages.langue.systeme") }}</button>
              <button
                v-for="l in LANGUES"
                :key="l.code"
                :class="{ on: langueChoisie === l.code }"
                :lang="l.code"
                @click="choisirLangue(l.code)"
              >
                {{ l.nom }}
              </button>
            </div>
          </div>

          <div class="champ">
            <span class="libelle">{{ t("reglages.region.titre") }}</span>
            <select
              class="region"
              :value="regionChoisie"
              :aria-label="t('reglages.region.titre')"
              @change="choisirRegion(($event.target as HTMLSelectElement).value)"
            >
              <option value="system">{{ t("reglages.region.systeme", { pays: nomDeRegion(regionDuSysteme()) }) }}</option>
              <option v-for="r in regionsTriees" :key="r.code" :value="r.code">{{ nomDeRegion(r.code) }}</option>
            </select>
            <span class="aide">{{ t("reglages.region.devise", { devise: deviseAttendue }) }}. {{ t("reglages.region.note") }}</span>
          </div>
        </template>

        <!-- Launchers -->
        <template v-else-if="etape === 2">
          <h2>{{ t("tuto.launchers.titre") }}</h2>
          <p class="intro">{{ t("tuto.launchers.texte") }}</p>
          <p class="trouves" role="status">
            <span v-if="loading" class="spin" aria-hidden="true" />
            <template v-if="jeux">{{ t("tuto.launchers.trouves", { n: jeux }) }}</template>
            <template v-else-if="loading">{{ t("tuto.launchers.recherche") }}</template>
            <template v-else>{{ t("tuto.launchers.aucun") }}</template>
          </p>
          <AccountsSettings />
        </template>

        <!-- Compte Torii -->
        <template v-else>
          <template v-if="connected">
            <h2>{{ t("tuto.compte.connecte", { nom: account?.displayName ?? "" }) }}</h2>
            <p class="intro">{{ t("tuto.compte.connecteTexte") }}</p>
          </template>
          <template v-else>
            <h2>{{ t("tuto.compte.titre") }}</h2>
            <p class="intro">{{ t("tuto.compte.texte") }}</p>
          </template>

          <ul class="avantages">
            <li v-for="a in AVANTAGES" :key="a.cle">
              <span class="icone" aria-hidden="true">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path :d="a.icone" /></svg>
              </span>
              <span class="avantage">
                <strong>{{ a.titre }}</strong>
                <span>{{ a.texte }}</span>
              </span>
            </li>
          </ul>

          <template v-if="!connected">
            <button class="btn-play creer" @click="openSignIn">{{ t("tuto.compte.creer") }}</button>
            <p class="aide rassurer">{{ t("tuto.compte.rassurer") }}</p>
          </template>
        </template>
      </section>

      <footer class="pied">
        <button class="discret" @click="terminer">{{ t("tuto.passer") }}</button>
        <span class="espace" />
        <button v-if="etape > 1" class="secondaire" @click="allerA(etape - 1)">{{ t("tuto.retour") }}</button>
        <button v-if="etape < TOTAL" class="btn-play" @click="allerA(etape + 1)">{{ t("tuto.suivant") }}</button>
        <button v-else-if="connected" class="btn-play" @click="versVisite">{{ t("tuto.compte.decouvrir") }}</button>
        <button v-else class="secondaire" @click="versVisite">{{ t("tuto.compte.plusTard") }}</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
/* Au-dessus des Paramètres (200), sous la connexion (260) qui s'ouvre depuis la dernière étape. */
.fond {
  position: fixed; inset: 0; z-index: 240; display: grid; place-items: center; padding: 24px;
  background: rgba(6, 4, 10, 0.72); backdrop-filter: blur(6px); outline: none;
}
.assistant {
  width: min(660px, 100%); height: min(680px, 100%); display: flex; flex-direction: column;
  background: var(--surface); border: 1px solid var(--border); border-radius: 20px; box-shadow: var(--shadow-hero); overflow: hidden;
}
.tete { display: flex; align-items: center; gap: 14px; padding: 20px 28px 0; }
.progression { display: flex; gap: 6px; flex: 1; }
.point { flex: 1; height: 4px; border-radius: 99px; background: var(--surface-3); transition: background 0.2s; }
.point.fait { background: var(--accent); }
.numero { flex: none; font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); }
.corps { flex: 1; min-height: 0; overflow-y: auto; padding: 22px 28px; }
.logo { display: block; width: 64px; height: 64px; margin: 6px 0 18px; }
h2 { margin: 0 0 8px; font-size: 26px; font-weight: 800; letter-spacing: -0.03em; }
/* Les cartes des Paramètres en plus court : l'essentiel de leur texte est dit une fois, en tête. */
.corps :deep(.group-label), .corps :deep(.account .hint), .corps :deep(.advanced-toggle) { display: none; }
.corps :deep(.account .row) { margin-top: 10px; }
.corps :deep(.group) { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.corps :deep(.account) { margin: 0; }
.intro { margin: 0 0 22px; font-size: 14.5px; line-height: 1.55; color: var(--text-dim); }

.champ { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; margin-bottom: 22px; }
.libelle { font-size: 13.5px; font-weight: 700; }
.segmented { flex-wrap: wrap; }
.region {
  width: min(340px, 100%); padding: 10px 12px; border-radius: 10px; font: inherit; font-size: 13.5px;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text);
}
.region:focus { border-color: var(--accent); }
.aide { font-size: 12.5px; line-height: 1.5; color: var(--text-faint); }

.trouves { display: flex; align-items: center; gap: 9px; margin: -8px 0 20px; font-size: 13.5px; font-weight: 600; color: var(--text); }
.spin { width: 15px; height: 15px; flex: none; border-radius: 50%; border: 2px solid var(--border-strong); border-top-color: var(--accent); animation: spin 0.7s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.avantages { list-style: none; margin: 0 0 22px; padding: 0; display: flex; flex-direction: column; gap: 14px; }
.avantages li { display: flex; gap: 14px; align-items: flex-start; }
.icone { flex: none; width: 38px; height: 38px; display: grid; place-items: center; border-radius: 11px; background: var(--accent-soft); color: var(--accent); }
.icone svg { width: 19px; height: 19px; }
.avantage { display: flex; flex-direction: column; gap: 2px; font-size: 13px; line-height: 1.45; color: var(--text-dim); }
.avantage strong { font-size: 14.5px; font-weight: 700; color: var(--text); }
.creer { width: 100%; justify-content: center; }
.rassurer { display: block; margin: 10px 0 0; text-align: center; }

.pied { display: flex; align-items: center; gap: 10px; padding: 14px 28px 18px; border-top: 1px solid var(--border); }
.espace { flex: 1; }
.discret { background: none; border: none; padding: 8px 0; font-size: 13px; color: var(--text-faint); }
.discret:hover { color: var(--text-dim); }
.secondaire {
  padding: 11px 18px; border-radius: 12px; font-size: 14px; font-weight: 600;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text);
}
.secondaire:hover { border-color: var(--border-strong); }

@media (max-width: 620px) {
  .fond { padding: 0; }
  .assistant { width: 100%; height: 100%; border: none; border-radius: 0; }
  .tete, .corps, .pied { padding-left: 18px; padding-right: 18px; }
  .corps :deep(.group) { grid-template-columns: minmax(0, 1fr); }
}
</style>
