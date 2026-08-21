<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useFriendsCommon } from "../composables/useFriendsCommon";
import { useStore } from "../composables/useStore";
import { useUi } from "../composables/useUi";
import { useTorii } from "../composables/useTorii";
import { useScrollLock } from "../composables/useScrollLock";
import { platformName } from "../data/platforms";
import { installSource, launchSource, openExternal, openInstallDir, steamAchievements, steamCurrentPlayers, uninstallGame } from "../lib/tauri";
import type { FriendLib, GameSource, SteamAchievements } from "../types";
import PlatformIcon from "./PlatformIcon.vue";

const { byId, ensureEnriched, enrichingId, setFavorite, markPlayed, launchOrInstall, removeManual } = useLibrary();
const { friends, ensureLoaded, ownersOf } = useFriendsCommon();
const { openForTitle } = useStore();
const { connected: toriiConnected, isMuted, setMuted } = useTorii();
const { selectedGameId, closeGame, showStore, openEditGame } = useUi();

/**
 * Ouvre le comparatif de prix de ce jeu **par-dessus** sa fiche, sans quitter la
 * bibliothèque : refermer la fiche prix ramène donc exactement là où on était.
 * (Avant, on basculait sur la section Boutique et le retour laissait l'utilisateur
 * sur la vitrine, loin de son jeu.)
 * Repli : si le jeu n'est pas trouvé tel quel, `openForTitle` bascule en recherche —
 * là il FAUT montrer la Boutique, puisque les résultats sont dans la vue de section.
 */
async function viewInStore() {
  const g = game.value;
  if (!g) return;
  const opened = await openForTitle(g.title);
  if (!opened) {
    closeGame();
    showStore();
  }
}

const game = computed(() => byId(selectedGameId.value));

// Fiche ouverte = la bibliothèque en dessous ne doit plus défiler (ni montrer sa barre).
useScrollLock(computed(() => !!game.value));

const friendById = computed(() => new Map(friends.value.map((f) => [f.steamId, f])));

/**
 * Amis qui possèdent aussi ce jeu : ceux dont on a lu la bibliothèque (via Steam),
 * plus les membres de la famille qui sont des amis (utile quand leur biblio est privée,
 * la donnée famille connaît la possession quand même). Déboublonné par SteamID.
 */
const friendOwners = computed<FriendLib[]>(() => {
  if (!game.value) return [];
  const map = new Map<string, FriendLib>();
  for (const f of ownersOf(game.value)) map.set(f.steamId, f);
  for (const sid of game.value.familyOwners ?? []) {
    const f = friendById.value.get(sid);
    if (f) map.set(sid, f);
  }
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name, "fr"));
});

/** Nombre de copies du jeu dans la famille Steam. */
const familyCopies = computed(() => game.value?.familyOwners?.length ?? 0);
/** Montrer l'info famille : jeu partagé, ou plusieurs copies détenues. */
const showFamily = computed(
  () => familyCopies.value >= 1 && (game.value?.familyShared || familyCopies.value >= 2),
);

// Charge (une fois, cache 6 h) les bibliothèques d'amis pour la section « qui l'a aussi ».
watch(
  selectedGameId,
  (id) => {
    if (id) void ensureLoaded();
  },
  { immediate: true },
);

const avatarFailed = ref(new Set<string>());
function ownerAvatar(f: FriendLib): string | null {
  return f.avatarUrl && !avatarFailed.value.has(f.avatarUrl) ? f.avatarUrl : null;
}
function onOwnerAvatarError(url: string) {
  avatarFailed.value = new Set(avatarFailed.value).add(url);
}
function ownerInitials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}
function openFriendProfile(f: FriendLib) {
  openExternal(`https://steamcommunity.com/profiles/${f.steamId}`);
}

/** Provenances jouables (jeu multi-plateforme) ; vide si mono-source. */
const sources = computed(() => game.value?.sources ?? []);
const multiSource = computed(() => sources.value.length > 1);
const launchMenuOpen = ref(false);

/** Bouton principal : lance (installé) ou installe (possédé non installé) ; ouvre le
 * choix si multi-source. */
function onPlay() {
  if (multiSource.value) {
    launchMenuOpen.value = !launchMenuOpen.value;
    uninstallMenuOpen.value = false; // les deux menus s'excluent
    return;
  }
  if (game.value) launchOrInstall(game.value);
}

/** Bascule le menu d'options (engrenage) ; ferme le menu « Jouer » s'il était ouvert. */
function toggleSettingsMenu() {
  uninstallMenuOpen.value = !uninstallMenuOpen.value;
  launchMenuOpen.value = false;
}
/** Action sur une provenance du menu multi-source : lancer si installée, sinon installer. */
function playFrom(s: GameSource) {
  if (!game.value) return;
  if (s.installed) {
    markPlayed(game.value.id);
    launchSource(s.platform, s.launchTarget);
  } else {
    installSource(s.platform, s.launchTarget);
  }
  launchMenuOpen.value = false;
}

