<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useNavigation } from "../composables/useNavigation";
import { useTorii } from "../composables/useTorii";
import { useUi } from "../composables/useUi";
import { hasTauriRuntime, jeuEnCours, onGameLaunched, type JeuEnCours } from "../lib/tauri";
import { MODE_WEB } from "../lib/mode";
import { t } from "../i18n";

/**
 * Ton état du moment, à gauche de la bande du haut (là où Windows écrit le titre d'une
 * fenêtre) : « En jeu · Hades II · 1 h 12 » pendant une partie (un clic ouvre sa fiche),
 * sinon ce que tes amis voient de toi (en ligne, invisible), et la synchronisation quand
 * elle tourne.
 *
 * La partie en cours vient de `jeu_en_cours` (Rust) : exactement celle que voient les amis
 * (`procwatch::partie_a_montrer`, sans jeux muets ni logiciels comme Wallpaper Engine). Relue à chaque lancement détecté et toutes les 20 s : la
 * fin d'une partie lancée ailleurs n'est signalée par aucun événement. Bande de l'appli de
 * bureau seulement : rien sur le web.
 */
const actif = hasTauriRuntime() && !MODE_WEB;

const { byId, loading } = useLibrary();
const { connected, presenceMode, setPresenceMode } = useTorii();
const { openGame } = useUi();
const { quitterLesSurcouches } = useNavigation();

const partie = ref<JeuEnCours | null>(null);
const maintenant = ref(Date.now());

async function relire() {
  partie.value = await jeuEnCours();
}

let sondage: ReturnType<typeof setInterval> | undefined;
let horloge: ReturnType<typeof setInterval> | undefined;
let arreter: (() => void) | null = null;
onMounted(async () => {
  if (!actif) return;
  void relire();
  sondage = setInterval(() => void relire(), 20_000);
  horloge = setInterval(() => (maintenant.value = Date.now()), 30_000);
  arreter = await onGameLaunched(() => void relire());
});
onBeforeUnmount(() => {
  clearInterval(sondage);
  clearInterval(horloge);
  arreter?.();
});

/** Le titre du surveillant, sinon celui de la bibliothèque (jeu reconnu entre-temps). */
const titre = computed(() => {
  const p = partie.value;
  if (!p) return "";
  return p.title || byId(p.id)?.title || t("amis.unJeu");
});

/** « 8 min », « 1 h 12 » : le temps de la partie, à la minute près. */
const duree = computed(() => {
  const p = partie.value;
  if (!p || !p.since) return "";
  const minutes = Math.max(0, Math.floor((maintenant.value / 1000 - p.since) / 60));
  if (minutes < 60) return t("systeme.etat.minutes", { m: minutes });
  return t("systeme.etat.heures", { h: Math.floor(minutes / 60), m: String(minutes % 60).padStart(2, "0") });
});

/** Ce que tes amis Torii voient de toi. Rien sans compte Torii. */
const presence = computed(() => {
  if (!connected.value) return null;
  return presenceMode.value === "offline"
    ? { classe: "invisible", libelle: t("systeme.etat.invisible") }
    : { classe: "en-ligne", libelle: t("systeme.etat.enLigne") };
});

/**
 * Changer ce que voient les amis, d'un clic sur son état : les trois mêmes choix que dans
 * Contacts et les Paramètres (mêmes libellés), dans un petit menu, sans explication : les
 * Paramètres les gardent pour qui veut le détail.
 */
const MODES = computed(() => [
  { cle: "detailed" as const, libelle: t("reglages.torii.presence.detaille") },
  { cle: "online" as const, libelle: t("reglages.torii.presence.enLigne") },
  { cle: "offline" as const, libelle: t("reglages.torii.presence.invisible") },
]);
const menuOuvert = ref(false);
const racine = ref<HTMLElement | null>(null);
const bouton = ref<HTMLElement | null>(null);

async function choisir(cle: (typeof MODES.value)[number]["cle"]) {
  menuOuvert.value = false;
  bouton.value?.focus();
  if (cle !== presenceMode.value) await setPresenceMode(cle);
}

/** Clic ailleurs ou Échap : le menu se referme (Échap rend le focus à l'état). */
function surClic(e: MouseEvent) {
  if (menuOuvert.value && !racine.value?.contains(e.target as Node)) menuOuvert.value = false;
}
function surTouche(e: KeyboardEvent) {
  if (e.key === "Escape" && menuOuvert.value) {
    e.preventDefault();
    menuOuvert.value = false;
    bouton.value?.focus();
  }
}
onMounted(() => {
  document.addEventListener("click", surClic);
  document.addEventListener("keydown", surTouche);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", surClic);
  document.removeEventListener("keydown", surTouche);
});

function ouvrirPartie() {
  const p = partie.value;
  if (!p || !byId(p.id)) return;
  quitterLesSurcouches();
  openGame(p.id);
}
</script>

