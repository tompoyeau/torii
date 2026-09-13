<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { usePreferences } from "../composables/usePreferences";
import { useStore } from "../composables/useStore";
import { useTheme } from "../composables/useTheme";
import { useUi } from "../composables/useUi";
import { useScrollLock } from "../composables/useScrollLock";
import { useFocusTrap } from "../composables/useFocusTrap";
import { useUpdater } from "../composables/useUpdater";
import { platformName } from "../data/platforms";
import { appVersion, clearCaches, getAutostart, getSettings, getWindowPrefs, openExternal, openLog, relaunchApp, setAutostart, setWindowPrefs } from "../lib/tauri";
import { etiquetteIntl, LANGUES, langueCourante, langueParDefaut, t, type Langue } from "../i18n";
import { deviseAttendue, instantGamingPertinent, nomDeRegion, REGIONS, regionDuSysteme, type CodeRegion } from "../i18n/regions";
import { ilYA } from "../lib/format";
import PlatformIcon from "./PlatformIcon.vue";
import AccountsSettings from "./AccountsSettings.vue";
import ToriiPanel from "./ToriiPanel.vue";
import { useTorii } from "../composables/useTorii";
import { useFriends } from "../composables/useFriends";
import { showToast } from "../composables/useToast";

const { games, setHidden } = useLibrary();
const { prefs } = usePreferences();
const { excludedStores, toggleStoreExcluded, clearExcludedStores } = useStore();
const {
  account: toriiAccount, connected: toriiConnected, prefs: toriiPrefs, mutedGames,
  suggestions, searching, searched, presenceMode,
  setPrefs: setToriiPrefs, setPresenceMode, setMuted, setSteamLink, logout: toriiLogout,
  deleteAccount: toriiDeleteAccount,
  findSteamFriends, inviteAccount, rotateCode, setDisplayName,
  libraryIndex: toriiLibrary, librarySyncing, refreshLibraryIndex,
  setLibrarySync, setShareLibrary, syncLibraryNow, forgetDevice,
  devices, devicesLoading, refreshDevices, revokeDevice, revokeOtherDevices,
} = useTorii();

/* ── Bibliothèque synchronisée ─────────────────────────────────────────────── */

/** Dernier refus du serveur sur la synchronisation, affiché sous les interrupteurs. */
const libraryError = ref<string | null>(null);

/** Cet appareil tel que le serveur le connaît. Null tant que rien n'est parti. */
const myDevice = computed(
  () => toriiLibrary.value.mine.find((d) => d.deviceId === toriiPrefs.value.deviceId) ?? null,
);
/** Les autres PC du même compte : un ancien portable dont la bibliothèque traîne encore. */
const otherDevices = computed(
  () => toriiLibrary.value.mine.filter((d) => d.deviceId !== toriiPrefs.value.deviceId),
);

/** « il y a 3 min » — un horodatage brut ne dit rien à quelqu'un qui vérifie un envoi. */
const sinceLabel = (timestamp: number): string => ilYA(timestamp);

async function withLibrary(action: () => Promise<unknown>) {
  libraryError.value = null;
  try {
    await action();
  } catch (e) {
    libraryError.value = e instanceof Error ? e.message : String(e);
  }
}

const onToggleLibrarySync = () =>
  withLibrary(async () => {
    const on = !toriiPrefs.value.syncLibrary;
    await setLibrarySync(on);
    showToast(on ? t("reglages.torii.bibliotheque.activee") : t("reglages.torii.bibliotheque.coupee"));
  });

const onToggleShareLibrary = () =>
  withLibrary(() => setShareLibrary(!toriiAccount.value?.shareLibrary));

const onSyncLibraryNow = () =>
  withLibrary(async () => {
    const res = await syncLibraryNow();
    showToast(res.uploaded
      ? t("reglages.torii.bibliotheque.envoyee", { n: res.gameCount })
      : t("reglages.torii.bibliotheque.rienAEnvoyer"));
  });

const onForgetDevice = (deviceId: string) => withLibrary(() => forgetDevice(deviceId));

/* ── Appareils connectés ───────────────────────────────────────────────────── */

/**
 * ⚠️ Rien à voir avec `otherDevices` juste au-dessus : ceux-là ont déposé une
 * bibliothèque, ceux-ci ont une session ouverte. Un PC peut être connecté sans jamais
 * avoir rien synchronisé — et c'est précisément celui qu'on veut pouvoir déconnecter.
 */
const devicesError = ref<string | null>(null);
const revokeAllOpen = ref(false);
const otherSessions = computed(() => devices.value.filter((d) => !d.current));

async function withDevices(action: () => Promise<unknown>) {
  devicesError.value = null;
  try {
    await action();
  } catch (e) {
    devicesError.value = e instanceof Error ? e.message : String(e);
  }
}

const onRevokeDevice = (id: string, nom: string) =>
  withDevices(async () => {
    await revokeDevice(id);
    showToast(t("reglages.torii.appareils.deconnecte", { nom }));
  });

const onRevokeOthers = () =>
  withDevices(async () => {
    const combien = otherSessions.value.length;
    await revokeOtherDevices();
    revokeAllOpen.value = false;
    showToast(t("reglages.torii.appareils.deconnectes", { n: combien }));
  });

/** Pseudo en cours d'édition (non enregistré tant qu'on ne valide pas). */
const pseudoDraft = ref("");
const pseudoBusy = ref(false);
/** Rien à enregistrer si le champ est vide ou identique à ce qui est déjà en place. */
const pseudoDirty = computed(() => {
  const v = pseudoDraft.value.trim();
  return !!v && v !== toriiAccount.value?.displayName;
});
watch(
  toriiAccount,
  (a) => { if (a && !pseudoDirty.value) pseudoDraft.value = a.displayName; },
  { immediate: true },
);
async function onSavePseudo() {
  if (!pseudoDirty.value || pseudoBusy.value) return;
  pseudoBusy.value = true;
  try {
    await setDisplayName(pseudoDraft.value.trim());
    showToast(t("reglages.torii.pseudo.misAJour"));
  } finally {
    pseudoBusy.value = false;
  }
}

/**
 * Les trois niveaux de partage, du plus ouvert au plus discret.
 *
 * ⚠️ `computed`, pas une constante : un tableau de libellés calculé une fois au
 * chargement garderait la langue du démarrage, et ne suivrait pas un changement fait
 * dans ce même écran. Même raison pour toutes les listes de choix ci-dessous.
 */
const PRESENCE_MODES = computed(() => [
  { key: "detailed" as const, label: t("reglages.torii.presence.detaille"), hint: t("reglages.torii.presence.detailleAide") },
  { key: "online" as const, label: t("reglages.torii.presence.enLigne"), hint: t("reglages.torii.presence.enLigneAide") },
  { key: "offline" as const, label: t("reglages.torii.presence.invisible"), hint: t("reglages.torii.presence.invisibleAide") },
]);
const { friends: steamFriends, refresh: refreshSteamFriends } = useFriends();

/** Délais d'inactivité proposés avant de passer « absent ». */
const AWAY_DELAYS = [5, 10, 20, 30] as const;