/** Épingle/retire le jeu affiché des favoris. */
function toggleFavorite() {
  if (game.value) setFavorite(game.value.id, !game.value.favorite);
}

// Menu d'options (engrenage) : ouvert seulement pour un jeu installé.
const uninstallMenuOpen = ref(false);
const uninstalling = ref(false);

/** Ouvre l'explorateur au dossier d'installation du jeu (si connu). */
function onOpenFolder() {
  if (game.value?.installDir) openInstallDir(game.value.installDir);
  uninstallMenuOpen.value = false;
}
/** Ce jeu est-il tenu à l'écart de ce que voient les amis ? */
const gameMuted = computed(() => !!game.value && isMuted(game.value.id));

/**
 * Empêche (ou rétablit) la diffusion de ce jeu aux amis. Sa place est ici autant que
 * dans le clic droit : c'est sur la fiche qu'on se dit « celui-là, je le garde pour moi ».
 */
async function onToggleMuted() {
  const g = game.value;
  if (!g) return;
  await setMuted(g.id, !gameMuted.value);
  uninstallMenuOpen.value = false;
}

/** Un jeu manuel n'est référencé que par Torii : « désinstaller » = le retirer d'ici. */
const isManual = computed(() => game.value?.platform === "manual");

/** Édite les informations saisies à la main (titre, exécutable, dossier, jaquette). */
function onEdit() {
  if (game.value) openEditGame(game.value.id);
  uninstallMenuOpen.value = false;
}

/** Retire un jeu manuel de la bibliothèque et referme sa fiche. */
async function onRemoveManual() {
  const g = game.value;
  if (!g) return;
  uninstalling.value = true;
  try {
    await removeManual(g.id);
    uninstallMenuOpen.value = false;
    closeGame();
  } finally {
    uninstalling.value = false;
  }
}

async function onUninstall() {
  if (!game.value) return;
  uninstalling.value = true;
  try {
    await uninstallGame(game.value);
    // Le launcher prend le relais (confirmation + suppression). On referme le menu ;
    // l'état « installé » se met à jour à la prochaine resynchronisation.
    uninstallMenuOpen.value = false;
  } finally {
    uninstalling.value = false;
  }
}

/** Charge les métadonnées (description, captures…) à l'ouverture d'une fiche. */
watch(
  selectedGameId,
  (id) => {
    if (id) ensureEnriched(id);
  },
  { immediate: true },
);

/** Métadonnées en cours de récupération pour la fiche affichée. */
const loadingMeta = computed(
  () => !!game.value && enrichingId.value === game.value.id,
);

/** Vraies captures si disponibles, sinon des panneaux dérivés du dégradé. */
const realShots = computed(() => game.value?.screenshots ?? []);
const fallbackShots = computed(() =>
  game.value
    ? [0, 1, 2, 3].map((k) => ({
        background: game.value!.cover,
        filter: `hue-rotate(${k * 22}deg) saturate(${1 + k * 0.12})`,
      }))
    : [],
);

function hideBrokenCover(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}

/* ── Défilement horizontal de la bande de captures ───────────────────────── */
const shotsRow = ref<HTMLElement | null>(null);
const shotsAtStart = ref(true);
const shotsAtEnd = ref(true);

/** Met à jour la visibilité des flèches selon la position de défilement. */
function updateShotsScroll() {
  const el = shotsRow.value;
  if (!el) return;
  shotsAtStart.value = el.scrollLeft <= 2;
  shotsAtEnd.value = el.scrollLeft + el.clientWidth >= el.scrollWidth - 2;
}
/** Défile d'environ un écran de captures (sens −1 = gauche, +1 = droite). */
function scrollShots(dir: number) {
  const el = shotsRow.value;
  if (el) el.scrollBy({ left: dir * el.clientWidth * 0.85, behavior: "smooth" });
}
// Recalcule (et remet à gauche) quand le jeu ou les captures changent.
watch([selectedGameId, realShots], () => {
  nextTick(() => {
    if (shotsRow.value) shotsRow.value.scrollLeft = 0;
    updateShotsScroll();
  });
});
onMounted(() => {
  nextTick(updateShotsScroll);
  window.addEventListener("resize", updateShotsScroll);
});

/** Visionneuse plein écran des captures (index dans `realShots`, sinon fermée). */
const zoomIndex = ref<number | null>(null);
const zoomedShot = computed(() =>
  zoomIndex.value != null ? realShots.value[zoomIndex.value] ?? null : null,
);
function openShot(i: number) {
  zoomIndex.value = i;
}
function closeShot() {
  zoomIndex.value = null;
}
function stepShot(delta: number) {
  const n = realShots.value.length;
  if (zoomIndex.value == null || n === 0) return;
  zoomIndex.value = (zoomIndex.value + delta + n) % n;
}

