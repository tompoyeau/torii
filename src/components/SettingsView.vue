<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { usePreferences } from "../composables/usePreferences";
import { useStore } from "../composables/useStore";
import { useTheme } from "../composables/useTheme";
import { useUi } from "../composables/useUi";
import { useScrollLock } from "../composables/useScrollLock";
import { useUpdater } from "../composables/useUpdater";
import { platformName } from "../data/platforms";
import { appVersion, clearCaches, getAutostart, getSettings, getWindowPrefs, setAutostart, setWindowPrefs } from "../lib/tauri";
import PlatformIcon from "./PlatformIcon.vue";
import AccountsSettings from "./AccountsSettings.vue";
import ToriiPanel from "./ToriiPanel.vue";
import { useTorii } from "../composables/useTorii";
import { useFriends } from "../composables/useFriends";

const { games, setHidden } = useLibrary();
const { prefs } = usePreferences();
const { excludedStores, toggleStoreExcluded, clearExcludedStores } = useStore();
const {
  account: toriiAccount, connected: toriiConnected, prefs: toriiPrefs, mutedGames,
  suggestions, searching, searched,
  setPrefs: setToriiPrefs, setMuted, setSteamLink, logout: toriiLogout,
  findSteamFriends, inviteAccount,
} = useTorii();
const { friends: steamFriends, refresh: refreshSteamFriends } = useFriends();

/** Délais d'inactivité proposés avant de passer « absent ». */
const AWAY_DELAYS = [5, 10, 20, 30] as const;

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
  await setSteamLink(visible ? mySteamId.value : "", visible);
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

const CATEGORIES = [
  { key: "general", label: "Paramètres généraux", group: "Application" },
  { key: "about", label: "À propos & maintenance", group: "Application" },
  { key: "hidden", label: "Jeux masqués", group: "Bibliothèque & Boutique" },
  { key: "stores", label: "Revendeurs masqués", group: "Bibliothèque & Boutique" },
  { key: "accounts", label: "Comptes & launchers", group: "Comptes" },
  { key: "torii", label: "Réseau Torii", group: "Comptes" },
] as const;

// --- Choix pour les préférences (segmented) --------------------------------
const MODES = [
  { key: "bureau", label: "Bureau" },
  { key: "salon", label: "Salon" },
] as const;
const START_FILTERS = [
  { key: "all", label: "Tous" },
  { key: "favorite", label: "Favoris" },
  { key: "installed", label: "Installés" },
] as const;
const START_SORTS = [
  { key: "recent", label: "Récemment joué" },
  { key: "alpha", label: "A → Z" },
  { key: "playtime", label: "Temps de jeu" },
] as const;
const DENSITIES = [
  { key: "compact", label: "Compact" },
  { key: "normal", label: "Normal" },
  { key: "large", label: "Grand" },
] as const;

// --- À propos & maintenance -------------------------------------------------
useScrollLock(settingsOpen);

const version = ref<string | null>(null);
const cacheMsg = ref("");
const cacheBusy = ref(false);
onMounted(async () => {
  version.value = await appVersion();
});
const updateLabel = computed(() => {
  switch (updateStatus.value) {
    case "checking": return "Vérification…";
    case "available": return `Mise à jour disponible : ${updateVersion.value ?? ""}`;
    case "downloading": return "Téléchargement…";
    case "ready": return "Installée — redémarrage…";
    case "error": return "Erreur de vérification.";
    default: return "Torii est à jour.";
  }
});
async function onClearCache() {
  if (cacheBusy.value) return;
  cacheBusy.value = true;
  cacheMsg.value = "";
  const n = await clearCaches();
  cacheBusy.value = false;
  cacheMsg.value = n == null
    ? "Indisponible hors de l'application."
    : `Cache vidé (${n} fichier${n > 1 ? "s" : ""}). Les données seront re-téléchargées au besoin.`;
}
// Groupes ordonnés (pour les libellés de section du rail).
const groups = computed(() => {
  const seen: string[] = [];
  for (const c of CATEGORIES) if (!seen.includes(c.group)) seen.push(c.group);
  return seen.map((g) => ({ label: g, items: CATEGORIES.filter((c) => c.group === g) }));
});