/** Renouvelle le code d'ami ; l'ancien cesse aussitôt de fonctionner. */
async function onRotateCode() {
  await rotateCode();
  showToast(t("reglages.torii.code.renouvele"));
}

/** Jeux réduits au silence, résolus en titres depuis la bibliothèque. */
const mutedList = computed(() =>
  mutedGames.value.map((id) => ({
    id,
    title: games.value.find((g) => g.id === id)?.title ?? id,
    platform: games.value.find((g) => g.id === id)?.platform ?? "manual",
  })),
);

/** SteamID de l'utilisateur, pour pouvoir lier son compte Torii à son compte Steam. */
const mySteamId = ref<string | null>(null);

/**
 * Lie le compte Torii au compte Steam connecté, ou rompt le lien. Les deux vont
 * ensemble : sans SteamID enregistré, « visible par mes amis Steam » n'a aucun effet.
 */
async function onToggleSteamLink() {
  // 🔑 L'état du bouton suit `steamDiscoverable`, et RIEN d'autre. Avant, l'affichage
  // suivait `steamDiscoverable` mais l'action se décidait sur `steamId` : dès que les
  // deux divergeaient, chaque clic était lu comme « délier » et le bouton ne répondait
  // plus. On envoie donc toujours les deux champs ensemble, cohérents par construction.
  const visible = !toriiAccount.value?.steamDiscoverable;
  steamLinkError.value = null;
  try {
    await setSteamLink(visible ? mySteamId.value : "", visible);
  } catch (e) {
    // Le refus le plus probable : ce compte Steam est déjà relié ailleurs. Le message
    // s'affiche SOUS l'interrupteur, à l'endroit exact où le clic n'a pas eu l'effet
    // attendu — un toast passe et s'en va, alors que l'explication doit rester sous les
    // yeux tant que l'interrupteur est dans un état qui n'est pas celui qu'on voulait.
    steamLinkError.value = e instanceof Error ? e.message : String(e);
  }
}

/** Dernier refus du serveur sur le lien Steam. Effacé dès qu'une tentative aboutit. */
const steamLinkError = ref<string | null>(null);

/* ── Suppression du compte ─────────────────────────────────────────────────── */

/**
 * Suppression définitive, derrière une confirmation qui demande de **recopier son
 * pseudo**. Ce n'est pas de la cérémonie : c'est la seule action de l'application qu'on
 * ne peut pas défaire, et elle est à deux centimètres d'un bouton « Déconnecter » qui,
 * lui, est anodin. Recopier son nom oblige à lire ce qu'on est en train de faire.
 */
const deleteOpen = ref(false);
const deleteEcho = ref("");
const deleteBusy = ref(false);
const deleteError = ref<string | null>(null);

const deleteReady = computed(
  () => deleteEcho.value.trim() === (toriiAccount.value?.displayName ?? "").trim(),
);

function openDelete() {
  deleteOpen.value = true;
  deleteEcho.value = "";
  deleteError.value = null;
}

async function onDeleteAccount() {
  if (!deleteReady.value || deleteBusy.value) return;
  deleteBusy.value = true;
  deleteError.value = null;
  try {
    await toriiDeleteAccount();
    deleteOpen.value = false;
    showToast(t("reglages.torii.suppression.fait"));
  } catch (e) {
    deleteError.value = e instanceof Error ? e.message : String(e);
  } finally {
    deleteBusy.value = false;
  }
}

/** Le lien Steam n'a de sens que si un compte Steam est connecté dans Torii. */
const canLinkSteam = computed(() => !!mySteamId.value);

/**
 * Cherche les amis Steam qui ont un compte Torii. On envoie la liste de SteamID de nos
 * amis Steam ; le serveur ne répond que pour ceux qui se sont rendus découvrables.
 */
async function onFindSteamFriends() {
  if (!steamFriends.value.length) await refreshSteamFriends();
  await findSteamFriends(steamFriends.value.map((f) => f.steamId));
}

/** Nom Steam d'une suggestion, plus parlant que le pseudo Torii. */
function steamNameOf(steamId: string | null | undefined): string | null {
  return steamFriends.value.find((f) => f.steamId === steamId)?.name ?? null;
}
const { theme, setTheme } = useTheme();
const { settingsOpen, settingsCategory, setSettingsCategory, closeSettings } = useUi();
const { status: updateStatus, version: updateVersion, check: checkUpdate, install: installUpdate } = useUpdater();

const CATEGORIES = computed(() => [
  { key: "general", label: t("reglages.categories.general"), group: t("reglages.groupes.application") },
  { key: "about", label: t("reglages.categories.apropos"), group: t("reglages.groupes.application") },
  { key: "hidden", label: t("reglages.categories.masques"), group: t("reglages.groupes.bibliotheque") },
  { key: "stores", label: t("reglages.categories.revendeurs"), group: t("reglages.groupes.bibliotheque") },
  { key: "accounts", label: t("reglages.categories.comptes"), group: t("reglages.groupes.comptes") },
  { key: "torii", label: t("reglages.categories.torii"), group: t("reglages.groupes.comptes") },
] as const);

// --- Choix pour les préférences (segmented) --------------------------------
const START_FILTERS = computed(() => [
  { key: "all", label: t("reglages.general.vue.tous") },
  { key: "favorite", label: t("reglages.general.vue.favoris") },
  { key: "installed", label: t("reglages.general.vue.installes") },
] as const);
const START_SORTS = computed(() => [
  { key: "recent", label: t("reglages.general.vue.recent") },
  { key: "alpha", label: t("reglages.general.vue.alpha") },
  { key: "playtime", label: t("reglages.general.vue.tempsDeJeu") },
] as const);
const DENSITIES = computed(() => [
  { key: "compact", label: t("reglages.general.densite.compact") },
  { key: "normal", label: t("reglages.general.densite.normal") },
  { key: "large", label: t("reglages.general.densite.grand") },
] as const);

// --- À propos & maintenance -------------------------------------------------
useScrollLock(settingsOpen);

/** Le clavier ne doit pas sortir des Paramètres, et doit revenir d’où il vient. */
const modale = ref<HTMLElement | null>(null);
useFocusTrap(modale, settingsOpen);

const version = ref<string | null>(null);
const cacheMsg = ref("");
const cacheBusy = ref(false);
onMounted(async () => {
  version.value = await appVersion();
});
const updateLabel = computed(() => {
  switch (updateStatus.value) {
    case "checking": return t("reglages.apropos.maj.verification");
    case "available": return t("reglages.apropos.maj.disponible", { version: updateVersion.value ?? "" });
    case "downloading": return t("reglages.apropos.maj.telechargement");
    case "ready": return t("reglages.apropos.maj.installee");
    case "error": return t("reglages.apropos.maj.erreur");
    default: return t("reglages.apropos.maj.aJour");
  }
});
/**
 * Ouvre un rapport de problème pré-rempli, hors de l'application.
 *
 * 🔑 POURQUOI PRÉ-REMPLIR. Un rapport sans version ni plateforme coûte un aller-retour à
 * tout le monde, et la moitié n'ont pas de deuxième tour : la personne a déjà désinstallé.
 * La version part donc toute seule, et le texte rappelle où trouver le journal — dont le
 * bouton est juste au-dessus, ce qui n'est pas un hasard.
 *
 * ⚠️ Les sauts de ligne doivent être encodés (`encodeURIComponent`), sinon GitHub ne reçoit
 * que la première ligne du corps.
 */