// Réinitialise la visionneuse et le menu de lancement quand on change de jeu.
watch(selectedGameId, () => {
  zoomIndex.value = null;
  launchMenuOpen.value = false;
  uninstallMenuOpen.value = false;
});

/**
 * Appid Steam du jeu affiché (jeu Steam direct, ou source Steam d'un jeu fusionné
 * multi-plateforme). `null` si aucune provenance Steam → pas de succès à récupérer.
 */
const steamAppid = computed<string | null>(() => {
  const g = game.value;
  if (!g) return null;
  if (g.platform === "steam") {
    return g.launchTarget ?? (g.id.startsWith("steam:") ? g.id.slice(6) : null);
  }
  return g.sources?.find((s) => s.platform === "steam")?.launchTarget ?? null;
});

/** Vrais succès Steam (nom, icône, date de déblocage) récupérés à l'ouverture de la fiche. */
const steamAch = ref<SteamAchievements | null>(null);
const loadingAch = ref(false);
const showAllAch = ref(false);

/** Joueurs en ce moment sur le jeu Steam (API publique), affiché dans la stat-card. */
const currentPlayers = ref<number | null>(null);
const playersLabel = computed(() =>
  currentPlayers.value != null ? currentPlayers.value.toLocaleString("fr-FR") : null,
);

/** Succès affichés : débloqués d'abord (tri backend), limités sauf « tout afficher ». */
const ACH_PREVIEW = 8;
const visibleAch = computed(() =>
  showAllAch.value ? steamAch.value?.items ?? [] : (steamAch.value?.items ?? []).slice(0, ACH_PREVIEW),
);
const achPct = computed(() => {
  const a = steamAch.value;
  return a && a.total > 0 ? Math.round((a.unlocked / a.total) * 100) : 0;
});

// Charge les succès Steam à l'ouverture d'une fiche (anti-course par id).
watch(
  selectedGameId,
  async (id) => {
    steamAch.value = null;
    showAllAch.value = false;
    loadingAch.value = false;
    currentPlayers.value = null;
    const appid = steamAppid.value;
    if (!id || !appid) return;
    loadingAch.value = true;
    // Joueurs en ce moment (rapide, public) en parallèle des succès (anti-course par id).
    void steamCurrentPlayers(appid).then((n) => {
      if (selectedGameId.value === id) currentPlayers.value = n;
    });
    const res = await steamAchievements(appid).catch(() => null);
    if (selectedGameId.value === id) steamAch.value = res;
    loadingAch.value = false;
  },
  { immediate: true },
);

function onKey(e: KeyboardEvent) {
  if (zoomIndex.value != null) {
    // La visionneuse capte les touches avant la fiche.
    if (e.key === "Escape") closeShot();
    else if (e.key === "ArrowRight") stepShot(1);
    else if (e.key === "ArrowLeft") stepShot(-1);
    return;
  }
  if (e.key === "Escape") closeGame();
}
onMounted(() => document.addEventListener("keydown", onKey));
onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKey);
  window.removeEventListener("resize", updateShotsScroll);
});
</script>

