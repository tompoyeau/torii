<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useUi } from "../composables/useUi";
import { useStore } from "../composables/useStore";
import { useTorii } from "../composables/useTorii";
import { useUpdater } from "../composables/useUpdater";
import { useNavigation } from "../composables/useNavigation";
import { usePageWeb } from "../composables/usePageWeb";
import { useScrollLock } from "../composables/useScrollLock";
import { avecRaccourci } from "../composables/useRaccourcis";
import { openExternal, pageAction, pageBornes, pageFermer, pageOuvrir, pageVisible, type BornesPage } from "../lib/tauri";
import { copierLien, lienPartageable } from "../lib/partage";
import { t } from "../i18n";

/**
 * Écran « page web » : un site ouvert dans Torii (boutique Steam, page d'achat d'un
 * revendeur…). Ce composant ne dessine que la barre et la place de la page : la page
 * elle-même est une WebView native (`page_web.rs`) posée par-dessus cette place.
 *
 * ⚠️ LA PAGE PASSE AU-DESSUS DE TOUT CE QUE DESSINE L'INTERFACE. Elle est cachée dès
 * qu'une surcouche Torii s'ouvre (fiche, Paramètres, modale), et rien de Torii ne doit
 * dépasser sur sa place : les notifications montent en haut de la fenêtre pendant qu'elle
 * est affichée, et le bandeau de mise à jour lui rogne le bas.
 */

const { pageWeb, settingsOpen, selectedGameId, addGameOpen } = useUi();
const { selectedGameId: produitOuvert } = useStore();
const { signInOpen } = useTorii();
const { status: statutMaj } = useUpdater();
const { etat } = usePageWeb();
const { reculer, avancer, peutReculer, peutAvancer, libellePrecedent, libelleSuivant, fermer } = useNavigation();

// La page ne défile pas avec la fenêtre : si la fenêtre défilait, la place réservée
// bougerait et la WebView native, elle, resterait où elle est.
useScrollLock(computed(() => true));

const racine = ref<HTMLElement | null>(null);
const zone = ref<HTMLElement | null>(null);
const erreur = ref("");

/** Hauteur de l'écran sous la barre du haut, mesurée : elle ne dépend que d'elle. */
const haut = ref(0);

function bornes(): BornesPage | null {
  const el = zone.value;
  if (!el) return null;
  const r = el.getBoundingClientRect();
  let bas = r.bottom;
  const bandeau = document.querySelector<HTMLElement>(".update-banner");
  if (bandeau) bas = Math.min(bas, bandeau.getBoundingClientRect().top - 12);
  return { x: r.left, y: r.top, largeur: r.width, hauteur: Math.max(1, bas - r.top) };
}

/** Une surcouche de Torii est ouverte : la page doit s'effacer, sinon elle la cacherait. */
const recouverte = computed(
  () => settingsOpen.value || selectedGameId.value != null || produitOuvert.value != null || addGameOpen.value || signInOpen.value,
);

async function ouvrir(url: string) {
  const b = bornes();
  if (!b) return;
  erreur.value = "";
  try {
    await pageOuvrir(url, b);
    if (recouverte.value) void pageVisible(false);
  } catch (err) {
    erreur.value = t("page.echec", { erreur: String(err) });
  }
}

// --- Recalage ------------------------------------------------------------------
// Une image au plus : un redimensionnement envoie des dizaines d'événements.
let prevu = 0;
function recaler() {
  if (prevu) return;
  prevu = requestAnimationFrame(() => {
    prevu = 0;
    if (racine.value) haut.value = racine.value.getBoundingClientRect().top;
    const b = bornes();
    if (b) void pageBornes(b);
  });
}
let observateur: ResizeObserver | null = null;

onMounted(async () => {
  if (racine.value) haut.value = racine.value.getBoundingClientRect().top;
  await nextTick();
  if (pageWeb.value) void ouvrir(pageWeb.value);
  observateur = new ResizeObserver(recaler);
  if (zone.value) observateur.observe(zone.value);
  window.addEventListener("resize", recaler);
});
onBeforeUnmount(() => {
  observateur?.disconnect();
  window.removeEventListener("resize", recaler);
  if (prevu) cancelAnimationFrame(prevu);
  void pageFermer();
});

// Retour à une étape où la page était ailleurs : on l'y renvoie. Quand c'est la page
// elle-même qui a changé d'adresse (un lien suivi), il n'y a rien à faire.
watch(pageWeb, (url) => {
  if (url && url !== etat.url) void ouvrir(url);
});
watch(recouverte, (r) => void pageVisible(!r));
watch(statutMaj, () => void nextTick(recaler));

// --- Barre ---------------------------------------------------------------------