function onSignaler() {
  const corps = [
    t("reglages.apropos.signaler.rapport.cequisepasse"), "", "",
    t("reglages.apropos.signaler.rapport.attendu"), "", "",
    t("reglages.apropos.signaler.rapport.reproduire"), "", "",
    "---", "",
    // Hors Tauri (la démo du site), il n'y a pas de version à donner : le dire clairement
    // vaut mieux qu'un « ? » qui ressemble à un bug, et identifie le rapport pour ce
    // qu'il est — quelqu'un qui essaie dans un onglet, pas une installation en panne.
    version.value ? `Torii ${version.value} · Windows` : t("reglages.apropos.signaler.rapport.demo"),
    "",
    t("reglages.apropos.signaler.rapport.journal1"),
    t("reglages.apropos.signaler.rapport.journal2"),
  ].join("\n");
  openExternal(
    `https://github.com/tompoyeau/torii/issues/new?body=${encodeURIComponent(corps)}`,
  );
}

async function onClearCache() {
  if (cacheBusy.value) return;
  cacheBusy.value = true;
  cacheMsg.value = "";
  const n = await clearCaches();
  cacheBusy.value = false;
  cacheMsg.value = n == null
    ? t("reglages.apropos.cache.indisponible")
    : t("reglages.apropos.cache.vide", { n });
}
// Groupes ordonnés (pour les libellés de section du rail).
const groups = computed(() => {
  const seen: string[] = [];
  for (const c of CATEGORIES.value) if (!seen.includes(c.group)) seen.push(c.group);
  return seen.map((g) => ({ label: g, items: CATEGORIES.value.filter((c) => c.group === g) }));
});

// --- Lancement au démarrage de Windows -------------------------------------
const autostart = ref(false);
const autostartBusy = ref(false);
// Préférences de fenêtre (persistées côté Rust).
const startMinimized = ref(false);
const closeToTray = ref(false);
async function refreshSystemPrefs() {
  // L'index des bibliothèques suit le même chemin : il a pu changer depuis un autre PC.
  // Les sessions ouvertes aussi — c'est même le seul endroit d'où on peut les voir.
  if (toriiConnected.value) {
    void refreshLibraryIndex().catch(() => {});
    void refreshDevices().catch(() => {});
  }
  // Relu à chaque ouverture : quelqu'un qui vient de connecter Steam ne doit pas avoir
  // à redémarrer Torii pour que « visible par mes amis Steam » devienne cliquable.
  mySteamId.value = (await getSettings())?.steamId ?? null;
  autostart.value = await getAutostart();
  const wp = await getWindowPrefs();
  startMinimized.value = wp.startMinimized;
  closeToTray.value = wp.closeToTray;
}
onMounted(() => {
  document.addEventListener("keydown", onKey);
  void refreshSystemPrefs();
});
onBeforeUnmount(() => document.removeEventListener("keydown", onKey));
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && settingsOpen.value) closeSettings();
}
// Recharge l'état à chaque ouverture (il a pu changer côté système).
watch(settingsOpen, (open) => {
  if (open) void refreshSystemPrefs();
});
async function onToggleAutostart() {
  if (autostartBusy.value) return;
  autostartBusy.value = true;
  autostart.value = await setAutostart(!autostart.value);
  autostartBusy.value = false;
}
function saveWindowPrefs() {
  void setWindowPrefs({ startMinimized: startMinimized.value, closeToTray: closeToTray.value });
}
function toggleStartMinimized() {
  startMinimized.value = !startMinimized.value;
  saveWindowPrefs();
}
function toggleCloseToTray() {
  closeToTray.value = !closeToTray.value;
  saveWindowPrefs();
}

// --- Thème ------------------------------------------------------------------
type ThemeChoice = "system" | "light" | "dark";
const themeChoice = computed<ThemeChoice>(() => theme.value ?? "system");
function pickTheme(c: ThemeChoice) {
  setTheme(c === "system" ? null : c);
}
const THEMES = computed<{ key: ThemeChoice; label: string }[]>(() => [
  { key: "system", label: t("reglages.general.theme.systeme") },
  { key: "light", label: t("reglages.general.theme.clair") },
  { key: "dark", label: t("reglages.general.theme.sombre") },
]);

// --- Langue et région -------------------------------------------------------

/**
 * Le bouton allumé du sélecteur. Une langue jamais choisie (`null`) s'affiche comme la
 * langue par défaut qu'elle produit — pas comme « Suivre Windows », qui est un choix.
 */
const langueChoisie = computed<Langue | "system">(() => prefs.language ?? langueParDefaut());

/**
 * La langue affichée au chargement de l'application.
 *
 * 🔑 Sert à savoir s'il faut proposer de redémarrer. L'interface bascule tout de suite,
 * mais les descriptions et genres des jeux déjà en mémoire ont été chargés dans l'ancienne
 * langue et n'en changeront qu'au prochain lancement. On ne le signale que si la langue
 * **effective** a changé : passer de « Suivre Windows » à « Français » sur un Windows
 * français ne change rien à l'écran, et proposer un redémarrage serait absurde.
 */
const langueAuDemarrage = langueCourante.value;
const langueAChange = computed(() => langueCourante.value !== langueAuDemarrage);

function choisirLangue(choix: Langue | "system") {
  prefs.language = choix;
}

const regionChoisie = computed<CodeRegion | "system">(() => prefs.region ?? "system");
function choisirRegion(choix: string) {
  prefs.region = choix === "system" ? null : (choix as CodeRegion);
}

/** Les régions, triées par leur nom dans la langue affichée — pas par code pays. */
const regionsTriees = computed(() =>
  [...REGIONS].sort((a, b) =>
    nomDeRegion(a.code).localeCompare(nomDeRegion(b.code), etiquetteIntl.value),
  ),
);

// --- Jeux masqués -----------------------------------------------------------
const hiddenGames = computed(() =>
  games.value.filter((g) => g.hidden).sort((a, b) => a.title.localeCompare(b.title, etiquetteIntl.value)),
);
function unhide(id: string) {
  void setHidden(id, false);
}
</script>

