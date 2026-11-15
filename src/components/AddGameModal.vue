<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useUi } from "../composables/useUi";
import { useLibrary } from "../composables/useLibrary";
import { useScrollLock } from "../composables/useScrollLock";
import { useFocusTrap } from "../composables/useFocusTrap";
import {
  displayableCover,
  inspectGameTarget,
  onFileDrop,
  pickFile,
  pickFolder,
  searchGameSheets,
  type ManualInput,
  type ManualSheet,
} from "../lib/tauri";
import { gradientFor } from "../lib/covers";
import type { GameSheet } from "../types";
import { t } from "../i18n";

const { addGameOpen, editGameId, closeAddGame, openGame } = useUi();
const { addManual, updateManual, byId } = useLibrary();

useScrollLock(addGameOpen);

/** Le clavier ne doit pas sortir de la modale, et doit revenir d’où il vient. */
const modale = ref<HTMLElement | null>(null);
useFocusTrap(modale, addGameOpen);

const title = ref("");
const launchTarget = ref("");
const launchArgs = ref("");
const installDir = ref("");
const coverUrl = ref("");
const saving = ref(false);
const error = ref<string | null>(null);

/** Édition d'un jeu existant plutôt que création. */
const editing = computed(() => editGameId.value != null);

/**
 * En création, on commence par désigner le fichier : c'est de lui que Torii tire le
 * titre, le dossier et les options de lancement. Le formulaire ne vient qu'ensuite.
 */
const etape = ref<"choix" | "fiche">("choix");
/** Lecture du fichier désigné en cours. */
const analyse = ref(false);
/** Un fichier est tenu au-dessus de la fenêtre. */
const survol = ref(false);
/** Options avancées dépliées. */
const avance = ref(false);
/**
 * Le titre vient de la lecture du fichier et personne ne l'a retouché : la fiche trouvée
 * peut le corriger (« Return To Moria » → son vrai nom). Un titre tapé, lui, est gardé.
 */
const titreDevine = ref(false);

const boutonChoisir = ref<HTMLButtonElement | null>(null);
const champTitre = ref<HTMLInputElement | null>(null);
const champCible = ref<HTMLInputElement | null>(null);

// --- Fiche du jeu (recherche IGDB) --------------------------------------------------

/** Résultats de la dernière recherche ; `null` tant qu'on n'a rien cherché. */
const fiches = ref<GameSheet[] | null>(null);
const ficheChoisie = ref<GameSheet | null>(null);
/** Une fiche a été choisie ou retirée ici : à l'enregistrement, elle remplace l'ancienne. */
const ficheTouchee = ref(false);
const recherche = ref(false);
const erreurRecherche = ref<string | null>(null);
/**
 * Jaquette posée par une fiche, pas par l'utilisateur : choisir une autre fiche peut la
 * remplacer, alors qu'une image choisie sur le disque ne bouge jamais.
 */
const jaquetteDeFiche = ref<string | null>(null);
/** Numéro de la recherche en cours : une réponse arrivée trop tard est ignorée. */
let tour = 0;
/** Adresse d'une jaquette qui n'a pas pu s'afficher : l'aperçu retombe sur le dégradé. */
const jaquetteRatee = ref<string | null>(null);

const canSave = computed(() => title.value.trim() !== "" && launchTarget.value.trim() !== "");
/**
 * ⚠️ Le bouton n'est PAS désactivé pendant la recherche (c'est `chercher` qui ignore un
 * second clic) : un bouton désactivé perd le focus, qui retombait alors sur la page.
 */
const peutChercher = computed(() => title.value.trim().length >= 2);

/** Les images d'IGDB sont des jaquettes de fiche, même sur un jeu enregistré il y a longtemps. */
const estUneJaquetteDeFiche = (url: string) => url.startsWith("https://images.igdb.com/");