const adresse = computed(() => etat.url || pageWeb.value || "");
/** L'adresse qu'on peut envoyer à quelqu'un : sans code de connexion (cf. `lienPartageable`). */
const lienACopier = computed(() => lienPartageable(adresse.value));
const domaine = computed(() => {
  try {
    return new URL(adresse.value).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
});
const chiffree = computed(() => adresse.value.startsWith("https:"));

const alt = computed(() => t("reglages.raccourcis.touches.alt"));
const titreRecul = computed(() =>
  avecRaccourci(
    etat.peutReculer
      ? t("page.precedente")
      : libellePrecedent.value
        ? t("bibliotheque.barre.precedentVers", { lieu: libellePrecedent.value })
        : t("bibliotheque.barre.precedent"),
    alt.value, "←",
  ),
);
const titreAvance = computed(() =>
  avecRaccourci(
    etat.peutAvancer
      ? t("page.suivante")
      : libelleSuivant.value
        ? t("bibliotheque.barre.suivantVers", { lieu: libelleSuivant.value })
        : t("bibliotheque.barre.suivant"),
    alt.value, "→",
  ),
);
const titreFermer = computed(() =>
  libellePrecedent.value ? t("page.fermerVers", { lieu: libellePrecedent.value }) : t("page.fermer"),
);
</script>

<template>
  <div ref="racine" class="page-web" :style="{ height: `calc(100vh - ${haut}px)` }">
    <div class="barre">
      <button class="icon-btn" :disabled="!etat.peutReculer && !peutReculer" :title="titreRecul" :aria-label="titreRecul" @click="reculer()">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
      <button class="icon-btn" :disabled="!etat.peutAvancer && !peutAvancer" :title="titreAvance" :aria-label="titreAvance" @click="avancer()">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
      </button>
      <button class="icon-btn" :class="{ tourne: etat.chargement }" :title="t('page.recharger')" :aria-label="t('page.recharger')" @click="pageAction('recharger')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
      </button>

      <!-- L'adresse est dessinée par Torii, pas par le site : c'est elle qui dit où l'on paie. -->
      <div class="adresse" :title="adresse">
        <svg v-if="chiffree" class="cadenas" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" :aria-label="t('page.securise', { domaine })">
          <rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" />
        </svg>
        <span class="domaine">{{ domaine }}</span>
        <span v-if="etat.titre" class="titre">{{ etat.titre }}</span>
      </div>

      <button
        class="icon-btn"
        :disabled="!lienACopier"
        :title="t('commun.partage.copierLien')"
        :aria-label="t('commun.partage.copierLien')"
        @click="lienACopier && copierLien(lienACopier)"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 14a4.5 4.5 0 0 0 6.4 0l3.2-3.2a4.5 4.5 0 0 0-6.4-6.4L11.6 6" /><path d="M14 10a4.5 4.5 0 0 0-6.4 0l-3.2 3.2a4.5 4.5 0 0 0 6.4 6.4l1.6-1.6" /></svg>
      </button>
      <button class="texte" :disabled="!adresse" @click="openExternal(adresse)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></svg>
        <span>{{ t("page.navigateur") }}</span>
      </button>
      <button class="icon-btn" :title="titreFermer" :aria-label="titreFermer" @click="fermer('page')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </div>

    <!-- La place de la page : la WebView native s'y pose. Ce qui est dessiné ici ne se
         voit qu'avant son arrivée, ou si elle n'a pas pu s'ouvrir. -->
    <div ref="zone" class="zone">
      <div v-if="etat.chargement" class="progression" />
      <p v-if="erreur" class="message erreur">{{ erreur }}</p>
      <p v-else class="message">{{ t("page.chargement") }}</p>
    </div>
  </div>
</template>

<style scoped>
.page-web { display: flex; flex-direction: column; gap: 12px; padding-bottom: 16px; box-sizing: border-box; }
.barre { display: flex; align-items: center; gap: 8px; flex: none; }
.icon-btn {
  width: 38px; height: 38px; border-radius: 11px; border: 1px solid var(--border); flex: none;
  background: var(--surface); color: var(--text-dim); display: grid; place-items: center;
  transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); }
.icon-btn:disabled { opacity: 0.4; cursor: default; }
.icon-btn svg { width: 17px; height: 17px; }
.icon-btn.tourne svg { animation: tourne 0.9s linear infinite; }
@keyframes tourne { to { transform: rotate(360deg); } }

.adresse {
  flex: 1; min-width: 0; height: 38px; box-sizing: border-box; padding: 0 14px;
  display: flex; align-items: center; gap: 8px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 11px;
  font-size: 13px; white-space: nowrap;
}
.cadenas { width: 14px; height: 14px; flex: none; color: #4bbe6b; }
.domaine { font-weight: 700; color: var(--text); flex: none; }
.titre { min-width: 0; overflow: hidden; text-overflow: ellipsis; color: var(--text-faint); }

.texte {
  display: inline-flex; align-items: center; gap: 7px; height: 38px; padding: 0 14px 0 11px; flex: none;
  border-radius: 11px; border: 1px solid var(--border); background: var(--surface); color: var(--text-dim);
  font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.texte:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); }
.texte svg { width: 15px; height: 15px; }

.zone {
  position: relative; flex: 1; min-height: 0; overflow: hidden;
  border-radius: 12px; border: 1px solid var(--border); background: var(--surface);
  display: grid; place-items: center;
}
.message { margin: 0; font-size: 13.5px; color: var(--text-faint); }
.message.erreur { color: var(--text-dim); max-width: 520px; text-align: center; line-height: 1.5; }
/* Barre de chargement indéterminée, visible en haut de la place pendant que la page charge
   (la WebView native la recouvre une fois arrivée, d'où aussi la roue du bouton recharger). */
.progression {
  position: absolute; top: 0; left: 0; height: 2px; width: 30%;
  background: var(--accent); animation: progression 1.1s ease-in-out infinite;
}
@keyframes progression { from { left: -30%; } to { left: 100%; } }
</style>