<template>
  <div v-if="actif && (partie || presence || loading)" ref="racine" class="etat" data-tauri-drag-region>
    <!-- Ta présence : un clic pour la changer. Pendant une partie, le point seul suffit (la
         partie prend la place), le nom revient dans l'infobulle. -->
    <div v-if="presence" class="presence-wrap">
      <button
        ref="bouton"
        class="presence"
        :class="{ compacte: partie, ouvert: menuOuvert }"
        aria-haspopup="menu"
        :aria-expanded="menuOuvert"
        :aria-label="t('systeme.etat.changer', { etat: presence.libelle })"
        :title="t('systeme.etat.changer', { etat: presence.libelle })"
        @click.stop="menuOuvert = !menuOuvert"
      >
        <span class="pastille" :class="presence.classe" aria-hidden="true" />
        <span v-if="!partie">{{ presence.libelle }}</span>
        <svg class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
      </button>
      <div v-if="menuOuvert" class="menu" role="menu" :aria-label="t('amis.ceQueVoient')">
        <p class="menu-tete">{{ t("amis.ceQueVoient") }}</p>
        <button
          v-for="m in MODES"
          :key="m.cle"
          class="menu-choix"
          :class="{ on: m.cle === presenceMode }"
          role="menuitemradio"
          :aria-checked="m.cle === presenceMode"
          @click="choisir(m.cle)"
        >
          <span class="pastille" :class="m.cle === 'offline' ? 'invisible' : 'en-ligne'" aria-hidden="true" />
          <span class="menu-nom">{{ m.libelle }}</span>
          <svg v-if="m.cle === presenceMode" class="coche" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" aria-hidden="true"><path d="M5 13l4 4L19 7" /></svg>
        </button>
      </div>
    </div>

    <button
      v-if="partie"
      class="partie"
      :title="t('systeme.etat.ouvrirFiche', { titre })"
      @click="ouvrirPartie"
    >
      <span class="pastille en-jeu" aria-hidden="true" />
      <span class="quoi">{{ t("systeme.etat.enJeu") }}</span>
      <span class="titre">{{ titre }}</span>
      <span v-if="duree" class="duree">{{ duree }}</span>
    </button>

    <span v-if="loading" class="synchro" data-tauri-drag-region>
      <span class="rond" aria-hidden="true" />{{ t("systeme.etat.synchro") }}
    </span>
  </div>
</template>

<style scoped>
/* Dans la bande du haut (`--hauteur-titre`, style.css), à gauche, aligné sur la marge du
   contenu, et jamais sous les commandes de la fenêtre (3 × 46 px au coin droit). */
.etat {
  position: fixed; top: 0; left: 34px; z-index: 450;
  height: var(--hauteur-titre, 32px); max-width: calc(100vw - 34px - 3 * 46px - 24px);
  display: flex; align-items: center; gap: 14px;
  font-size: 12px; color: var(--text-faint); user-select: none;
}
.synchro { display: inline-flex; align-items: center; gap: 7px; white-space: nowrap; }

/* La présence reste du texte discret ; le survol seul dit qu'elle se clique. */
.presence-wrap { position: relative; display: flex; }
.presence {
  display: inline-flex; align-items: center; gap: 7px; height: 24px; padding: 0 8px; margin-left: -8px;
  border: none; border-radius: 7px; background: none; color: inherit; font: inherit; white-space: nowrap; cursor: pointer;
}
.presence.compacte { gap: 4px; padding: 0 6px; }
.presence:hover, .presence.ouvert { color: var(--text-dim); background: var(--surface-2); }
.presence .caret { width: 11px; height: 11px; opacity: 0; transition: opacity 0.12s; }
.presence:hover .caret, .presence.ouvert .caret, .presence:focus-visible .caret { opacity: 0.7; }

.menu {
  position: absolute; top: calc(100% + 6px); left: -8px; min-width: 200px; padding: 6px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px; box-shadow: var(--shadow-hero);
}
.menu-tete {
  margin: 6px 10px 8px; font-size: 11px; font-weight: 700; letter-spacing: 0.07em;
  text-transform: uppercase; color: var(--text-faint);
}
.menu-choix {
  display: flex; align-items: center; gap: 10px; width: 100%; padding: 8px 10px;
  border: none; border-radius: 9px; background: none; color: var(--text); font: inherit; text-align: left; cursor: pointer;
}
.menu-choix:hover, .menu-choix:focus-visible { background: var(--surface-2); outline: none; }
.menu-choix.on { background: var(--accent-soft); }
.menu-nom { flex: 1; font-size: 13px; font-weight: 600; white-space: nowrap; }
.coche { width: 15px; height: 15px; flex: none; color: var(--accent); }
.rond {
  width: 10px; height: 10px; border-radius: 50%; flex: none;
  border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent); border-top-color: var(--accent);
  animation: tourne 0.8s linear infinite;
}
@keyframes tourne { to { transform: rotate(360deg); } }

.partie {
  display: inline-flex; align-items: center; gap: 7px; min-width: 0; height: 24px; padding: 0 10px;
  border-radius: 99px; border: 1px solid color-mix(in srgb, #3ad07f 35%, transparent);
  background: color-mix(in srgb, #3ad07f 10%, transparent); color: var(--text-dim);
  font: inherit; cursor: pointer;
}
.partie:hover { color: var(--text); border-color: color-mix(in srgb, #3ad07f 60%, transparent); }
.quoi { color: #3ad07f; font-weight: 700; white-space: nowrap; }
.titre { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; color: var(--text); }
.duree { font-family: var(--mono); font-size: 11px; color: var(--text-faint); white-space: nowrap; }

/* Les mêmes points que la présence des amis (`FriendsView`). */
.pastille { width: 7px; height: 7px; border-radius: 50%; flex: none; background: var(--text-faint); }
.pastille.en-jeu, .pastille.en-ligne { background: #3ad07f; box-shadow: 0 0 6px #3ad07f; }
.pastille.invisible { background: none; border: 1.5px solid var(--text-faint); box-sizing: border-box; }
</style>