<template>
  <div v-if="settingsOpen" class="overlay" @click.self="closeSettings">
    <div ref="modale" class="dialog" role="dialog" aria-modal="true" tabindex="-1" :aria-label="t('reglages.titre')">
      <!-- Rail de navigation -->
      <aside class="snav">
        <div class="snav-title">{{ t("reglages.titre") }}</div>
        <template v-for="grp in groups" :key="grp.label">
          <div class="snav-group">{{ grp.label }}</div>
          <button
            v-for="c in grp.items"
            :key="c.key"
            class="snav-item"
            :class="{ on: settingsCategory === c.key }"
            @click="setSettingsCategory(c.key)"
          >
            {{ c.label }}
            <span v-if="c.key === 'hidden' && hiddenGames.length" class="snav-count">{{ hiddenGames.length }}</span>
            <span v-else-if="c.key === 'stores' && excludedStores.length" class="snav-count">{{ excludedStores.length }}</span>
          </button>
        </template>
      </aside>

      <!-- Contenu -->
      <section class="spane">
        <button class="close" :aria-label="t('commun.fermer')" @click="closeSettings">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>

        <div class="spane-inner">
          <!-- Paramètres généraux -->
          <template v-if="settingsCategory === 'general'">
            <h2 class="pane-title">{{ t("reglages.categories.general") }}</h2>

            <!--
              🔑 LANGUE ET RÉGION EN TÊTE, et pas au milieu des réglages d'affichage. C'est
              ce qu'un nouvel utilisateur cherche en premier quand l'application ne parle
              pas sa langue — et il le cherche sans pouvoir lire les intitulés.
            -->
            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.langue.titre") }}</span>
                <span class="row-sub">{{ t("reglages.langue.description") }}</span>
              </div>
              <div class="segmented">
                <button class="seg" :class="{ on: langueChoisie === 'system' }" @click="choisirLangue('system')">
                  {{ t("reglages.langue.systeme") }}
                </button>
                <!--
                  ⚠️ Le nom de chaque langue est écrit DANS cette langue (« English »,
                  « Français »), jamais traduit. Quelqu'un bloqué dans une interface qu'il
                  ne lit pas doit pouvoir reconnaître la sienne.
                -->
                <button
                  v-for="l in LANGUES"
                  :key="l.code"
                  class="seg"
                  :class="{ on: langueChoisie === l.code }"
                  :lang="l.code"
                  @click="choisirLangue(l.code)"
                >
                  {{ l.nom }}
                </button>
              </div>
            </div>
            <div v-if="langueAChange" class="row-note">
              <span>{{ t("reglages.langue.appliquer") }}</span>
              <button class="ghost-btn" @click="relaunchApp()">{{ t("reglages.langue.redemarrer") }}</button>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.region.titre") }}</span>
                <span class="row-sub">
                  {{ t("reglages.region.description") }} {{ t("reglages.region.note") }}
                  <template v-if="!instantGamingPertinent"> {{ t("prix.revendeurIndisponible") }}</template>
                </span>
              </div>
              <div class="region">
                <select
                  class="region-select"
                  :value="regionChoisie"
                  :aria-label="t('reglages.region.titre')"
                  @change="choisirRegion(($event.target as HTMLSelectElement).value)"
                >
                  <option value="system">{{ t("reglages.region.systeme", { pays: nomDeRegion(regionDuSysteme()) }) }}</option>
                  <option v-for="r in regionsTriees" :key="r.code" :value="r.code">{{ nomDeRegion(r.code) }}</option>
                </select>
                <span class="region-devise">{{ t("reglages.region.devise", { devise: deviseAttendue }) }}</span>
              </div>
            </div>

            <div class="divider" />

            <button
              class="pref toggle-row"
              role="switch"
              :aria-checked="autostart"
              :disabled="autostartBusy"
              @click="onToggleAutostart"
            >
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.autostart.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.autostart.sous") }}</span>
              </div>
              <span class="switch" :class="{ on: autostart }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <button class="pref toggle-row" role="switch" :aria-checked="startMinimized" @click="toggleStartMinimized">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.minimise.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.minimise.sous") }}</span>
              </div>
              <span class="switch" :class="{ on: startMinimized }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <button class="pref toggle-row" role="switch" :aria-checked="closeToTray" @click="toggleCloseToTray">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.tray.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.tray.sous") }}</span>
              </div>
              <span class="switch" :class="{ on: closeToTray }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <button
              class="pref toggle-row"
              role="switch"
              :aria-checked="prefs.returnOnGameExit"
              @click="prefs.returnOnGameExit = !prefs.returnOnGameExit"
            >
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.retourJeu.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.retourJeu.sous") }}</span>
              </div>
              <span class="switch" :class="{ on: prefs.returnOnGameExit }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.theme.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.theme.sous") }}</span>
              </div>
              <div class="segmented">
                <!-- ⚠️ Pas `t` comme variable de boucle : elle masquerait la fonction de traduction. -->
                <button
                  v-for="th in THEMES"
                  :key="th.key"
                  class="seg"
                  :class="{ on: themeChoice === th.key }"
                  @click="pickTheme(th.key)"
                >
                  {{ th.label }}
                </button>
              </div>
            </div>

            <div class="divider" />

            <div class="pref wrap">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.vue.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.vue.sous") }}</span>
              </div>
              <div class="controls">
                <div class="segmented">
                  <button v-for="f in START_FILTERS" :key="f.key" class="seg" :class="{ on: prefs.defaultFilter === f.key }" @click="prefs.defaultFilter = f.key">{{ f.label }}</button>
                </div>
                <div class="segmented">
                  <button v-for="s in START_SORTS" :key="s.key" class="seg" :class="{ on: prefs.defaultSort === s.key }" @click="prefs.defaultSort = s.key">{{ s.label }}</button>
                </div>
                <div class="segmented">
                  <button class="seg" :class="{ on: !prefs.listView }" @click="prefs.listView = false">{{ t("reglages.general.vue.grille") }}</button>
                  <button class="seg" :class="{ on: prefs.listView }" @click="prefs.listView = true">{{ t("reglages.general.vue.liste") }}</button>
                </div>
              </div>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.densite.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.densite.sous") }}</span>
              </div>
              <div class="segmented">
                <button v-for="d in DENSITIES" :key="d.key" class="seg" :class="{ on: prefs.density === d.key }" @click="prefs.density = d.key">{{ d.label }}</button>
              </div>
            </div>

            <div class="divider" />

            <button
              class="pref toggle-row"
              role="switch"
              :aria-checked="prefs.reduceMotion"
              @click="prefs.reduceMotion = !prefs.reduceMotion"
            >
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.animations.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.animations.sous") }}</span>
              </div>
              <span class="switch" :class="{ on: prefs.reduceMotion }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <button
              class="pref toggle-row"
              role="switch"
              :aria-checked="prefs.wishlistNotifications"
              @click="prefs.wishlistNotifications = !prefs.wishlistNotifications"
            >
              <div class="row-text">
                <span class="row-title">{{ t("reglages.general.alertesPrix.titre") }}</span>
                <span class="row-sub">{{ t("reglages.general.alertesPrix.sous") }}</span>
              </div>
              <span class="switch" :class="{ on: prefs.wishlistNotifications }"><span class="knob" /></span>
            </button>
          </template>

          <!-- À propos & maintenance -->
          <template v-else-if="settingsCategory === 'about'">
            <h2 class="pane-title">{{ t("reglages.categories.apropos") }}</h2>

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.apropos.version") }}</span>
                <span class="row-sub">{{ updateLabel }}</span>
              </div>
              <span class="version-tag">{{ version ?? "—" }}</span>
            </div>

            <div class="row-actions">
              <button class="ghost-btn" :disabled="updateStatus === 'checking' || updateStatus === 'downloading'" @click="checkUpdate(false)">
                {{ t("reglages.apropos.verifier") }}
              </button>
              <button v-if="updateStatus === 'available'" class="primary-btn sm" @click="installUpdate()">
                {{ t("reglages.apropos.installer") }}
              </button>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.apropos.cache.titre") }}</span>
                <span class="row-sub">{{ t("reglages.apropos.cache.sous") }}</span>
              </div>
              <button class="ghost-btn" :disabled="cacheBusy" @click="onClearCache">
                {{ cacheBusy ? t("reglages.apropos.cache.enCours") : t("reglages.apropos.cache.bouton") }}
              </button>
            </div>
            <p v-if="cacheMsg" class="cache-msg">{{ cacheMsg }}</p>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.apropos.journal.titre") }}</span>
                <span class="row-sub">{{ t("reglages.apropos.journal.sous") }}</span>
              </div>
              <button class="ghost-btn" @click="openLog()">{{ t("reglages.apropos.journal.bouton") }}</button>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">{{ t("reglages.apropos.signaler.titre") }}</span>
                <span class="row-sub">{{ t("reglages.apropos.signaler.sous") }}</span>
              </div>
              <button class="ghost-btn" @click="onSignaler()">{{ t("reglages.apropos.signaler.bouton") }}</button>
            </div>
          </template>

          <!-- Jeux masqués -->
          <template v-else-if="settingsCategory === 'hidden'">
            <h2 class="pane-title">{{ t("reglages.categories.masques") }}</h2>
            <p class="pane-hint">{{ t("reglages.masques.sous") }}</p>
            <div v-if="hiddenGames.length" class="items">
              <div v-for="g in hiddenGames" :key="g.id" class="item">
                <div class="thumb" :style="{ background: g.cover }">
                  <img v-if="g.coverUrl" :src="g.coverUrl" alt="" loading="lazy" @error="($event.target as HTMLElement).style.display='none'" />
                </div>
                <div class="item-text">
                  <span class="item-title">{{ g.title }}</span>
                  <span class="item-sub"><PlatformIcon :platform="g.platform" /> {{ platformName(g.platform) }}</span>
                </div>
                <button class="ghost-btn" @click="unhide(g.id)">{{ t("reglages.masques.reafficher") }}</button>
              </div>
            </div>
            <p v-else class="empty">{{ t("reglages.masques.aucun") }}</p>
          </template>

          <!-- Revendeurs masqués -->
          <template v-else-if="settingsCategory === 'stores'">
            <h2 class="pane-title">{{ t("reglages.categories.revendeurs") }}</h2>
            <p class="pane-hint">{{ t("reglages.revendeurs.sous") }}</p>
            <div v-if="excludedStores.length" class="items">
              <div v-for="name in excludedStores" :key="name" class="item">
                <div class="thumb store"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 8h16l-1 4a3 3 0 0 1-3 2.4H8A3 3 0 0 1 5 12Z" /><path d="M4 8l1.4-3.4A2 2 0 0 1 7.2 3.4h9.6a2 2 0 0 1 1.8 1.2L20 8" /></svg></div>
                <div class="item-text"><span class="item-title">{{ name }}</span></div>
                <button class="ghost-btn" @click="toggleStoreExcluded(name)">{{ t("reglages.revendeurs.reafficher") }}</button>
              </div>
              <button class="clear-all" @click="clearExcludedStores">{{ t("reglages.revendeurs.toutReafficher") }}</button>
            </div>
            <p v-else class="empty">{{ t("reglages.revendeurs.aucun") }}</p>
          </template>

          <!-- Réseau Torii -->
          <template v-else-if="settingsCategory === 'torii'">
            <h2 class="pane-title">{{ t("reglages.categories.torii") }}</h2>
            <p class="pane-hint">{{ t("reglages.torii.intro") }}</p>

            <ToriiPanel />

            <template v-if="toriiConnected">
              <div class="pref">
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.pseudo.titre") }}</span>
                  <span class="row-sub">{{ t("reglages.torii.pseudo.sous") }}</span>
                </div>
                <div class="row-actions" style="margin-top: 0">
                  <input v-model="pseudoDraft" class="pseudo-input" maxlength="40" spellcheck="false" />
                  <button class="ghost-btn" :disabled="!pseudoDirty || pseudoBusy" @click="onSavePseudo">
                    {{ pseudoBusy ? t("commun.enCours") : t("commun.enregistrer") }}
                  </button>
                </div>
              </div>

              <div class="divider" />

              <div class="pref">
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.code.titre") }}</span>
                  <span class="row-sub">{{ t("reglages.torii.code.sous") }}</span>
                </div>
                <div class="row-actions" style="margin-top: 0">
                  <span class="friend-code">{{ toriiAccount?.friendCode }}</span>
                  <button class="ghost-btn" @click="onRotateCode">{{ t("reglages.torii.code.renouveler") }}</button>
                </div>
              </div>

              <div class="divider" />

              <div class="pref presence-choice">
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.presence.titre") }}</span>
                  <span class="row-sub">{{ t("reglages.torii.presence.sous") }}</span>
                </div>
                <div class="modes">
                  <button
                    v-for="m in PRESENCE_MODES"
                    :key="m.key"
                    class="mode"
                    :class="{ on: presenceMode === m.key }"
                    @click="setPresenceMode(m.key)"
                  >
                    <span class="mode-label">{{ m.label }}</span>
                    <span class="mode-hint">{{ m.hint }}</span>
                  </button>
                </div>
              </div>

              <div class="divider" />

              <div class="pref">
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.absent.titre") }}</span>
                  <span class="row-sub">{{ t("reglages.torii.absent.sous") }}</span>
                </div>
                <div class="segmented">
                  <button
                    v-for="d in AWAY_DELAYS"
                    :key="d"
                    class="seg"
                    :class="{ on: toriiPrefs.awayAfterMinutes === d }"
                    @click="setToriiPrefs({ awayAfterMinutes: d })"
                  >
                    {{ t("reglages.torii.absent.minutes", { n: d }) }}
                  </button>
                </div>
              </div>

              <div class="divider" />

              <button
                class="pref toggle-row"
                role="switch"
                :aria-checked="!!toriiAccount?.steamDiscoverable"
                :disabled="!canLinkSteam"
                @click="onToggleSteamLink"
              >
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.steam.titre") }}</span>
                  <span class="row-sub">
                    {{ canLinkSteam ? t("reglages.torii.steam.sous") : t("reglages.torii.steam.sousSansSteam") }}
                  </span>
                </div>
                <span class="switch" :class="{ on: toriiAccount?.steamDiscoverable }"><span class="knob" /></span>
              </button>
              <p v-if="steamLinkError" class="row-error" role="alert">{{ steamLinkError }}</p>

              <template v-if="toriiAccount?.steamDiscoverable">
                <div class="divider" />
                <h3 class="sub-title">{{ t("reglages.torii.steam.retrouver") }}</h3>
                <p class="pane-hint">{{ t("reglages.torii.steam.retrouverAide") }}</p>
                <div class="row-actions">
                  <button class="ghost-btn" :disabled="searching" @click="onFindSteamFriends">
                    {{ searching ? t("commun.recherche") : t("reglages.torii.steam.chercher") }}
                  </button>
                </div>
                <div v-if="suggestions.length" class="items">
                  <div v-for="p in suggestions" :key="p.id" class="item">
                    <div class="thumb store">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 5.2a3 3 0 0 1 0 5.6M17.5 20a5.5 5.5 0 0 0-3-4.9" /></svg>
                    </div>
                    <div class="item-text">
                      <span class="item-title">{{ p.displayName }}</span>
                      <span v-if="steamNameOf(p.steamId)" class="item-sub">
                        {{ t("reglages.torii.steam.surSteam", { nom: steamNameOf(p.steamId) ?? "" }) }}
                      </span>
                    </div>
                    <button class="ghost-btn" @click="inviteAccount(p.id)">{{ t("commun.ajouter") }}</button>
                  </div>
                </div>
                <p v-else-if="searched && !searching" class="empty">{{ t("reglages.torii.steam.aucun") }}</p>
              </template>

              <div class="divider" />

              <h3 class="sub-title">{{ t("reglages.torii.bibliotheque.titre") }}</h3>
              <p class="pane-hint">{{ t("reglages.torii.bibliotheque.aide") }}</p>

              <button
                class="pref toggle-row"
                role="switch"
                :aria-checked="toriiPrefs.syncLibrary"
                :disabled="librarySyncing"
                @click="onToggleLibrarySync"
              >
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.bibliotheque.synchro") }}</span>
                  <span class="row-sub">{{ t("reglages.torii.bibliotheque.synchroSous") }}</span>
                </div>
                <span class="switch" :class="{ on: toriiPrefs.syncLibrary }"><span class="knob" /></span>
              </button>

              <button
                class="pref toggle-row spaced"
                role="switch"
                :aria-checked="!!toriiAccount?.shareLibrary"
                :disabled="!toriiPrefs.syncLibrary"
                @click="onToggleShareLibrary"
              >
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.bibliotheque.partage") }}</span>
                  <span class="row-sub">
                    {{ toriiPrefs.syncLibrary ? t("reglages.torii.bibliotheque.partageSous") : t("reglages.torii.bibliotheque.partageSansSynchro") }}
                  </span>
                </div>
                <span class="switch" :class="{ on: toriiAccount?.shareLibrary }"><span class="knob" /></span>
              </button>

              <p v-if="libraryError" class="row-error" role="alert">{{ libraryError }}</p>

              <template v-if="toriiPrefs.syncLibrary">
                <p class="sync-state">
                  <template v-if="myDevice">
                    {{ t("reglages.torii.bibliotheque.envoye", { n: myDevice.gameCount, appareil: myDevice.deviceName, quand: sinceLabel(myDevice.updatedAt) }) }}
                  </template>
                  <template v-else>{{ t("reglages.torii.bibliotheque.rienEnvoye") }}</template>
                </p>
                <div class="row-actions">
                  <button class="ghost-btn" :disabled="librarySyncing" @click="onSyncLibraryNow">
                    {{ librarySyncing ? t("reglages.torii.bibliotheque.envoi") : t("reglages.torii.bibliotheque.maintenant") }}
                  </button>
                </div>

                <template v-if="otherDevices.length">
                  <h3 class="sub-title">{{ t("reglages.torii.bibliotheque.autres") }}</h3>
                  <p class="pane-hint">{{ t("reglages.torii.bibliotheque.autresAide") }}</p>
                  <div class="items">
                    <div v-for="d in otherDevices" :key="d.deviceId" class="item">
                      <div class="item-text">
                        <span class="item-title">{{ d.deviceName }}</span>
                        <span class="item-sub">{{ t("reglages.torii.bibliotheque.ligneAppareil", { n: d.gameCount, quand: sinceLabel(d.updatedAt) }) }}</span>
                      </div>
                      <button class="ghost-btn" @click="onForgetDevice(d.deviceId)">{{ t("commun.retirer") }}</button>
                    </div>
                  </div>
                </template>
              </template>

              <div class="divider" />

              <button
                class="pref toggle-row"
                role="switch"
                :aria-checked="toriiPrefs.notifyFriendLaunch"
                @click="setToriiPrefs({ notifyFriendLaunch: !toriiPrefs.notifyFriendLaunch })"
              >
                <div class="row-text">
                  <span class="row-title">{{ t("reglages.torii.notifAmi.titre") }}</span>
                  <span class="row-sub">{{ t("reglages.torii.notifAmi.sous") }}</span>
                </div>
                <span class="switch" :class="{ on: toriiPrefs.notifyFriendLaunch }"><span class="knob" /></span>
              </button>

              <div class="divider" />

              <h3 class="sub-title">{{ t("reglages.torii.silence.titre") }}</h3>
              <p class="pane-hint">{{ t("reglages.torii.silence.aide") }}</p>
              <div v-if="mutedList.length" class="items">
                <div v-for="g in mutedList" :key="g.id" class="item">
                  <div class="thumb"><PlatformIcon :platform="g.platform" /></div>
                  <div class="item-text"><span class="item-title">{{ g.title }}</span></div>
                  <button class="ghost-btn" @click="setMuted(g.id, false)">{{ t("reglages.torii.silence.rediffuser") }}</button>
                </div>
              </div>
              <p v-else class="empty">{{ t("reglages.torii.silence.aucun") }}</p>

              <div class="divider" />

              <h3 class="sub-title">{{ t("reglages.torii.appareils.titre") }}</h3>
              <p class="pane-hint">{{ t("reglages.torii.appareils.aide") }}</p>

              <p v-if="devicesError" class="row-error" role="alert">{{ devicesError }}</p>

              <div v-if="devices.length" class="items">
                <div v-for="d in devices" :key="d.id" class="item">
                  <div class="item-text">
                    <span class="item-title">
                      {{ d.device }}
                      <span v-if="d.current" class="tag">{{ t("reglages.torii.appareils.celuiCi") }}</span>
                    </span>
                    <span class="item-sub">
                      {{ t("reglages.torii.appareils.connecte", { quand: sinceLabel(d.createdAt) }) }}<template v-if="!d.current">
                        · {{ t("reglages.torii.appareils.actif", { quand: sinceLabel(d.lastSeenAt) }) }}</template>
                    </span>
                  </div>
                  <button
                    v-if="!d.current"
                    class="ghost-btn"
                    @click="onRevokeDevice(d.id, d.device)"
                  >
                    {{ t("reglages.torii.appareils.deconnecter") }}
                  </button>
                </div>
              </div>
              <p v-else class="empty">
                {{ devicesLoading ? t("commun.chargement") : t("reglages.torii.appareils.aucun") }}
              </p>

              <div v-if="otherSessions.length" class="row-actions">
                <button v-if="!revokeAllOpen" class="ghost-btn" @click="revokeAllOpen = true">
                  {{ t("reglages.torii.appareils.deconnecterTous") }}
                </button>
                <template v-else>
                  <span class="confirm-lead">
                    {{ t("reglages.torii.appareils.confirmationTous", { n: otherSessions.length }) }}
                  </span>
                  <button class="danger-btn" @click="onRevokeOthers">{{ t("commun.confirmer") }}</button>
                  <button class="ghost-btn" @click="revokeAllOpen = false">{{ t("commun.annuler") }}</button>
                </template>
              </div>

              <div class="row-actions">
                <button class="ghost-btn" @click="toriiLogout()">{{ t("reglages.torii.appareils.deconnecterCompte") }}</button>
              </div>

              <div class="divider" />

              <h3 class="sub-title danger">{{ t("reglages.torii.suppression.titre") }}</h3>
              <p class="pane-hint">{{ t("reglages.torii.suppression.aide") }}</p>

              <div v-if="!deleteOpen" class="row-actions">
                <button class="danger-btn" @click="openDelete">{{ t("reglages.torii.suppression.bouton") }}</button>
              </div>

              <div v-else class="danger-zone">
                <p class="danger-lead">
                  {{ t("reglages.torii.suppression.avant") }}
                  <strong>{{ toriiAccount?.displayName }}</strong>
                  {{ t("reglages.torii.suppression.apres") }}
                </p>
                <div class="controls">
                  <input
                    v-model="deleteEcho"
                    :placeholder="toriiAccount?.displayName"
                    spellcheck="false"
                    autocomplete="off"
                  />
                  <button
                    class="danger-btn"
                    :disabled="!deleteReady || deleteBusy"
                    @click="onDeleteAccount"
                  >
                    {{ deleteBusy ? t("reglages.torii.suppression.enCours") : t("reglages.torii.suppression.definitif") }}
                  </button>
                  <button class="ghost-btn" @click="deleteOpen = false">{{ t("commun.annuler") }}</button>
                </div>
                <p v-if="deleteError" class="row-error" role="alert">{{ deleteError }}</p>
              </div>
            </template>
          </template>

          <!-- Comptes & launchers -->
          <template v-else>
            <h2 class="pane-title">{{ t("reglages.categories.comptes") }}</h2>
            <AccountsSettings />
          </template>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed; inset: 0; z-index: 200; display: grid; place-items: center; padding: 28px;
  background: rgba(8, 5, 14, 0.55); backdrop-filter: blur(4px);
}
.dialog {
  /* 🔑 UN titre et sa description sont séparés de cette valeur, partout dans les
     Paramètres — qu'il s'agisse d'une ligne d'interrupteur, d'un en-tête de section ou
     d'une ligne de liste. Trois motifs distincts divergeaient (2px, 2px et -4px). */
  --gap-desc: 4px;
  width: min(1120px, 96vw); height: min(760px, 92vh); display: flex; overflow: hidden;
  background: var(--surface); border: 1px solid var(--border); border-radius: 20px;
  box-shadow: var(--shadow-hero);
}