// (Ré)initialise le formulaire à chaque ouverture : vide en création, pré-rempli en édition.
watch(addGameOpen, (open) => {
  if (!open) return;
  const g = editGameId.value ? byId(editGameId.value) : null;
  title.value = g?.title ?? "";
  launchTarget.value = g?.launchTarget ?? "";
  launchArgs.value = g?.launchArgs ?? "";
  installDir.value = g?.installDir ?? "";
  // On réaffiche le chemin/l'URL d'origine, pas l'URL `asset://` de rendu.
  coverUrl.value = g?.coverSource ?? "";
  error.value = null;
  saving.value = false;
  analyse.value = false;
  survol.value = false;
  avance.value = false;
  titreDevine.value = false;
  etape.value = g ? "fiche" : "choix";
  tour++;
  fiches.value = null;
  ficheChoisie.value = null;
  ficheTouchee.value = false;
  recherche.value = false;
  erreurRecherche.value = null;
  jaquetteDeFiche.value = estUneJaquetteDeFiche(coverUrl.value) ? coverUrl.value : null;
  jaquetteRatee.value = null;
  // Après le piège à focus, qui vise le premier bouton venu (la croix de fermeture).
  void nextTick(() => nextTick(() => (g ? champTitre.value : boutonChoisir.value)?.focus()));
});

/**
 * Lit le fichier désigné (choisi, déposé) et en remplit le formulaire. Un raccourci est
 * remplacé par sa cible et ses arguments ; un fichier introuvable est refusé ici, pas au
 * premier clic sur « Jouer ».
 */
async function prendre(chemin: string) {
  if (analyse.value || saving.value) return;
  analyse.value = true;
  error.value = null;
  try {
    const vu = await inspectGameTarget(chemin);
    if (!vu) return;
    launchTarget.value = vu.launchTarget;
    launchArgs.value = vu.launchArgs ?? "";
    installDir.value = vu.installDir ?? "";
    // Des options de lancement lues dans un raccourci : autant les montrer.
    if (vu.launchArgs) avance.value = true;
    const premierChoix = etape.value === "choix";
    // En édition, le titre est celui que l'utilisateur a déjà validé.
    if (!editing.value && vu.title && (premierChoix || titreDevine.value || !title.value.trim())) {
      title.value = vu.title;
      titreDevine.value = true;
    }
    etape.value = "fiche";
    await nextTick();
    champTitre.value?.focus();
    // Une seule recherche, à ce moment précis : le quota d'IGDB est partagé.
    if (premierChoix && title.value.trim()) void chercher();
  } catch (e) {
    error.value = String(e);
  } finally {
    analyse.value = false;
  }
}

/** Choix du fichier à lancer dans l'explorateur Windows. */
async function browseExe() {
  const path = await pickFile(t("comptes.ajout.choisirExe"), [
    { name: t("comptes.ajout.programmes"), extensions: ["exe", "bat", "cmd", "lnk", "url"] },
    { name: t("comptes.ajout.tousFichiers"), extensions: ["*"] },
  ]);
  if (path) await prendre(path);
}

/** Pas de fichier à désigner (une adresse `steam://…`, un chemin connu par cœur). */
async function saisirALaMain() {
  etape.value = "fiche";
  await nextTick();
  champCible.value?.focus();
}

/** Choix du dossier d'installation. */
async function browseDir() {
  const dir = await pickFolder(t("comptes.ajout.choisirDossier"));
  if (dir) installDir.value = dir;
}

/** Choix d'une image de jaquette sur le disque (une URL reste saisissable à la main). */
async function browseCover() {
  const path = await pickFile(t("comptes.ajout.choisirJaquette"), [
    { name: t("comptes.ajout.images"), extensions: ["jpg", "jpeg", "png", "webp", "avif", "gif", "bmp"] },
  ]);
  if (path) coverUrl.value = path;
}

/**
 * Cherche les fiches qui portent ce titre : une fois après la lecture du fichier, puis
 * au bouton.
 *
 * ⚠️ Jamais à la frappe : un appel par action, le quota d'IGDB est partagé par tous.
 */