// --- Lancement au démarrage de Windows -------------------------------------
const autostart = ref(false);
const autostartBusy = ref(false);
// Préférences de fenêtre (persistées côté Rust).
const startMinimized = ref(false);
const closeToTray = ref(false);
async function refreshSystemPrefs() {
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
const THEMES: { key: ThemeChoice; label: string }[] = [
  { key: "system", label: "Système" },
  { key: "light", label: "Clair" },
  { key: "dark", label: "Sombre" },
];

// --- Jeux masqués -----------------------------------------------------------
const hiddenGames = computed(() =>
  games.value.filter((g) => g.hidden).sort((a, b) => a.title.localeCompare(b.title, "fr")),
);
function unhide(id: string) {
  void setHidden(id, false);
}
</script>

<template>
  <div v-if="settingsOpen" class="overlay" @click.self="closeSettings">
    <div class="dialog">
      <!-- Rail de navigation -->
      <aside class="snav">
        <div class="snav-title">Paramètres</div>
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
        <button class="close" aria-label="Fermer" @click="closeSettings">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>

        <div class="spane-inner">
          <!-- Paramètres généraux -->
          <template v-if="settingsCategory === 'general'">
            <h2 class="pane-title">Paramètres généraux</h2>

            <button
              class="pref toggle-row"
              role="switch"
              :aria-checked="autostart"
              :disabled="autostartBusy"
              @click="onToggleAutostart"
            >
              <div class="row-text">
                <span class="row-title">Lancer au démarrage de Windows</span>
                <span class="row-sub">Torii s'ouvrira automatiquement à l'ouverture de ta session.</span>
              </div>
              <span class="switch" :class="{ on: autostart }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <button class="pref toggle-row" role="switch" :aria-checked="startMinimized" @click="toggleStartMinimized">
              <div class="row-text">
                <span class="row-title">Démarrer minimisé</span>
                <span class="row-sub">Se lance réduit dans la zone de notification (à côté de l'horloge).</span>
              </div>
              <span class="switch" :class="{ on: startMinimized }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <button class="pref toggle-row" role="switch" :aria-checked="closeToTray" @click="toggleCloseToTray">
              <div class="row-text">
                <span class="row-title">Fermer réduit dans la zone de notification</span>
                <span class="row-sub">
                  Activé par défaut : Torii continue de repérer tes parties une fois la
                  fenêtre fermée. Décoche pour que la croix quitte vraiment l'application —
                  « Quitter » reste disponible par clic droit sur l'icône près de l'horloge.
                </span>
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
                <span class="row-title">Revenir à la fermeture d'un jeu</span>
                <span class="row-sub">Torii se minimise au lancement d'un jeu, puis revient au premier plan sur sa fiche quand tu le fermes. (Jeux installés lancés depuis Torii.)</span>
              </div>
              <span class="switch" :class="{ on: prefs.returnOnGameExit }"><span class="knob" /></span>
            </button>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">Thème</span>
                <span class="row-sub">Apparence de l'application.</span>
              </div>
              <div class="segmented">
                <button
                  v-for="t in THEMES"
                  :key="t.key"
                  class="seg"
                  :class="{ on: themeChoice === t.key }"
                  @click="pickTheme(t.key)"
                >
                  {{ t.label }}
                </button>
              </div>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">Mode au démarrage</span>
                <span class="row-sub">Interface ouverte au lancement de Torii.</span>
              </div>
              <div class="segmented">
                <button v-for="m in MODES" :key="m.key" class="seg" :class="{ on: prefs.defaultMode === m.key }" @click="prefs.defaultMode = m.key">{{ m.label }}</button>
              </div>
            </div>

            <div class="divider" />

            <div class="pref wrap">
              <div class="row-text">
                <span class="row-title">Vue par défaut</span>
                <span class="row-sub">Filtre, tri et affichage au démarrage de la bibliothèque.</span>
              </div>
              <div class="controls">
                <div class="segmented">
                  <button v-for="f in START_FILTERS" :key="f.key" class="seg" :class="{ on: prefs.defaultFilter === f.key }" @click="prefs.defaultFilter = f.key">{{ f.label }}</button>
                </div>
                <div class="segmented">
                  <button v-for="s in START_SORTS" :key="s.key" class="seg" :class="{ on: prefs.defaultSort === s.key }" @click="prefs.defaultSort = s.key">{{ s.label }}</button>
                </div>
                <div class="segmented">
                  <button class="seg" :class="{ on: !prefs.listView }" @click="prefs.listView = false">Grille</button>
                  <button class="seg" :class="{ on: prefs.listView }" @click="prefs.listView = true">Liste</button>
                </div>
              </div>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">Densité de la bibliothèque</span>
                <span class="row-sub">Taille des jaquettes dans la grille.</span>
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
                <span class="row-title">Réduire les animations</span>
                <span class="row-sub">Désactive les transitions et effets (accessibilité, machines modestes).</span>
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
                <span class="row-title">Alertes de prix (wishlist)</span>
                <span class="row-sub">Une notification quand un jeu de ta wishlist Steam passe en promo ou atteint son plus bas prix historique. (Torii doit tourner, même réduit dans le tray.)</span>
              </div>
              <span class="switch" :class="{ on: prefs.wishlistNotifications }"><span class="knob" /></span>
            </button>
          </template>

          <!-- À propos & maintenance -->
          <template v-else-if="settingsCategory === 'about'">
            <h2 class="pane-title">À propos &amp; maintenance</h2>

            <div class="pref">
              <div class="row-text">
                <span class="row-title">Version de Torii</span>
                <span class="row-sub">{{ updateLabel }}</span>
              </div>
              <span class="version-tag">{{ version ?? "—" }}</span>
            </div>

            <div class="row-actions">
              <button class="ghost-btn" :disabled="updateStatus === 'checking' || updateStatus === 'downloading'" @click="checkUpdate(false)">
                Vérifier les mises à jour
              </button>
              <button v-if="updateStatus === 'available'" class="primary-btn sm" @click="installUpdate()">
                Installer maintenant
              </button>
            </div>

            <div class="divider" />

            <div class="pref">
              <div class="row-text">
                <span class="row-title">Vider le cache</span>
                <span class="row-sub">Supprime les métadonnées, jaquettes et prix mis en cache (re-téléchargés au besoin). N'affecte ni tes comptes ni tes favoris.</span>
              </div>
              <button class="ghost-btn" :disabled="cacheBusy" @click="onClearCache">
                {{ cacheBusy ? "Nettoyage…" : "Vider le cache" }}
              </button>
            </div>
            <p v-if="cacheMsg" class="cache-msg">{{ cacheMsg }}</p>
          </template>

          <!-- Jeux masqués -->
          <template v-else-if="settingsCategory === 'hidden'">
            <h2 class="pane-title">Jeux masqués</h2>
            <p class="pane-hint">Les jeux masqués sont retirés de la bibliothèque. Réaffiche-les ici.</p>
            <div v-if="hiddenGames.length" class="items">
              <div v-for="g in hiddenGames" :key="g.id" class="item">
                <div class="thumb" :style="{ background: g.cover }">
                  <img v-if="g.coverUrl" :src="g.coverUrl" alt="" loading="lazy" @error="($event.target as HTMLElement).style.display='none'" />
                </div>
                <div class="item-text">
                  <span class="item-title">{{ g.title }}</span>
                  <span class="item-sub"><PlatformIcon :platform="g.platform" /> {{ platformName(g.platform) }}</span>
                </div>
                <button class="ghost-btn" @click="unhide(g.id)">Réafficher</button>
              </div>
            </div>
            <p v-else class="empty">Aucun jeu masqué.</p>
          </template>

          <!-- Revendeurs masqués -->
          <template v-else-if="settingsCategory === 'stores'">
            <h2 class="pane-title">Revendeurs masqués</h2>
            <p class="pane-hint">Boutiques masquées dans le comparatif de prix de la Boutique.</p>
            <div v-if="excludedStores.length" class="items">
              <div v-for="name in excludedStores" :key="name" class="item">
                <div class="thumb store"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 8h16l-1 4a3 3 0 0 1-3 2.4H8A3 3 0 0 1 5 12Z" /><path d="M4 8l1.4-3.4A2 2 0 0 1 7.2 3.4h9.6a2 2 0 0 1 1.8 1.2L20 8" /></svg></div>
                <div class="item-text"><span class="item-title">{{ name }}</span></div>
                <button class="ghost-btn" @click="toggleStoreExcluded(name)">Réafficher</button>
              </div>
              <button class="clear-all" @click="clearExcludedStores">Tout réafficher</button>
            </div>
            <p v-else class="empty">
              Aucun revendeur masqué. Tu peux en masquer depuis la fiche d'un jeu dans la Boutique.
            </p>
          </template>

          <!-- Réseau Torii -->
          <template v-else-if="settingsCategory === 'torii'">
            <h2 class="pane-title">Réseau Torii</h2>
            <p class="pane-hint">
              Voir à quoi jouent tes amis, quel que soit leur launcher — et leur montrer ce
              que tu joues, si tu le décides.
            </p>

            <ToriiPanel />

            <template v-if="toriiConnected">
              <button
                class="pref toggle-row"
                role="switch"
                :aria-checked="toriiPrefs.sharePresence"
                @click="setToriiPrefs({ sharePresence: !toriiPrefs.sharePresence })"
              >
                <div class="row-text">
                  <span class="row-title">Partager ma présence</span>
                  <span class="row-sub">
                    Tes amis voient à quoi tu joues et depuis quand. Tant que c'est coupé,
                    rien ne quitte ton PC.
                  </span>
                </div>
                <span class="switch" :class="{ on: toriiPrefs.sharePresence }"><span class="knob" /></span>
              </button>

              <div class="divider" />

              <div class="pref">
                <div class="row-text">
                  <span class="row-title">Passer « absent » après</span>
                  <span class="row-sub">Sans action au clavier ni à la souris.</span>
                </div>
                <div class="segmented">
                  <button
                    v-for="d in AWAY_DELAYS"
                    :key="d"
                    class="seg"
                    :class="{ on: toriiPrefs.awayAfterMinutes === d }"
                    @click="setToriiPrefs({ awayAfterMinutes: d })"
                  >
                    {{ d }} min
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
                  <span class="row-title">Visible par mes amis Steam</span>
                  <span class="row-sub">
                    <template v-if="canLinkSteam">
                      Permet à tes amis Steam déjà sur Torii de te retrouver, et de fusionner
                      ta fiche avec ton profil Steam. Il faut que vous l'ayez activé tous les deux.
                    </template>
                    <template v-else>
                      Connecte d'abord ton compte Steam dans « Comptes &amp; launchers » :
                      sans lui, il n'y a rien à rapprocher.
                    </template>
                  </span>
                </div>
                <span class="switch" :class="{ on: toriiAccount?.steamDiscoverable }"><span class="knob" /></span>
              </button>

              <template v-if="toriiAccount?.steamDiscoverable">
                <div class="divider" />
                <h3 class="sub-title">Retrouver mes amis Steam</h3>
                <p class="pane-hint">
                  Torii compare ta liste d'amis Steam aux comptes existants. Seuls ceux qui
                  ont eux aussi activé cette option apparaissent — c'est ce qui empêche de
                  s'en servir pour savoir qui utilise Torii.
                </p>
                <div class="row-actions">
                  <button class="ghost-btn" :disabled="searching" @click="onFindSteamFriends">
                    {{ searching ? "Recherche…" : "Chercher parmi mes amis Steam" }}
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
                        {{ steamNameOf(p.steamId) }} sur Steam
                      </span>
                    </div>
                    <button class="ghost-btn" @click="inviteAccount(p.id)">Ajouter</button>
                  </div>
                </div>
                <p v-else-if="searched && !searching" class="empty">
                  Aucun de tes amis Steam n'a de compte Torii visible pour l'instant.
                </p>
              </template>

              <div class="divider" />

              <h3 class="sub-title">Jeux jamais diffusés</h3>
              <p class="pane-hint">
                Ces jeux n'apparaissent jamais dans ta présence, même en cours de partie.
                Utile pour les applications qui tournent en permanence.
              </p>
              <div v-if="mutedList.length" class="items">
                <div v-for="g in mutedList" :key="g.id" class="item">
                  <div class="thumb"><PlatformIcon :platform="g.platform" /></div>
                  <div class="item-text"><span class="item-title">{{ g.title }}</span></div>
                  <button class="ghost-btn" @click="setMuted(g.id, false)">Diffuser à nouveau</button>
                </div>
              </div>
              <p v-else class="empty">
                Aucun jeu masqué. Fais un clic droit sur un jeu pour l'ajouter.
              </p>

              <div class="row-actions">
                <button class="ghost-btn" @click="toriiLogout()">Déconnecter ce compte</button>
              </div>
            </template>
          </template>

          <!-- Comptes & launchers -->
          <template v-else>
            <h2 class="pane-title">Comptes &amp; launchers</h2>
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
.pane-title { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0 0 20px; }
.pane-hint { font-size: 12.5px; color: var(--text-dim); line-height: 1.5; margin: -10px 0 20px; }

/* Préférences */
.pref { display: flex; align-items: center; gap: 16px; }
.row-text { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.row-title { font-weight: 600; font-size: 14px; color: var(--text); }
.row-sub { font-size: 12px; color: var(--text-dim); line-height: 1.4; }
.toggle-row { width: 100%; text-align: left; background: none; border: none; padding: 0; cursor: pointer; }
.toggle-row:disabled { cursor: default; opacity: 0.6; }
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
.sub-title { font-size: 14px; font-weight: 700; margin: 4px 0 6px; }
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
.item-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
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

@media (max-width: 720px) {
  .dialog { flex-direction: column; height: 90vh; }
  .snav { width: 100%; flex-direction: row; flex-wrap: wrap; border-right: none; border-bottom: 1px solid var(--border); }
  .snav-title, .snav-group { width: 100%; }
}
</style>