<template>
  <div class="detail" :class="{ open: !!game }">
    <template v-if="game">
      <div class="detail-banner">
        <div class="detail-banner-art" :style="{ background: game.cover }" />
        <img v-if="game.heroUrl" class="detail-banner-img" :src="game.heroUrl" alt="" @error="hideBrokenCover" />
        <div class="detail-banner-scrim" />
        <button class="detail-back" @click="closeGame">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>Bibliothèque
        </button>
        <div class="detail-header">
          <div class="detail-title-wrap">
            <span class="detail-plat"><PlatformIcon :platform="game.platform" />{{ platformName(game.platform) }}</span>
            <h1 class="detail-title">{{ game.title }}</h1>
          </div>
          <div class="detail-actions">
            <div class="play-wrap">
              <button class="btn-play big" @click.stop="onPlay">
                <svg v-if="game.installed" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
                <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v12m0 0l-4.5-4.5M12 15l4.5-4.5M5 21h14" /></svg>
                {{ game.installed ? "Jouer" : "Installer" }}
                <svg v-if="multiSource" class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
              </button>
              <div v-if="multiSource && launchMenuOpen" class="launch-menu" @click.stop>
                <div class="launch-menu-label">{{ game.installed ? "Jouer depuis…" : "Installer depuis…" }}</div>
                <button
                  v-for="s in sources"
                  :key="s.platform + (s.launchTarget ?? '')"
                  class="launch-opt"
                  @click="playFrom(s)"
                >
                  <PlatformIcon :platform="s.platform" />
                  <span class="launch-opt-name">{{ platformName(s.platform) }}</span>
                  <span class="launch-opt-tag" :class="s.installed ? 'on' : ''">{{ s.installed ? "Installé" : "Non installé" }}</span>
                </button>
              </div>
            </div>
            <button
              class="btn-ghost solid"
              :class="{ 'fav-on': game.favorite }"
              :title="game.favorite ? 'Retirer des favoris' : 'Ajouter aux favoris'"
              :aria-pressed="game.favorite"
              @click.stop="toggleFavorite"
            >
              <svg viewBox="0 0 24 24" :fill="game.favorite ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 4.5l2.3 4.7 5.2.8-3.8 3.7.9 5.1L12 16.9l-4.6 2.4.9-5.1L4.5 10l5.2-.8z" /></svg>
            </button>
            <button class="btn-ghost solid" title="Voir dans la boutique" aria-label="Voir dans la boutique" @click.stop="viewInStore">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20.5 13.3 12.8 21a1.5 1.5 0 0 1-2.1 0l-7-7a1.4 1.4 0 0 1-.4-1V4.6A1.5 1.5 0 0 1 4.6 3h8.4a1.4 1.4 0 0 1 1 .4l6.5 6.5a2 2 0 0 1 0 2.8Z" /><circle cx="7.8" cy="7.8" r="1.4" fill="currentColor" stroke="none" /></svg>
            </button>
            <div v-if="game.installed" class="settings-wrap">
              <button
                class="btn-ghost solid"
                title="Options du jeu"
                :aria-expanded="uninstallMenuOpen"
                @click.stop="toggleSettingsMenu"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3" /><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.3 1a7 7 0 0 0-1.7-1L14.5 2h-5l-.4 2.5a7 7 0 0 0-1.7 1l-2.3-1-2 3.5 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.5 2.3-1a7 7 0 0 0 1.7 1l.4 2.5h5l.4-2.5a7 7 0 0 0 1.7-1l2.3 1 2-3.5-2-1.5a7 7 0 0 0 .1-1Z" /></svg>
              </button>
              <div v-if="uninstallMenuOpen" class="settings-menu" @click.stop>
                <div class="settings-menu-label">Options</div>
                <button v-if="game.installDir" class="settings-opt" @click="onOpenFolder">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" /></svg>
                  <span>Ouvrir l'emplacement du fichier</span>
                </button>
                <button v-if="toriiConnected" class="settings-opt" @click="onToggleMuted">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 3l18 18" /><path d="M10.6 10.7a2 2 0 0 0 2.8 2.8" /><path d="M9.4 5.2A9.3 9.3 0 0 1 12 5c5 0 9 4.5 9 7a12 12 0 0 1-2.2 3M6.1 6.2A12.7 12.7 0 0 0 3 12c0 2.5 4 7 9 7a9.4 9.4 0 0 0 3.6-.7" /></svg>
                  <span>{{ gameMuted ? "Diffuser ce jeu aux amis" : "Ne pas diffuser ce jeu" }}</span>
                </button>
                <button v-if="isManual" class="settings-opt" @click="onEdit">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 20h4L19 9a2.1 2.1 0 0 0-3-3L5 17Z" /><path d="M14.5 7.5 16.5 9.5" /></svg>
                  <span>Modifier les informations</span>
                </button>
                <button
                  class="settings-opt danger"
                  :disabled="uninstalling"
                  @click="isManual ? onRemoveManual() : onUninstall()"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m2 0v14a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6M10 11v6M14 11v6" /></svg>
                  <span>
                    {{ uninstalling
                      ? (isManual ? "Retrait…" : "Désinstallation…")
                      : (isManual ? "Retirer de la bibliothèque" : "Désinstaller") }}
                  </span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="detail-body">
        <div>
          <div class="detail-section">
            <h4>À propos</h4>
            <p v-if="game.description" class="detail-desc">{{ game.description }}</p>
            <p v-else-if="loadingMeta" class="detail-desc dim">Chargement des détails…</p>
            <p v-else class="detail-desc dim">Aucune description disponible pour ce jeu.</p>
          </div>
          <div v-if="friendOwners.length" class="detail-section">
            <h4>Amis qui possèdent ce jeu <span class="sec-count">{{ friendOwners.length }}</span></h4>
            <div class="owners-list">
              <button
                v-for="f in friendOwners"
                :key="f.steamId"
                class="owner"
                :title="`Voir le profil de ${f.name}`"
                @click="openFriendProfile(f)"
              >
                <span class="owner-av">
                  <img v-if="ownerAvatar(f)" :src="ownerAvatar(f)!" alt="" loading="lazy" @error="onOwnerAvatarError(f.avatarUrl)" />
                  <span v-else class="owner-av-fb">{{ ownerInitials(f.name) }}</span>
                </span>
                <span class="owner-name">{{ f.name }}</span>
              </button>
            </div>
          </div>
          <div class="detail-section">
            <h4>Captures d'écran</h4>
            <div class="shots-wrap">
              <button
                v-if="!shotsAtStart"
                class="shots-nav prev"
                type="button"
                aria-label="Captures précédentes"
                @click="scrollShots(-1)"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
              </button>
              <div ref="shotsRow" class="shots no-scrollbar" @scroll="updateShotsScroll">
                <template v-if="realShots.length">
                  <button
                    v-for="(s, i) in realShots"
                    :key="'r' + i"
                    class="shot shot-btn"
                    type="button"
                    aria-label="Agrandir la capture"
                    @click="openShot(i)"
                  >
                    <img :src="s" alt="" loading="lazy" @error="hideBrokenCover" />
                  </button>
                </template>
                <template v-else>
                  <div v-for="(s, i) in fallbackShots" :key="'f' + i" class="shot" :style="s" />
                </template>
              </div>
              <button
                v-if="!shotsAtEnd"
                class="shots-nav next"
                type="button"
                aria-label="Captures suivantes"
                @click="scrollShots(1)"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
              </button>
            </div>
          </div>
          <div v-if="loadingAch || (steamAch && steamAch.items.length)" class="detail-section">
            <h4>
              Succès
              <span v-if="steamAch" class="sec-count">{{ steamAch.unlocked }} / {{ steamAch.total }}</span>
            </h4>
            <p v-if="loadingAch && !steamAch" class="detail-desc dim">Chargement des succès…</p>
            <template v-else-if="steamAch">
              <div class="ach-list">
                <div v-for="a in visibleAch" :key="a.name" class="ach" :class="{ locked: !a.unlocked }">
                  <div class="ach-badge">
                    <img :src="a.icon" alt="" loading="lazy" @error="hideBrokenCover" />
                  </div>
                  <div class="ach-info">
                    <div class="ach-name">{{ a.name }}</div>
                    <div v-if="a.description" class="ach-desc">{{ a.description }}</div>
                    <div class="ach-pct">{{ a.unlocked ? (a.unlockedAt ?? "Débloqué") : "Verrouillé" }}</div>
                  </div>
                </div>
              </div>
              <button
                v-if="steamAch.items.length > ACH_PREVIEW"
                class="ach-toggle"
                @click="showAllAch = !showAllAch"
              >
                {{ showAllAch ? "Réduire" : `Afficher tout (${steamAch.items.length})` }}
              </button>
            </template>
          </div>
        </div>

        <aside class="stat-card">
          <template v-if="game.hoursPlayed != null">
            <div class="stat-play">{{ game.hoursPlayed }}<span> h</span></div>
            <div class="stat-label">de temps de jeu</div>
          </template>
          <template v-else-if="game.sizeGb">
            <div class="stat-play">{{ game.sizeGb }}<span> Go</span></div>
            <div class="stat-label">{{ game.installed ? "sur le disque" : "taille du jeu" }}</div>
          </template>
          <template v-else>
            <div class="stat-play">—</div>
            <div class="stat-label">aucune statistique</div>
          </template>
          <div class="stat-rows">
            <div class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 5a2 2 0 0 1 2-2h9l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z" /></svg>Statut</span><span class="v">{{ game.installed ? "Installé" : "Non installé" }}</span></div>
            <div class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>Dernière session</span><span class="v">{{ game.lastPlayed ?? "—" }}</span></div>
            <div v-if="playersLabel" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="8" r="3" /><path d="M3.5 19a5.5 5.5 0 0 1 11 0" /><path d="M16 6a3 3 0 0 1 0 5.6M17.5 19a5.5 5.5 0 0 0-2.5-4.3" /></svg>En ce moment</span><span class="v">{{ playersLabel }}</span></div>
            <div v-if="game.developer" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h10" /></svg>Développeur</span><span class="v">{{ game.developer }}</span></div>
            <div v-if="game.year" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" /><path d="M8 2v4M16 2v4M3 10h18" /></svg>Sortie</span><span class="v">{{ game.year }}</span></div>
            <div v-if="game.genre" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h18M3 6h18M3 18h18" /></svg>Genre</span><span class="v">{{ game.genre }}</span></div>
            <div class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 5a2 2 0 0 1 2-2h9l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z" /></svg>Taille</span><span class="v">{{ game.sizeGb ? game.sizeGb + " Go" : "—" }}</span></div>
            <div v-if="showFamily" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="8" r="3" /><path d="M3.5 19a5.5 5.5 0 0 1 11 0" /><path d="M16 6a3 3 0 0 1 0 5.6M17.5 19a5.5 5.5 0 0 0-2.5-4.3" /></svg>Famille Steam</span><span class="v">{{ familyCopies }} copie{{ familyCopies > 1 ? "s" : "" }}</span></div>
          </div>
          <div v-if="steamAch && steamAch.total" class="ach-progress">
            <div class="top"><span>Succès</span><span><b>{{ steamAch.unlocked }} / {{ steamAch.total }} ({{ achPct }}%)</b></span></div>
            <div class="ach-bar"><div class="ach-fill" :style="{ width: `${achPct}%` }" /></div>
          </div>
        </aside>
      </div>

      <div v-if="zoomedShot" class="lightbox" @click.self="closeShot">
        <button class="lb-close" aria-label="Fermer" @click="closeShot">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
        <button
          v-if="realShots.length > 1"
          class="lb-nav prev"
          aria-label="Capture précédente"
          @click.stop="stepShot(-1)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
        </button>
        <img class="lb-img" :src="zoomedShot" alt="" @error="hideBrokenCover" />
        <button
          v-if="realShots.length > 1"
          class="lb-nav next"
          aria-label="Capture suivante"
          @click.stop="stepShot(1)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
        </button>
        <div v-if="realShots.length > 1" class="lb-count">{{ (zoomIndex ?? 0) + 1 }} / {{ realShots.length }}</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.detail {
  position: fixed; inset: 0; z-index: 100; background: var(--bg); overflow-y: auto;
  opacity: 0; visibility: hidden; transition: opacity 0.28s;
}
.detail.open { opacity: 1; visibility: visible; }
.detail-banner { position: relative; height: 46vh; min-height: 340px; overflow: hidden; }
.detail-banner-art { position: absolute; inset: 0; }
.detail-banner-img {
  position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center 22%; display: block;
}
.detail-banner-art::after {
  content: ""; position: absolute; inset: 0;
  background: repeating-linear-gradient(115deg, rgba(255, 255, 255, 0.04) 0 2px, transparent 2px 11px);
  mix-blend-mode: overlay;
}
.detail-banner-scrim {
  position: absolute; inset: 0;
  background: linear-gradient(0deg, var(--bg) 1%, rgba(10, 7, 16, 0.2) 45%, rgba(10, 7, 16, 0.5) 100%);
}
:root[data-theme="light"] .detail-banner-scrim {
  background: linear-gradient(0deg, var(--bg) 1%, rgba(255, 255, 255, 0.1) 50%, transparent 100%);
}
.detail-back {
  position: absolute; top: 22px; left: 24px; z-index: 5; display: inline-flex; align-items: center; gap: 8px;
  padding: 9px 16px 9px 12px; border-radius: 11px; border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(12, 8, 18, 0.5); backdrop-filter: blur(8px); color: #fff; font-weight: 600; font-size: 13.5px;
}
:root[data-theme="light"] .detail-back { color: var(--text); border-color: var(--border); background: rgba(255, 255, 255, 0.6); }
.detail-back svg { width: 17px; height: 17px; }
.detail-header { position: absolute; bottom: 0; left: 0; right: 0; padding: 0 56px 26px; display: flex; align-items: flex-end; gap: 20px; }
.detail-title-wrap { flex: 1; min-width: 0; }
.detail-plat {
  display: inline-flex; align-items: center; gap: 8px; font-family: var(--mono); font-size: 12px;
  letter-spacing: 0.08em; text-transform: uppercase; color: rgba(255, 255, 255, 0.9); margin-bottom: 12px;
  padding: 4px 11px 4px 8px; border-radius: 99px; background: rgba(12, 8, 18, 0.5); backdrop-filter: blur(6px);
  border: 1px solid rgba(255, 255, 255, 0.16);
}
:root[data-theme="light"] .detail-plat { color: var(--text-dim); background: rgba(255, 255, 255, 0.6); border-color: var(--border); }
.detail-plat :deep(.platform-icon) { width: 15px; height: 15px; }
.detail-title {
  font-size: clamp(36px, 5vw, 60px); font-weight: 800; letter-spacing: -0.035em; margin: 0; color: #fff;
  line-height: 1; text-shadow: 0 3px 24px rgba(0, 0, 0, 0.4); text-wrap: balance;
}
:root[data-theme="light"] .detail-title { color: var(--text); text-shadow: none; }
.detail-actions { display: flex; gap: 10px; flex: none; padding-bottom: 6px; z-index: 10; }
.play-wrap { position: relative; }
.btn-play.big .caret { width: 17px; height: 17px; margin-left: 2px; }
.launch-menu {
  /* Le bouton Jouer est tout en bas de .detail-banner (overflow:hidden) : un menu
     ouvert vers le bas déborderait sous la bannière et serait recouvert par le corps
     de la page. On l'ouvre donc vers le haut, où il reste dans la bannière. */
  position: absolute; bottom: calc(100% + 8px); left: 0; z-index: 200; min-width: 230px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
  box-shadow: var(--shadow-hero); padding: 7px; display: flex; flex-direction: column; gap: 2px;
}
.launch-menu-label {
  font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint);
  font-weight: 700; padding: 6px 9px 4px;
}
.launch-opt {
  display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13.5px; text-align: left; width: 100%;
}
.launch-opt:hover { background: var(--surface-2); }
.launch-opt :deep(.platform-icon) { width: 17px; height: 17px; flex: none; }
.launch-opt-name { font-weight: 600; }
.launch-opt-tag {
  margin-left: auto; font-family: var(--mono); font-size: 10.5px; padding: 2px 8px; border-radius: 99px;
  background: var(--surface-3); color: var(--text-faint);
}
.launch-opt-tag.on { background: color-mix(in srgb, var(--manual) 20%, transparent); color: var(--manual); }