/* Rail gauche */
.snav {
  width: 232px; flex: none; overflow-y: auto;
  background: var(--surface-2); border-right: 1px solid var(--border);
  padding: 20px 12px; display: flex; flex-direction: column; gap: 2px;
}
.snav-title { font-size: 17px; font-weight: 700; letter-spacing: -0.02em; padding: 2px 10px 10px; }
.snav-group {
  font-size: 10px; text-transform: uppercase; letter-spacing: 0.13em;
  color: var(--text-faint); font-weight: 700; padding: 13px 10px 5px;
}
.snav-item {
  display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 9px;
  background: none; border: none; width: 100%; text-align: left; cursor: pointer;
  color: var(--text-dim); font-size: 13.5px; transition: background 0.15s, color 0.15s;
}
.snav-item:hover { background: var(--surface-3); color: var(--text); }
.snav-item.on { background: var(--accent-soft); color: var(--text); font-weight: 600; }
.snav-count {
  margin-left: auto; font-family: var(--mono); font-size: 10.5px; color: var(--text-faint);
  background: var(--surface-3); padding: 1px 7px; border-radius: 99px;
}
.snav-item.on .snav-count { background: var(--accent); color: var(--accent-ink); }

/* Contenu droit */
.spane { flex: 1; min-width: 0; overflow-y: auto; position: relative; }
.close {
  position: absolute; top: 18px; right: 18px; z-index: 2;
  width: 34px; height: 34px; border-radius: 10px; border: 1px solid var(--border);
  background: var(--surface-2); color: var(--text-dim); display: grid; place-items: center; cursor: pointer;
}
.close:hover { color: var(--text); }
.close svg { width: 16px; height: 16px; }
.spane-inner { padding: 30px 40px 32px; max-width: 760px; }
.pane-title { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0 0 10px; }
/* 🔑 PAS de marge haute négative ici. Elle valait -10px pour venir se glisser sous un
   `.pane-title` (marge basse 20px → 10px au final), mais elle rendait l'espacement
   dépendant de ce qui précède : sous un `.sub-title` (marge basse 6px) les marges
   fusionnaient à -4px et la description remontait DANS le titre. C'est aussi ce qui la
   faisait chevaucher un interrupteur. Le `.pane-title` porte désormais l'écart lui-même. */