async function chercher() {
  const titre = title.value.trim();
  if (titre.length < 2 || recherche.value) return;
  const moi = ++tour;
  recherche.value = true;
  erreurRecherche.value = null;
  try {
    const trouvees = await searchGameSheets(titre);
    if (moi !== tour) return;
    fiches.value = trouvees;
    // On ne présélectionne que ce qui correspond vraiment, et sans ambiguïté ; pour le
    // reste, à l'utilisateur de dire lequel c'est. Une recherche floue ramène toujours
    // quelque chose, et un même nom peut désigner deux jeux : « Ocarina of Time » est à la
    // fois celui de 1998 et son remake, et l'ordre d'IGDB ne dit pas lequel on possède.
    const seule = (liste: GameSheet[]) => (liste.length === 1 ? liste[0] : null);
    const exactes = trouvees.filter((f) => f.exact);
    const sure = exactes.length ? seule(exactes) : seule(trouvees.filter((f) => f.proche));
    if (sure) choisir(sure, "auto");
    // La fiche retenue jusque-là n'est plus à l'écran : la garder serait l'appliquer en douce.
    else if (ficheChoisie.value && !trouvees.some((f) => memeFiche(f, ficheChoisie.value))) retirerFiche();
  } catch (e) {
    if (moi !== tour) return;
    fiches.value = null;
    erreurRecherche.value = String(e);
  } finally {
    if (moi === tour) recherche.value = false;
  }
}

function memeFiche(a: GameSheet, b: GameSheet | null): boolean {
  if (!b) return false;
  return a.igdbId != null ? a.igdbId === b.igdbId : a.name === b.name;
}

/**
 * Retient une fiche : sa jaquette (sauf image choisie à la main) et, à l'enregistrement,
 * sa description, son studio, ses captures. Un clic prend aussi son nom ; une
 * présélection ne remplace qu'un titre deviné, jamais un titre tapé.
 */
function choisir(fiche: GameSheet, origine: "clic" | "auto") {
  ficheChoisie.value = fiche;
  ficheTouchee.value = true;
  if (origine === "clic" || fiche.exact || titreDevine.value) {
    title.value = fiche.name;
    titreDevine.value = false;
  }
  if (!coverUrl.value.trim() || coverUrl.value === jaquetteDeFiche.value) {
    coverUrl.value = fiche.coverUrl ?? "";
    jaquetteDeFiche.value = fiche.coverUrl ?? null;
  }
}

/** « Aucune de ces fiches » : le jeu n'en aura pas (et perd celle qu'il avait). */
function retirerFiche() {
  ficheChoisie.value = null;
  ficheTouchee.value = true;
  if (coverUrl.value === jaquetteDeFiche.value) coverUrl.value = "";
  jaquetteDeFiche.value = null;
}

function versFiche(f: GameSheet): ManualSheet {
  return {
    heroUrl: f.heroUrl ?? null,
    genre: f.genre ?? null,
    description: f.description ?? null,
    developer: f.developer ?? null,
    year: f.year ?? null,
    screenshots: f.screenshots ?? [],
  };
}

// --- Aperçu de la jaquette ----------------------------------------------------------

const apercuSrc = computed(() => {
  const brut = coverUrl.value.trim();
  if (!brut) return null;
  const url = displayableCover(brut);
  return url === jaquetteRatee.value ? null : url;
});

/** Le dégradé que portera la carte : il se calcule sur l'id du jeu, tiré du titre. */
const degrade = computed(() =>
  gradientFor(editGameId.value ?? `manual:${title.value.trim().toLowerCase().replace(/[^\p{L}\p{N}]/gu, "-")}`),
);