/* Bouton favori épinglé : rempli en couleur d'accent. */
.btn-ghost.fav-on { background: var(--accent); color: var(--accent-ink); border-color: transparent; }
.btn-ghost.fav-on:hover { background: var(--accent-hover); }

/* Engrenage « Options du jeu » → menu. Ouvert vers le haut, comme le menu Jouer.
   `display: flex` pour que le bouton s'étire à la hauteur de ses voisins (favori/boutique),
   sinon le wrap s'étire mais pas le bouton → engrenage plus court que les autres. */
.settings-wrap { position: relative; display: flex; }
.settings-menu {
  position: absolute; bottom: calc(100% + 8px); right: 0; z-index: 200; min-width: 190px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
  box-shadow: var(--shadow-hero); padding: 7px; display: flex; flex-direction: column; gap: 2px;
}
.settings-menu-label {
  font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint);
  font-weight: 700; padding: 6px 9px 4px;
}
.settings-opt {
  display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13.5px; text-align: left; width: 100%;
}
.settings-opt svg { width: 17px; height: 17px; flex: none; }
.settings-opt:hover:not(:disabled) { background: var(--surface-2); }
.settings-opt.danger { color: #ff6b6b; }
.settings-opt.danger:hover { background: color-mix(in srgb, #ff6b6b 15%, transparent); }
.settings-opt:disabled { opacity: 0.6; cursor: default; }
/* minmax(0, 1fr) : autorise la colonne gauche à rétrécir sous la largeur de son
   contenu (sinon la rangée de captures en flex élargit le grid et pousse le bandeau
   stats hors de l'écran → scroll horizontal). Les captures scrollent dans leur propre conteneur. */
.detail-body { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 40px; padding: 34px 56px 70px; align-items: start; }
.detail-section { margin-bottom: 34px; z-index: 1; }
.detail-section h4 {
  font-size: 13px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint);
  font-weight: 700; margin: 0 0 16px;
}
.detail-desc { font-size: 15.5px; line-height: 1.7; color: var(--text-dim); max-width: 90ch; }
.detail-desc.dim { color: var(--text-faint); font-style: italic; }
.sec-count {
  display: inline-grid; place-items: center; min-width: 20px; height: 20px; padding: 0 6px; margin-left: 8px;
  border-radius: 99px; background: var(--surface-2); color: var(--text-dim);
  font-family: var(--mono); font-size: 11px; font-weight: 700; letter-spacing: 0; vertical-align: middle;
}
.owners-list { display: flex; flex-wrap: wrap; gap: 8px; }
.owner {
  display: inline-flex; align-items: center; gap: 8px; padding: 5px 12px 5px 5px; border-radius: 99px;
  background: var(--surface); border: 1px solid var(--border); color: var(--text-dim); cursor: pointer;
  font-size: 13.5px; transition: all 0.14s;
}
.owner:hover { color: var(--text); border-color: var(--border-strong); transform: translateY(-1px); }
.owner-av { width: 26px; height: 26px; flex: none; border-radius: 50%; overflow: hidden; }
.owner-av img, .owner-av-fb { width: 26px; height: 26px; border-radius: 50%; object-fit: cover; display: grid; place-items: center; }
.owner-av-fb { background: linear-gradient(140deg, #6b6f7a, #3a3d47); color: #fff; font-size: 11px; font-weight: 700; font-family: var(--mono); }
.owner-name { max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.shots-wrap { position: relative; }
.shots { display: flex; gap: 14px; overflow-x: auto; padding-bottom: 8px; }
.shots-nav {
  position: absolute; top: calc(50% - 4px); transform: translateY(-50%); z-index: 6;
  width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%;
  background: rgba(14, 10, 20, 0.62); border: 1px solid rgba(255, 255, 255, 0.22);
  color: #fff; backdrop-filter: blur(8px); box-shadow: var(--shadow-card); cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}
.shots-nav:hover { background: rgba(28, 20, 38, 0.9); border-color: rgba(255, 255, 255, 0.4); }
.shots-nav.prev { left: 8px; }
.shots-nav.next { right: 8px; }
.shots-nav svg { width: 21px; height: 21px; }
.shot {
  flex: none; width: 300px; aspect-ratio: 16 / 9; border-radius: 13px; border: 1px solid var(--border);
  box-shadow: var(--shadow-card); position: relative; overflow: hidden;
}
.shot::after {
  content: ""; position: absolute; inset: 0;
  background: repeating-linear-gradient(120deg, rgba(255, 255, 255, 0.05) 0 1px, transparent 1px 9px);
  mix-blend-mode: overlay;
}
img.shot { object-fit: cover; }
.shot-btn { padding: 0; cursor: zoom-in; background: none; display: block; }
.shot-btn img { width: 100%; height: 100%; object-fit: cover; display: block; }
.shot-btn::after {
  content: ""; position: absolute; inset: 0;
  background: repeating-linear-gradient(120deg, rgba(255, 255, 255, 0.05) 0 1px, transparent 1px 9px);
  mix-blend-mode: overlay; pointer-events: none;
}
.shot-btn:hover { border-color: var(--border-strong); }

.lightbox {
  position: fixed; inset: 0; z-index: 300; display: grid; place-items: center; padding: 4vh 5vw;
  background: rgba(6, 4, 10, 0.86); backdrop-filter: blur(6px); cursor: zoom-out;
}
.lb-img {
  max-width: 100%; max-height: 92vh; border-radius: 12px; object-fit: contain;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6); cursor: default;
}
.lb-close, .lb-nav {
  position: fixed; display: grid; place-items: center; border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.18); background: rgba(14, 10, 20, 0.55);
  color: #fff; backdrop-filter: blur(8px); cursor: pointer;
}
.lb-close { top: 20px; right: 22px; width: 42px; height: 42px; }
.lb-close svg { width: 20px; height: 20px; }
.lb-nav { top: 50%; transform: translateY(-50%); width: 48px; height: 48px; }
.lb-nav.prev { left: 20px; }
.lb-nav.next { right: 20px; }
.lb-nav svg { width: 24px; height: 24px; }
.lb-close:hover, .lb-nav:hover { background: rgba(30, 22, 40, 0.8); border-color: rgba(255, 255, 255, 0.35); }
.lb-count {
  position: fixed; bottom: 22px; left: 50%; transform: translateX(-50%);
  font-family: var(--mono); font-size: 12.5px; color: rgba(255, 255, 255, 0.85);
  padding: 5px 12px; border-radius: 99px; background: rgba(14, 10, 20, 0.55); backdrop-filter: blur(8px);
}
.ach-list { display: flex; flex-direction: column; gap: 12px; }
.ach { display: flex; align-items: center; gap: 14px; }
.ach-badge {
  width: 44px; height: 44px; border-radius: 10px; flex: none; overflow: hidden;
  background: var(--surface-2); border: 1px solid var(--border);
}
.ach-badge img { width: 100%; height: 100%; object-fit: cover; display: block; }
/* Succès verrouillé : icône désaturée + assombrie, texte atténué. */
.ach.locked .ach-badge img { filter: grayscale(1) brightness(0.65); }
.ach.locked .ach-name { color: var(--text-dim); }
.ach-info { flex: 1; min-width: 0; }
.ach-name { font-size: 14px; font-weight: 600; }
.ach-desc {
  font-size: 12.5px; color: var(--text-dim); margin-top: 1px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.ach-pct { font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); margin-top: 2px; }
.ach-toggle {
  margin-top: 16px; padding: 8px 14px; border-radius: 9px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text-dim); font-size: 13px; font-weight: 600; cursor: pointer;
}
.ach-toggle:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-2); }

.stat-card {
  background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 22px;
  position: sticky; top: 24px;
}
.stat-play {
  font-family: var(--mono); font-size: 44px; font-weight: 700; letter-spacing: -0.03em; color: var(--text);
  line-height: 1; font-variant-numeric: tabular-nums;
}
.stat-play span { font-size: 20px; color: var(--text-faint); }
.stat-label { font-size: 12px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; margin-top: 8px; }
.stat-rows { margin-top: 22px; display: flex; flex-direction: column; }
.stat-row {
  display: flex; justify-content: space-between; align-items: center; padding: 12px 0;
  border-top: 1px solid var(--border); font-size: 13.5px;
}
.stat-row .k { color: var(--text-dim); display: flex; align-items: center; gap: 9px; }
.stat-row .k svg { width: 15px; height: 15px; color: var(--text-faint); }
.stat-row .v { font-family: var(--mono); color: var(--text); font-variant-numeric: tabular-nums; }
.ach-progress { margin-top: 22px; }
.ach-progress .top { display: flex; justify-content: space-between; font-size: 13px; margin-bottom: 9px; }
.ach-progress .top b { font-family: var(--mono); }
.ach-bar { height: 7px; border-radius: 99px; background: var(--surface-3); overflow: hidden; }
.ach-fill { height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--accent), #ffb08a); }

@media (max-width: 980px) {
  .detail-body { grid-template-columns: minmax(0, 1fr); }
  .stat-card { position: static; }
}
@media (max-width: 820px) {
  .detail-header, .detail-body { padding-left: 22px; padding-right: 22px; }
}
</style>