.pane-hint { font-size: 12.5px; color: var(--text-dim); line-height: 1.5; margin: 0 0 20px; }
/* ⚠️ PAS `.pane-hint` : celui-ci porte un `margin-top` négatif pour se glisser sous un
   titre, et remonterait dans l'interrupteur qui précède cette ligne d'état. */
.sync-state { font-size: 12.5px; color: var(--text-dim); line-height: 1.5; margin: 14px 0 0; }

/* Préférences */
.pref { display: flex; align-items: center; gap: 16px; }
.row-text { display: flex; flex-direction: column; gap: var(--gap-desc); flex: 1; min-width: 0; }
.row-title { font-weight: 600; font-size: 14px; color: var(--text); }
.row-sub { font-size: 12px; color: var(--text-dim); line-height: 1.4; }
/* Refus du serveur, juste sous l'interrupteur concerné : il reste affiché tant que la
   situation n'est pas réglée, là où le clic n'a pas produit l'effet attendu. */
.sub-title.danger { color: #ff6b6b; }
.danger-zone {
  display: flex; flex-direction: column; gap: 12px; margin-top: 12px;
  padding: 14px 16px; border-radius: 12px;
  background: color-mix(in srgb, #ff6b6b 7%, transparent);
  border: 1px solid color-mix(in srgb, #ff6b6b 26%, transparent);
}
.danger-lead { margin: 0; font-size: 13px; line-height: 1.5; color: var(--text-dim); }
.danger-lead strong { color: var(--text); font-weight: 700; }
.danger-zone input {
  padding: 9px 13px; border-radius: 10px; font-size: 13.5px; font-family: inherit;
  background: var(--bg); border: 1px solid var(--border); color: var(--text); min-width: 200px;
}
.danger-zone input:focus { border-color: #ff6b6b; }
.danger-btn {
  padding: 9px 15px; border-radius: 10px; font-size: 13px; font-weight: 600; cursor: pointer;
  font-family: inherit; color: #ff6b6b;
  background: color-mix(in srgb, #ff6b6b 12%, transparent);
  border: 1px solid color-mix(in srgb, #ff6b6b 38%, transparent);
}
.danger-btn:hover:not(:disabled) { background: color-mix(in srgb, #ff6b6b 20%, transparent); }
.danger-btn:disabled { opacity: 0.45; cursor: default; }

.row-error {
  margin: 12px 0 4px; padding: 9px 12px; border-radius: 9px;
  font-size: 12.5px; line-height: 1.45; color: #ff6b6b;
  background: color-mix(in srgb, #ff6b6b 12%, transparent);
  border: 1px solid color-mix(in srgb, #ff6b6b 28%, transparent);
}
.toggle-row { width: 100%; text-align: left; background: none; border: none; padding: 0; cursor: pointer; }
.toggle-row:disabled { cursor: default; opacity: 0.6; }
/* Deux interrupteurs qui vont ensemble : groupés sans séparateur, mais pas collés. */
.toggle-row.spaced { margin-top: 14px; }
.divider { height: 1px; background: var(--border); margin: 18px 0; }

.switch {
  flex: none; width: 42px; height: 24px; border-radius: 99px;
  background: var(--surface-3); border: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 2px; transition: background 0.15s ease;
}
.switch .knob {
  width: 18px; height: 18px; border-radius: 50%; background: var(--text-faint);
  transition: transform 0.15s ease, background 0.15s ease;
}
.switch.on { background: var(--accent); border-color: var(--accent); }
.switch.on .knob { transform: translateX(18px); background: var(--accent-ink); }

.segmented { display: inline-flex; background: var(--surface-2); border: 1px solid var(--border); border-radius: 10px; padding: 2px; gap: 2px; flex: none; }
.seg { padding: 6px 12px; border-radius: 8px; border: none; background: none; color: var(--text-dim); font-size: 12.5px; font-weight: 600; cursor: pointer; }
.seg:hover { color: var(--text); }
.seg.on { background: var(--accent); color: var(--accent-ink); }

/* Ligne de préférence empilée (plusieurs contrôles) */
.pref.wrap { flex-direction: column; align-items: stretch; gap: 12px; }
.controls { display: flex; flex-wrap: wrap; gap: 8px; }

/* À propos & maintenance */
.row-actions { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 12px; }
.sub-title { font-size: 14px; font-weight: 700; margin: 4px 0 var(--gap-desc); }
.presence-choice { flex-direction: column; align-items: stretch; gap: 12px; }
.modes { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 8px; }
.mode {
  display: flex; flex-direction: column; gap: 3px; text-align: left; cursor: pointer;
  padding: 11px 13px; border-radius: 11px;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim);
}
.mode:hover { border-color: var(--border-strong); color: var(--text); }
.mode.on { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }
.mode-label { font-size: 13.5px; font-weight: 600; }
.mode-hint { font-size: 11.5px; color: var(--text-faint); line-height: 1.4; }

.pseudo-input {
  padding: 8px 12px; border-radius: 9px; font-size: 13.5px; font-family: inherit; min-width: 190px;
  background: var(--bg); border: 1px solid var(--border); color: var(--text);
}
.pseudo-input:focus { border-color: var(--accent); }

.friend-code {
  font-family: var(--mono); font-size: 15px; letter-spacing: 0.16em; font-weight: 600;
  padding: 7px 13px; border-radius: 9px;
  background: var(--surface-2); border: 1px dashed var(--border-strong);
}
.version-tag {
  flex: none; font-family: var(--mono); font-size: 13px; color: var(--text-dim);
  background: var(--surface-2); border: 1px solid var(--border); padding: 4px 11px; border-radius: 8px;
}
.primary-btn {
  padding: 8px 15px; border-radius: 9px; border: none; background: var(--accent);
  color: var(--accent-ink); font-weight: 700; font-size: 12.5px; cursor: pointer;
}
.primary-btn:hover { background: var(--accent-hover); }
.cache-msg { font-size: 12px; color: var(--accent); margin: 10px 0 0; line-height: 1.4; }

/* Listes */
.items { display: flex; flex-direction: column; }
.item { display: flex; align-items: center; gap: 12px; padding: 10px 0; border-top: 1px solid var(--border); }
.item:first-child { border-top: none; }
.thumb {
  width: 40px; height: 40px; border-radius: 9px; flex: none; overflow: hidden;
  background: var(--surface-2); display: grid; place-items: center; color: var(--text-faint);
}
.thumb img { width: 100%; height: 100%; object-fit: cover; }
.thumb.store svg { width: 18px; height: 18px; }
.item-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: var(--gap-desc); }
.item-title { font-size: 14px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.item-sub { display: inline-flex; align-items: center; gap: 5px; font-size: 11.5px; color: var(--text-faint); }
.item-sub :deep(.platform-icon) { width: 13px; height: 13px; }
.ghost-btn {
  flex: none; padding: 7px 14px; border-radius: 9px; border: 1px solid var(--border);
  background: var(--surface-2); color: var(--text-dim); font-size: 12.5px; font-weight: 600; cursor: pointer;
}
.ghost-btn:hover { color: var(--text); border-color: var(--border-strong); }
.clear-all {
  align-self: flex-start; margin-top: 12px; background: none; border: none;
  color: var(--text-faint); font-size: 12px; font-family: var(--mono); cursor: pointer; padding: 2px 0;
}
.clear-all:hover { color: var(--accent); }
.empty { font-size: 13.5px; color: var(--text-faint); margin: 4px 0 0; }

/* Étiquette « cet appareil » : discrète, mais c'est elle qui évite de se déconnecter
   soi-même en croyant fermer une autre machine. */
.tag {
  margin-left: 8px; padding: 2px 7px; border-radius: 99px;
  font-size: 11px; font-weight: 600; letter-spacing: 0.01em;
  color: var(--accent); background: var(--accent-soft);
}
.confirm-lead { align-self: center; font-size: 12.5px; color: var(--text-dim); }

/* --- Langue et région --------------------------------------------------------- */

/* Invite à redémarrer après un changement de langue : sous la ligne concernée, pas en
   toast — elle doit rester visible tant que le redémarrage n'a pas eu lieu. */
.row-note {
  display: flex; align-items: center; gap: 12px; margin-top: 12px;
  padding: 9px 12px; border-radius: 9px; font-size: 12.5px; line-height: 1.45;
  color: var(--text-dim); background: var(--accent-soft);
}
.row-note span { flex: 1; }

.region { display: flex; flex-direction: column; align-items: flex-end; gap: 5px; flex: none; }
/* 🔑 Une liste déroulante et pas un sélecteur segmenté comme la langue : douze pays ne
   tiennent pas sur une ligne, et une rangée de drapeaux serait illisible. */
.region-select {
  padding: 7px 30px 7px 12px; border-radius: 9px; font-size: 12.5px; font-weight: 600;
  font-family: inherit; color: var(--text); cursor: pointer; min-width: 190px;
  background: var(--surface-2); border: 1px solid var(--border);
}
.region-select:hover { border-color: var(--border-strong); }
.region-select:focus { border-color: var(--accent); }
.region-devise { font-family: var(--mono); font-size: 11px; color: var(--text-faint); }

@media (max-width: 720px) {
  .dialog { flex-direction: column; height: 90vh; }
  .snav { width: 100%; flex-direction: row; flex-wrap: wrap; border-right: none; border-bottom: 1px solid var(--border); }
  .snav-title, .snav-group { width: 100%; }
}
</style>