async function save() {
  if (!canSave.value || saving.value) return;
  saving.value = true;
  error.value = null;
  const input: ManualInput = {
    title: title.value.trim(),
    launchTarget: launchTarget.value.trim(),
    launchArgs: launchArgs.value.trim() || null,
    installDir: installDir.value.trim() || null,
    coverUrl: coverUrl.value.trim() || null,
  };
  if (ficheTouchee.value) input.fiche = ficheChoisie.value ? versFiche(ficheChoisie.value) : {};
  try {
    const id = editGameId.value;
    const game = id ? await updateManual(id, input) : await addManual(input);
    closeAddGame();
    if (game) openGame(game.id);
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function onKey(e: KeyboardEvent) {
  // `preventDefault` : la touche a servi, `useRaccourcis` ne referme pas en plus la fiche dessous.
  if (e.key === "Escape") {
    e.preventDefault();
    closeAddGame();
  }
}

// --- Glisser-déposer depuis l'Explorateur -------------------------------------------

let nePlusEcouter: (() => void) | null = null;

onMounted(async () => {
  nePlusEcouter = await onFileDrop(
    (chemins) => {
      // La modale est montée en permanence : un dépôt ne la concerne que si elle est ouverte.
      if (!addGameOpen.value) return;
      if (chemins.length !== 1) {
        error.value = t("comptes.ajout.unSeulFichier");
        return;
      }
      void prendre(chemins[0]);
    },
    (dessus) => (survol.value = dessus && addGameOpen.value),
  );
});
onBeforeUnmount(() => nePlusEcouter?.());
</script>

<template>
  <div v-if="addGameOpen" class="modal-backdrop" @click.self="closeAddGame" @keydown="onKey">
    <div
      ref="modale"
      class="modal"
      :class="{ large: etape === 'fiche', survol: survol && etape === 'fiche' }"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      :aria-label="editing ? t('comptes.ajout.modifier') : t('comptes.ajout.ajouter')"
    >
      <div class="modal-head">
        <h3>{{ editing ? t("comptes.ajout.modifier") : t("comptes.ajout.ajouter") }}</h3>
        <button class="modal-close" :aria-label="t('commun.fermer')" @click="closeAddGame">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
      </div>
      <p class="modal-sub">
        {{ editing ? t("comptes.ajout.introModif") : t("comptes.ajout.introAjout") }}
      </p>

      <!-- Étape 1 (création) : désigner le fichier. -->
      <div v-if="etape === 'choix'" class="depot" :class="{ survol }">
        <svg class="depot-icone" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
          <path d="M12 15V4m0 0L8 8m4-4l4 4" /><path d="M4 14v4a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4" />
        </svg>
        <p class="depot-titre">{{ survol ? t("comptes.ajout.depotSurvol") : t("comptes.ajout.depot") }}</p>
        <p class="depot-aide">{{ t("comptes.ajout.depotAide") }}</p>
        <button ref="boutonChoisir" type="button" class="btn-save" :disabled="analyse" @click="browseExe">
          {{ analyse ? t("comptes.ajout.analyse") : t("comptes.ajout.choisirFichier") }}
        </button>
        <button type="button" class="lien" @click="saisirALaMain">{{ t("comptes.ajout.saisirALaMain") }}</button>
        <p v-if="error" class="modal-error" role="alert">{{ error }}</p>
      </div>

      <!-- Étape 2 : le jeu, sa fiche, son fichier. -->
      <form v-else class="modal-form" @submit.prevent="save">
        <div class="corps">
          <div class="apercu" :style="{ background: degrade }" role="img" :aria-label="t('comptes.ajout.apercu')">
            <img v-if="apercuSrc" :key="apercuSrc" :src="apercuSrc" alt="" @error="jaquetteRatee = apercuSrc" />
            <template v-else>
              <span class="apercu-voile" />
              <span class="apercu-titre">{{ title }}</span>
            </template>
          </div>

          <div class="champs">
            <label class="field">
              <span class="field-label">{{ t("comptes.ajout.titre") }} <em>*</em></span>
              <div class="field-row">
                <input
                  ref="champTitre"
                  v-model="title"
                  type="text"
                  :placeholder="t('comptes.ajout.titreExemple')"
                  @input="titreDevine = false"
                />
                <button
                  type="button"
                  class="btn-browse"
                  :disabled="!peutChercher"
                  :aria-busy="recherche"
                  :title="t('comptes.ajout.rechercherAide')"
                  @click="chercher()"
                >
                  {{ recherche ? t("comptes.ajout.recherche") : t("comptes.ajout.rechercher") }}
                </button>
              </div>
            </label>

            <p v-if="erreurRecherche" class="modal-error" role="alert">{{ erreurRecherche }}</p>
            <p v-else-if="fiches && !fiches.length" class="field-hint" role="status">{{ t("comptes.ajout.aucunResultat") }}</p>
            <div v-else-if="fiches" class="field">
              <span id="ajout-fiches" class="field-label">{{ t("comptes.ajout.fiches") }}</span>
              <div class="fiches" role="group" aria-labelledby="ajout-fiches">
                <button
                  v-for="f in fiches"
                  :key="f.igdbId ?? f.name"
                  type="button"
                  class="fiche"
                  :class="{ on: memeFiche(f, ficheChoisie) }"
                  :aria-pressed="memeFiche(f, ficheChoisie)"
                  @click="choisir(f, 'clic')"
                >
                  <img v-if="f.coverUrl" :src="f.coverUrl" alt="" loading="lazy" decoding="async" />
                  <span v-else class="fiche-vide" />
                  <span class="fiche-texte">
                    <b>{{ f.name }}</b>
                    <small>{{ [f.year, f.developer].filter(Boolean).join(" · ") }}</small>
                  </span>
                </button>
                <button
                  type="button"
                  class="fiche sans"
                  :class="{ on: ficheTouchee && !ficheChoisie }"
                  :aria-pressed="ficheTouchee && !ficheChoisie"
                  @click="retirerFiche"
                >
                  {{ t("comptes.ajout.aucuneDeCesFiches") }}
                </button>
              </div>
              <span class="field-hint">{{ t("comptes.ajout.fichesAide") }}</span>
            </div>

            <label class="field">
              <span class="field-label">{{ t("comptes.ajout.executable") }} <em>*</em></span>
              <div class="field-row">
                <input ref="champCible" v-model="launchTarget" type="text" :placeholder="t('comptes.ajout.executableExemple')" spellcheck="false" />
                <button type="button" class="btn-browse" :disabled="analyse" @click="browseExe">{{ t("comptes.ajout.parcourir") }}</button>
              </div>
              <span class="field-hint">{{ t("comptes.ajout.executableAide") }}</span>
            </label>

            <button type="button" class="repli" :aria-expanded="avance" aria-controls="ajout-avance" @click="avance = !avance">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
              {{ t("comptes.ajout.avance") }}
            </button>
            <div v-show="avance" id="ajout-avance" class="avance">
              <label class="field">
                <span class="field-label">{{ t("comptes.ajout.arguments") }} <i>{{ t("comptes.ajout.optionnel") }}</i></span>
                <input v-model="launchArgs" type="text" :placeholder="t('comptes.ajout.argumentsExemple')" spellcheck="false" />
                <span class="field-hint">{{ t("comptes.ajout.argumentsAide") }}</span>
              </label>
              <label class="field">
                <span class="field-label">{{ t("comptes.ajout.dossier") }} <i>{{ t("comptes.ajout.optionnel") }}</i></span>
                <div class="field-row">
                  <input v-model="installDir" type="text" :placeholder="t('comptes.ajout.dossierExemple')" spellcheck="false" />
                  <button type="button" class="btn-browse" @click="browseDir">{{ t("comptes.ajout.parcourir") }}</button>
                </div>
                <span class="field-hint">{{ t("comptes.ajout.dossierAide") }}</span>
              </label>
              <label class="field">
                <span class="field-label">{{ t("comptes.ajout.jaquette") }} <i>{{ t("comptes.ajout.optionnel") }}</i></span>
                <div class="field-row">
                  <input v-model="coverUrl" type="text" :placeholder="t('comptes.ajout.jaquetteExemple')" spellcheck="false" />
                  <button type="button" class="btn-browse" @click="browseCover">{{ t("comptes.ajout.parcourir") }}</button>
                </div>
                <span class="field-hint">{{ t("comptes.ajout.jaquetteAide") }}</span>
              </label>
            </div>
          </div>
        </div>

        <p v-if="error" class="modal-error" role="alert">{{ error }}</p>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="closeAddGame">{{ t("commun.annuler") }}</button>
          <!-- Pas désactivé pendant l'enregistrement (`save` ignore un second envoi) : un bouton
               désactivé perd le focus, et le message d'erreur arrivait sur une page sans focus. -->
          <button type="submit" class="btn-save" :disabled="!canSave" :aria-busy="saving">
            {{ saving ? t("comptes.ajout.enregistrement") : editing ? t("comptes.ajout.enregistrer") : t("comptes.ajout.ajouterJeu") }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.modal-backdrop {
  position: fixed; inset: 0; z-index: 260; display: grid; place-items: center; padding: 5vh 20px;
  background: rgba(6, 4, 10, 0.62); backdrop-filter: blur(5px);
}
.modal {
  width: 100%; max-width: 460px; max-height: 90vh; overflow-y: auto;
  background: var(--surface); border: 1px solid var(--border);
  border-radius: 18px; box-shadow: var(--shadow-hero); padding: 24px;
}
.modal.large { max-width: 680px; }
/* Un fichier tenu au-dessus du formulaire : il remplacera celui du jeu. */
.modal.survol { border-color: var(--accent); }
.modal-head { display: flex; align-items: center; justify-content: space-between; }
.modal-head h3 { font-size: 19px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.modal-close {
  width: 34px; height: 34px; border-radius: 10px; display: grid; place-items: center;
  background: none; border: 1px solid var(--border); color: var(--text-dim); cursor: pointer;
}
.modal-close:hover { color: var(--text); border-color: var(--border-strong); }
.modal-close svg { width: 17px; height: 17px; }
.modal-sub { font-size: 13px; color: var(--text-faint); margin: 6px 0 18px; line-height: 1.5; }

/* Étape 1 : la zone de dépôt. */
.depot {
  display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center;
  padding: 30px 24px 22px; border-radius: 16px; border: 1.5px dashed var(--border-strong);
  background: var(--bg); transition: border-color 0.15s, background 0.15s;
}
.depot.survol { border-color: var(--accent); background: var(--accent-soft); }
.depot-icone { width: 34px; height: 34px; color: var(--text-dim); }
.depot.survol .depot-icone { color: var(--accent); }
.depot-titre { margin: 0; font-size: 15px; font-weight: 650; letter-spacing: -0.01em; text-wrap: balance; }
.depot-aide { margin: 0 0 8px; font-size: 12.5px; color: var(--text-faint); line-height: 1.5; max-width: 340px; text-wrap: pretty; }
.lien {
  background: none; border: none; padding: 4px 6px; cursor: pointer; font-family: inherit;
  font-size: 12.5px; color: var(--text-dim); text-decoration: underline; text-underline-offset: 3px;
}
.lien:hover { color: var(--text); }
.depot .modal-error { align-self: stretch; text-align: left; }

/* Étape 2 : aperçu à gauche, champs à droite. */
.modal-form { display: flex; flex-direction: column; gap: 16px; }
.corps { display: grid; grid-template-columns: 170px minmax(0, 1fr); gap: 20px; align-items: start; }
.apercu {
  position: relative; aspect-ratio: 3 / 4; border-radius: 14px; overflow: hidden;
  border: 1px solid var(--border); box-shadow: var(--shadow-card);
}
.apercu img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block; }
.apercu-voile { position: absolute; inset: 0; background: linear-gradient(0deg, rgba(0, 0, 0, 0.55) 0%, transparent 46%); }
.apercu-titre {
  position: absolute; left: 12px; right: 12px; bottom: 12px; color: #fff;
  font-weight: 800; font-size: 16px; line-height: 1.08; letter-spacing: -0.02em;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.5); text-wrap: balance; overflow-wrap: anywhere;
}
.champs { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
.field { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
.field-label { font-size: 12.5px; font-weight: 600; color: var(--text-dim); }
.field-label em { color: var(--accent); font-style: normal; }
.field-label i { color: var(--text-faint); font-style: normal; font-weight: 400; }
.field input {
  width: 100%; padding: 10px 13px; background: var(--bg); border: 1px solid var(--border);
  border-radius: 10px; color: var(--text); font-size: 13.5px; font-family: inherit;
}
.field-row { display: flex; gap: 8px; align-items: stretch; }
.field-row input { flex: 1; min-width: 0; }
.btn-browse {
  flex: none; padding: 0 14px; border-radius: 10px; font-size: 12.5px; font-weight: 600; cursor: pointer;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim); white-space: nowrap;
}
.btn-browse:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); background: var(--surface-3); }
.btn-browse:disabled { opacity: 0.5; cursor: default; }
.field input:focus { border-color: var(--border-strong); background: var(--surface-2); }
.field input::placeholder { color: var(--text-faint); }
.field-hint { font-size: 11.5px; color: var(--text-faint); line-height: 1.4; margin: 0; }

/* Les fiches trouvées : une liste de boutons, celle retenue est marquée. */
.fiches {
  display: flex; flex-direction: column; gap: 3px; max-height: 236px; overflow-y: auto;
  padding: 4px; border: 1px solid var(--border); border-radius: 12px; background: var(--bg);
}
.fiche {
  display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 9px; cursor: pointer;
  background: none; border: 1px solid transparent; color: var(--text); text-align: left; font-family: inherit;
}
.fiche:hover { background: var(--surface-2); }
.fiche.on { background: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
.fiche img, .fiche-vide {
  flex: none; width: 33px; height: 44px; border-radius: 6px; object-fit: cover; background: var(--surface-3);
}
.fiche-texte { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.fiche-texte b { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.fiche-texte small { font-size: 11.5px; color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.fiche.sans { padding: 9px 10px; font-size: 12.5px; color: var(--text-dim); }

/* Options avancées. */
.repli {
  align-self: flex-start; display: inline-flex; align-items: center; gap: 6px; padding: 4px 6px 4px 2px;
  background: none; border: none; cursor: pointer; font-family: inherit;
  font-size: 12.5px; font-weight: 600; color: var(--text-dim);
}
.repli:hover { color: var(--text); }
.repli svg { width: 14px; height: 14px; transition: transform 0.15s; }
.repli[aria-expanded="true"] svg { transform: rotate(90deg); }
.avance { display: flex; flex-direction: column; gap: 14px; }

.modal-error {
  font-size: 12.5px; color: #ff6b6b; margin: 0;
  padding: 9px 12px; border-radius: 9px; background: color-mix(in srgb, #ff6b6b 12%, transparent);
}
.modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 2px; }
.btn-cancel, .btn-save {
  padding: 10px 18px; border-radius: 11px; font-size: 13.5px; font-weight: 600; cursor: pointer; font-family: inherit;
}
.btn-cancel { background: none; border: 1px solid var(--border); color: var(--text-dim); }
.btn-cancel:hover { color: var(--text); border-color: var(--border-strong); }
.btn-save { background: var(--accent); border: 1px solid transparent; color: var(--accent-ink); }
.btn-save:hover:not(:disabled) { background: var(--accent-hover); }
.btn-save:disabled { opacity: 0.5; cursor: default; }

@media (max-width: 620px) {
  .corps { grid-template-columns: 1fr; }
  .apercu { width: 130px; }
}
@media (prefers-reduced-motion: reduce) {
  .depot, .repli svg { transition: none; }
}
</style>
