<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useFichePossede } from "../composables/useFichePossede";
import { useFriendsCommon } from "../composables/useFriendsCommon";
import { useStore } from "../composables/useStore";
import { useUi } from "../composables/useUi";
import { useTorii } from "../composables/useTorii";
import { useScrollLock } from "../composables/useScrollLock";
import { useFocusTrap } from "../composables/useFocusTrap";
import { estHorsLauncher, platformName } from "../data/platforms";
import { libelleAvis, nombre, pageSteam, tonAvis } from "../lib/boutique";
import { installSource, launchSource, openExternal, openInstallDir, steamAchievements, steamCurrentPlayers, uninstallGame } from "../lib/tauri";
import type { FriendLib, GameSource, SteamAchievements } from "../types";
import GameDetailAchievements from "./GameDetailAchievements.vue";
import GameDetailDlc from "./GameDetailDlc.vue";
import GameDetailNews from "./GameDetailNews.vue";
import PlatformIcon from "./PlatformIcon.vue";
import StoreSheetBlocs from "./StoreSheetBlocs.vue";
import StoreSheetConfig from "./StoreSheetConfig.vue";
import StoreSheetFacts from "./StoreSheetFacts.vue";
import StoreSheetMedia from "./StoreSheetMedia.vue";
import { etiquetteIntl, langueCourante, t } from "../i18n";
import { useImageCascade } from "../lib/images";

const { byId, ensureEnriched, refreshInfo, enrichingId, setFavorite, markPlayed, launchOrInstall, removeManual } = useLibrary();
const { charger: chargerFiche, entree: entreeFiche } = useFichePossede();
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

/**
 * Le panneau recouvre la page : au clavier, sans piège, la tabulation continuait de
 * parcourir la bibliothèque cachée derrière lui.
 */
const panneau = ref<HTMLElement | null>(null);
useFocusTrap(panneau, computed(() => !!game.value));

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
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name, etiquetteIntl.value));
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
/** Redemande description, visuels et captures en ignorant les caches. */
function onRefreshInfo() {
  if (game.value) void refreshInfo(game.value.id);
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

/** Un jeu que Torii est seul à connaître (ajouté à la main ou repéré hors launcher) :
 *  « désinstaller » = le retirer d'ici, et sa fiche se corrige à la main. */
const isManual = computed(() =>
  estHorsLauncher(game.value?.platform),
);

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

const { src: bannerSrc, onError: onBannerError } = useImageCascade(() => [game.value?.heroUrl, game.value?.heroAlt]);

function hideBrokenCover(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}

/**
 * Appid Steam du jeu affiché (jeu Steam direct, ou source Steam d'un jeu fusionné
 * multi-plateforme). `null` si aucune provenance Steam → ni succès, ni contenus possédés.
 */
const steamAppid = computed<string | null>(() => {
  const g = game.value;
  if (!g) return null;
  if (g.platform === "steam") {
    return g.launchTarget ?? (g.id.startsWith("steam:") ? g.id.slice(6) : null);
  }
  return g.sources?.find((s) => s.platform === "steam")?.launchTarget ?? null;
});

/**
 * Ce que Steam dit du jeu — pour un jeu Steam, mais aussi pour un jeu acheté ailleurs
 * quand le lien est sûr (Control sur Epic). Chaque bloc de la fiche n'apparaît que si sa
 * donnée existe : un jeu Riot n'a que ce qu'IGDB et le launcher en savent, et sa fiche
 * raccourcit d'autant, sans case vide.
 */
watch(
  selectedGameId,
  (id) => {
    if (id && game.value) void chargerFiche(game.value, steamAppid.value);
  },
  { immediate: true },
);
const fiche = computed(() => entreeFiche(game.value?.id).fiche);
const steam = computed(() => fiche.value?.steam ?? null);
/** L'appid de la page Steam du jeu, s'il en a une sûre. */
const appIdSteam = computed(() => fiche.value?.appId ?? (steamAppid.value ? Number(steamAppid.value) : null));
function ouvrirSteam() {
  if (appIdSteam.value) void openExternal(pageSteam(appIdSteam.value));
}

/** Médias : ceux de Steam (bandes-annonces avec le son, toutes les captures), sinon les captures connues. */
const captures = computed(() =>
  steam.value?.captures.length
    ? steam.value.captures
    : (game.value?.screenshots ?? []).map((s) => ({ mini: s, grande: s })),
);
const bandesAnnonces = computed(() => steam.value?.bandesAnnonces ?? []);

/** Visionneuse plein écran des captures (index dans `captures`, sinon fermée). */
const shots = computed(() => captures.value.map((c) => c.grande));
const zoomIndex = ref<number | null>(null);
const zoomedShot = computed(() => (zoomIndex.value != null ? shots.value[zoomIndex.value] ?? null : null));
function openShot(i: number) {
  zoomIndex.value = i;
}
function closeShot() {
  zoomIndex.value = null;
}
function stepShot(delta: number) {
  const n = shots.value.length;
  if (zoomIndex.value == null || n === 0) return;
  zoomIndex.value = (zoomIndex.value + delta + n) % n;
}

// Réinitialise la visionneuse et le menu de lancement quand on change de jeu, et repart du haut.
watch(selectedGameId, () => {
  zoomIndex.value = null;
  launchMenuOpen.value = false;
  uninstallMenuOpen.value = false;
  if (panneau.value) panneau.value.scrollTop = 0;
});

/**
 * Description : celle d'une source sûre dans la langue de l'interface (Steam, GOG, Epic),
 * sinon le résumé Steam d'un jeu relié à Steam, sinon celle d'IGDB — signalée quand elle
 * reste en anglais dans une interface qui ne l'est pas.
 */
const description = computed(() => {
  const g = game.value;
  if (!g) return null;
  if (g.description && g.descriptionSource === "locale") return { texte: g.description, anglais: false };
  if (steam.value?.resume) return { texte: steam.value.resume, anglais: false };
  if (!g.description) return null;
  return { texte: g.description, anglais: g.descriptionSource === "igdb" && langueCourante.value !== "en" };
});

/** Vrais succès Steam (rareté comprise) récupérés à l'ouverture de la fiche. */
const steamAch = ref<SteamAchievements | null>(null);
const loadingAch = ref(false);

/** Joueurs en ce moment sur le jeu Steam (API publique), affiché dans la stat-card. */
const currentPlayers = ref<number | null>(null);
const playersLabel = computed(() =>
  currentPlayers.value != null ? currentPlayers.value.toLocaleString(etiquetteIntl.value) : null,
);
const achPct = computed(() => {
  const a = steamAch.value;
  return a && a.total > 0 ? Math.round((a.unlocked / a.total) * 100) : 0;
});

/** « 20 octobre », ou « 20 oct. 2025 » pour une autre année. */
function dateCourte(s: number): string {
  const d = new Date(s * 1000);
  const memeAnnee = d.getFullYear() === new Date().getFullYear();
  return new Intl.DateTimeFormat(etiquetteIntl.value, {
    day: "numeric",
    month: memeAnnee ? "long" : "short",
    year: memeAnnee ? undefined : "numeric",
  }).format(d);
}
const taille = (n: number) => t("fiche.stats.tailleGo", { n: nombre(n) });

/** Sous le titre : ton rapport au jeu, en cases. Une case sans donnée n'existe pas. */
const bandeau = computed(() => {
  const g = game.value;
  if (!g) return [];
  const cases: { cle: string; titre: string; valeur: string; detail?: string }[] = [];
  if (g.hoursPlayed != null) {
    cases.push({
      cle: "temps",
      titre: t("fiche.bandeau.tempsDeJeu"),
      valeur: `${nombre(g.hoursPlayed)} ${t("fiche.stats.heures")}`,
      detail: t("fiche.bandeau.selon", { plateforme: platformName(g.platform) }),
    });
  }
  if (g.lastPlayedAt) {
    cases.push({ cle: "session", titre: t("fiche.bandeau.derniereSession"), valeur: dateCourte(g.lastPlayedAt), detail: g.lastPlayed });
  }
  if (g.sizeGb) {
    cases.push({
      cle: "taille",
      titre: g.installed ? t("fiche.bandeau.installe") : t("fiche.bandeau.aInstaller"),
      valeur: taille(g.sizeGb),
      detail: g.installed ? t("fiche.bandeau.surDisque") : t("fiche.bandeau.aTelecharger"),
    });
  }
  return cases;
});

/** Le grand chiffre de la carte de droite : le temps de jeu, sinon la dernière session, sinon la taille. */
const grandChiffre = computed(() => {
  const g = game.value;
  if (!g) return null;
  if (g.hoursPlayed != null) {
    return {
      valeur: nombre(g.hoursPlayed),
      unite: t("fiche.stats.heures"),
      libelle: `${t("fiche.bandeau.tempsDeJeu")} · ${t("fiche.bandeau.selon", { plateforme: platformName(g.platform) })}`,
    };
  }
  if (g.lastPlayedAt) {
    // « 27 sept. » : en grand, le mois entier passerait sur deux lignes.
    const d = new Date(g.lastPlayedAt * 1000);
    const memeAnnee = d.getFullYear() === new Date().getFullYear();
    const valeur = new Intl.DateTimeFormat(etiquetteIntl.value, {
      day: "numeric",
      month: "short",
      year: memeAnnee ? undefined : "numeric",
    }).format(d);
    return { valeur, unite: "", libelle: t("fiche.bandeau.derniereSession"), texte: true };
  }
  if (g.sizeGb) {
    return {
      valeur: nombre(g.sizeGb),
      unite: t("fiche.stats.go"),
      libelle: g.installed ? t("fiche.stats.surDisque") : t("fiche.stats.tailleJeu"),
    };
  }
  return null;
});

/** Note Metacritic : même code couleur que la Boutique. */
const tonNote = (score: number) => (score >= 75 ? "pos" : score >= 50 ? "mixte" : "neg");
const aDesNotes = computed(() => !!steam.value && !!(steam.value.reviewScore || steam.value.metacritic || steam.value.classification));

const liens = computed(() => {
  const out: { cle: string; libelle: string; url: string }[] = [];
  const appId = appIdSteam.value;
  if (appId) out.push({ cle: "steam", libelle: t("fiche.liens.pageSteam"), url: pageSteam(appId) });
  if (steam.value?.site) out.push({ cle: "site", libelle: t("fiche.liens.site"), url: steam.value.site });
  if (appId) out.push({ cle: "communaute", libelle: t("fiche.liens.communaute"), url: `https://steamcommunity.com/app/${appId}` });
  return out;
});

// Charge les succès Steam à l'ouverture d'une fiche (anti-course par id).
watch(
  selectedGameId,
  async (id) => {
    steamAch.value = null;
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
onBeforeUnmount(() => document.removeEventListener("keydown", onKey));
</script>

<template>
  <div
    ref="panneau"
    class="detail"
    :class="{ open: !!game }"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    :aria-label="game ? game.title : t('fiche.fiche')"
  >
    <template v-if="game">
      <div class="detail-banner" :class="{ riche: bandeau.length || aDesNotes || (steamAch && steamAch.total) }">
        <div class="detail-banner-art" :style="{ background: game.cover }" />
        <img v-if="bannerSrc" :key="bannerSrc" class="detail-banner-img" :src="bannerSrc" alt="" @error="onBannerError" />
        <div class="detail-banner-scrim" />
        <button class="detail-back" @click="closeGame">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>{{ t("fiche.retour") }}
        </button>
        <div class="detail-header">
          <div class="detail-head-row">
          <div class="detail-title-wrap">
            <span class="detail-plat"><PlatformIcon :platform="game.platform" />{{ platformName(game.platform) }}</span>
            <h1 class="detail-title">{{ game.title }}</h1>
          </div>
          <div class="detail-actions">
            <div class="play-wrap">
              <button class="btn-play big" @click.stop="onPlay">
                <svg v-if="game.installed" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
                <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v12m0 0l-4.5-4.5M12 15l4.5-4.5M5 21h14" /></svg>
                {{ game.installed ? t("bibliotheque.jouer") : t("bibliotheque.installer") }}
                <svg v-if="multiSource" class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
              </button>
              <div v-if="multiSource && launchMenuOpen" class="launch-menu" @click.stop>
                <div class="launch-menu-label">{{ game.installed ? t("fiche.jouerDepuis") : t("fiche.installerDepuis") }}</div>
                <button
                  v-for="s in sources"
                  :key="s.platform + (s.launchTarget ?? '')"
                  class="launch-opt"
                  @click="playFrom(s)"
                >
                  <PlatformIcon :platform="s.platform" />
                  <span class="launch-opt-name">{{ platformName(s.platform) }}</span>
                  <span class="launch-opt-tag" :class="s.installed ? 'on' : ''">{{ s.installed ? t("bibliotheque.installe") : t("bibliotheque.nonInstalle") }}</span>
                </button>
              </div>
            </div>
            <button
              class="btn-ghost solid"
              :class="{ 'fav-on': game.favorite }"
              :title="game.favorite ? t('bibliotheque.carte.retirerFavoris') : t('bibliotheque.carte.ajouterFavoris')"
              :aria-pressed="game.favorite"
              @click.stop="toggleFavorite"
            >
              <svg viewBox="0 0 24 24" :fill="game.favorite ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 4.5l2.3 4.7 5.2.8-3.8 3.7.9 5.1L12 16.9l-4.6 2.4.9-5.1L4.5 10l5.2-.8z" /></svg>
            </button>
            <button class="btn-ghost solid" :title="t('fiche.voirBoutique')" :aria-label="t('fiche.voirBoutique')" @click.stop="viewInStore">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20.5 13.3 12.8 21a1.5 1.5 0 0 1-2.1 0l-7-7a1.4 1.4 0 0 1-.4-1V4.6A1.5 1.5 0 0 1 4.6 3h8.4a1.4 1.4 0 0 1 1 .4l6.5 6.5a2 2 0 0 1 0 2.8Z" /><circle cx="7.8" cy="7.8" r="1.4" fill="currentColor" stroke="none" /></svg>
            </button>
            <!-- Visible pour tous les jeux : « Actualiser les infos » concerne aussi les
                 jeux non installés. Les options d'installation restent réservées aux autres. -->
            <div class="settings-wrap">
              <button
                class="btn-ghost solid"
                :title="t('fiche.options')"
                aria-haspopup="true"
                :aria-expanded="uninstallMenuOpen"
                @click.stop="toggleSettingsMenu"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3" /><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.3 1a7 7 0 0 0-1.7-1L14.5 2h-5l-.4 2.5a7 7 0 0 0-1.7 1l-2.3-1-2 3.5 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.5 2.3-1a7 7 0 0 0 1.7 1l.4 2.5h5l.4-2.5a7 7 0 0 0 1.7-1l2.3 1 2-3.5-2-1.5a7 7 0 0 0 .1-1Z" /></svg>
              </button>
              <div v-if="uninstallMenuOpen" class="settings-menu" @click.stop>
                <div class="settings-menu-label">{{ t("fiche.optionsTitre") }}</div>
                <button v-if="game.installDir" class="settings-opt" @click="onOpenFolder">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" /></svg>
                  <span>{{ t("bibliotheque.menu.ouvrirEmplacement") }}</span>
                </button>
                <button class="settings-opt" :disabled="loadingMeta" @click="onRefreshInfo">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 11a8 8 0 0 0-14.7-4.3L3 9" /><path d="M3 4v5h5" /><path d="M4 13a8 8 0 0 0 14.7 4.3L21 15" /><path d="M21 20v-5h-5" /></svg>
                  <span>{{ t("fiche.actualiserInfos") }}</span>
                </button>
                <button v-if="toriiConnected" class="settings-opt" @click="onToggleMuted">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 3l18 18" /><path d="M10.6 10.7a2 2 0 0 0 2.8 2.8" /><path d="M9.4 5.2A9.3 9.3 0 0 1 12 5c5 0 9 4.5 9 7a12 12 0 0 1-2.2 3M6.1 6.2A12.7 12.7 0 0 0 3 12c0 2.5 4 7 9 7a9.4 9.4 0 0 0 3.6-.7" /></svg>
                  <span>{{ gameMuted ? t("bibliotheque.menu.diffuser") : t("bibliotheque.menu.nePasDiffuser") }}</span>
                </button>
                <button v-if="isManual" class="settings-opt" @click="onEdit">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 20h4L19 9a2.1 2.1 0 0 0-3-3L5 17Z" /><path d="M14.5 7.5 16.5 9.5" /></svg>
                  <span>{{ t("bibliotheque.menu.modifier") }}</span>
                </button>
                <button
                  v-if="game.installed"
                  class="settings-opt danger"
                  :disabled="uninstalling"
                  @click="isManual ? onRemoveManual() : onUninstall()"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m2 0v14a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6M10 11v6M14 11v6" /></svg>
                  <span>
                    {{ uninstalling
                      ? (isManual ? t("fiche.retrait") : t("fiche.desinstallation"))
                      : (isManual ? t("bibliotheque.menu.retirer") : t("bibliotheque.menu.desinstaller")) }}
                  </span>
                </button>
              </div>
            </div>
          </div>
          </div>

          <!-- Ton rapport au jeu, puis ce que Steam en dit. Rien de vide : une case sans
               donnée n'est pas dessinée. -->
          <div v-if="bandeau.length || (steamAch && steamAch.total)" class="detail-strip">
            <div v-for="c in bandeau" :key="c.cle" class="strip-cell">
              <span class="strip-titre">{{ c.titre }}</span>
              <span class="strip-valeur">{{ c.valeur }}</span>
              <span v-if="c.detail" class="strip-detail">{{ c.detail }}</span>
            </div>
            <div v-if="steamAch && steamAch.total" class="strip-cell large">
              <span class="strip-titre">{{ t("fiche.succes") }}</span>
              <span class="strip-valeur">{{ steamAch.unlocked }} / {{ steamAch.total }} <small>· {{ achPct }} %</small></span>
              <span class="strip-barre"><span :style="{ width: `${achPct}%` }" /></span>
            </div>
          </div>
          <div v-if="steam && aDesNotes" class="detail-badges">
            <button v-if="steam.reviewScore" class="badge" :title="t('boutique.produit.voirSur', { source: 'Steam' })" @click="ouvrirSteam">
              <span class="pastille" :class="tonAvis(steam.reviewScore)" />
              Steam · {{ libelleAvis(steam.reviewScore) }}<template v-if="steam.reviewPercent != null"> · {{ steam.reviewPercent }} %</template>
              <span v-if="steam.reviewCount" class="dim">({{ nombre(steam.reviewCount) }})</span>
            </button>
            <button
              v-if="steam.metacritic"
              class="badge"
              :disabled="!steam.metacriticUrl"
              :title="t('boutique.produit.voirSur', { source: 'Metacritic' })"
              @click="steam.metacriticUrl && openExternal(steam.metacriticUrl)"
            >
              <b class="score" :class="tonNote(steam.metacritic)">{{ steam.metacritic }}</b>Metacritic
            </button>
            <span v-if="steam.classification" class="badge fixe">
              <b>{{ steam.classification.systeme }} {{ steam.classification.note }}</b>
            </span>
          </div>
        </div>
      </div>

      <div class="detail-body">
        <div class="detail-main">
          <StoreSheetMedia
            v-if="bandesAnnonces.length || captures.length"
            :key="`${game.id}-${steam ? 'steam' : 'base'}`"
            class="detail-section"
            :bandes-annonces="bandesAnnonces"
            :captures="captures"
            @agrandir="openShot"
            @steam="ouvrirSteam"
          />

          <GameDetailNews
            v-if="fiche && fiche.actualites.length"
            class="detail-section"
            :actualites="fiche.actualites"
            :app-id="fiche.appId"
            :studio="game.developer"
          />

          <GameDetailAchievements
            v-if="loadingAch || (steamAch && steamAch.items.length)"
            class="detail-section"
            :succes="steamAch"
            :chargement="loadingAch"
          />

          <GameDetailDlc
            v-if="fiche && fiche.dlc.length"
            :key="`dlc-${fiche.appId}`"
            class="detail-section"
            :app-id="fiche.appId"
            :dlc="fiche.dlc"
            :possedes="fiche.dlcPossedes"
            :total="steam?.dlcTotal ?? fiche.dlc.length"
          />

          <div v-if="description || loadingMeta" class="detail-section">
            <div class="titre-ligne">
              <h4>{{ t("fiche.apropos") }}</h4>
              <span v-if="description?.anglais" class="etiquette">{{ t("fiche.enAnglais") }}</span>
            </div>
            <p v-if="description" class="detail-desc" :lang="description.anglais ? 'en' : undefined">{{ description.texte }}</p>
            <p v-else class="detail-desc dim">{{ t("fiche.chargementDetails") }}</p>
            <StoreSheetBlocs v-if="steam" :key="game.id" :blocs="steam.description" />
          </div>

          <StoreSheetFacts v-if="steam" class="detail-section" :steam="steam" :hors-steam="!steamAppid" />

          <StoreSheetConfig
            v-if="!game.installed && steam?.configuration"
            class="detail-section"
            :configuration="steam.configuration"
          />

          <div v-if="friendOwners.length" class="detail-section">
            <h4>{{ t("fiche.amis") }} <span class="sec-count">{{ friendOwners.length }}</span></h4>
            <div class="owners-list">
              <button
                v-for="f in friendOwners"
                :key="f.steamId"
                class="owner"
                :title="t('fiche.voirProfil', { nom: f.name })"
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
        </div>

        <aside class="detail-aside">
          <div class="stat-card">
            <template v-if="grandChiffre">
              <div class="stat-play" :class="{ texte: 'texte' in grandChiffre }">{{ grandChiffre.valeur }}<span v-if="grandChiffre.unite"> {{ grandChiffre.unite }}</span></div>
              <div class="stat-label">{{ grandChiffre.libelle }}</div>
            </template>
            <div class="stat-rows" :class="{ seules: !grandChiffre }">
              <div class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 5a2 2 0 0 1 2-2h9l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z" /></svg>{{ t("fiche.stats.statut") }}</span><span class="v">{{ game.installed ? t("bibliotheque.installe") : t("bibliotheque.nonInstalle") }}</span></div>
              <div v-if="game.lastPlayed" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>{{ t("fiche.stats.derniereSession") }}</span><span class="v">{{ game.lastPlayed }}</span></div>
              <div v-if="playersLabel" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="8" r="3" /><path d="M3.5 19a5.5 5.5 0 0 1 11 0" /><path d="M16 6a3 3 0 0 1 0 5.6M17.5 19a5.5 5.5 0 0 0-2.5-4.3" /></svg>{{ t("fiche.stats.enCeMoment") }}</span><span class="v">{{ playersLabel }}</span></div>
              <div v-if="game.developer" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h10" /></svg>{{ t("fiche.stats.developpeur") }}</span><span class="v">{{ game.developer }}</span></div>
              <div v-if="game.year" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" /><path d="M8 2v4M16 2v4M3 10h18" /></svg>{{ t("fiche.stats.sortie") }}</span><span class="v">{{ game.year }}</span></div>
              <div v-if="game.genre" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h18M3 6h18M3 18h18" /></svg>{{ t("fiche.stats.genre") }}</span><span class="v">{{ game.genre }}</span></div>
              <div v-if="game.sizeGb" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 5a2 2 0 0 1 2-2h9l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z" /></svg>{{ t("fiche.stats.taille") }}</span><span class="v">{{ taille(game.sizeGb) }}</span></div>
              <div v-if="showFamily" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="8" r="3" /><path d="M3.5 19a5.5 5.5 0 0 1 11 0" /><path d="M16 6a3 3 0 0 1 0 5.6M17.5 19a5.5 5.5 0 0 0-2.5-4.3" /></svg>{{ t("fiche.stats.familleSteam") }}</span><span class="v">{{ t("fiche.stats.copies", { n: familyCopies }) }}</span></div>
            </div>
          </div>

          <div v-if="liens.length" class="liens-card">
            <span class="liens-titre">{{ t("fiche.liens.titre") }}</span>
            <button v-for="l in liens" :key="l.cle" class="lien" @click="openExternal(l.url)">{{ l.libelle }} ↗</button>
          </div>
        </aside>
      </div>

      <div v-if="zoomedShot" class="lightbox" @click.self="closeShot">
        <button class="lb-close" :aria-label="t('commun.fermer')" @click="closeShot">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
        <button
          v-if="shots.length > 1"
          class="lb-nav prev"
          :aria-label="t('fiche.capturePrecedente')"
          @click.stop="stepShot(-1)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
        </button>
        <img class="lb-img" :src="zoomedShot" alt="" @error="hideBrokenCover" />
        <button
          v-if="shots.length > 1"
          class="lb-nav next"
          :aria-label="t('fiche.captureSuivante')"
          @click.stop="stepShot(1)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
        </button>
        <div v-if="shots.length > 1" class="lb-count">{{ (zoomIndex ?? 0) + 1 }} / {{ shots.length }}</div>
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
/* Bannière plus haute quand le bandeau et les notes s'ajoutent sous le titre. */
.detail-banner.riche { height: 56vh; min-height: 460px; }
.detail-header {
  position: absolute; bottom: 0; left: 0; right: 0; padding: 0 56px 22px;
  display: flex; flex-direction: column; gap: 14px;
}
.detail-head-row { display: flex; align-items: flex-end; gap: 20px; }
.detail-title-wrap { flex: 1; min-width: 0; }

/* Bandeau et notes : toujours clair sur sombre, ils sont posés sur l'image du jeu. */
/* Aussi large que ses cases : deux cases étirées sur tout l'écran flottaient dans le vide. */
.detail-strip {
  align-self: flex-start; max-width: 100%;
  display: flex; flex-wrap: wrap; row-gap: 12px; padding: 12px 6px; border-radius: 14px;
  background: rgba(12, 8, 18, 0.72); border: 1px solid rgba(255, 255, 255, 0.12); backdrop-filter: blur(10px);
}
.strip-cell { flex: 0 1 auto; min-width: 170px; display: flex; flex-direction: column; gap: 3px; padding: 0 24px 0 18px; }
.strip-cell + .strip-cell { border-left: 1px solid rgba(255, 255, 255, 0.1); }
.strip-cell.large { min-width: 260px; gap: 6px; }
.strip-titre { font-size: 11px; text-transform: uppercase; letter-spacing: 0.1em; color: rgba(255, 255, 255, 0.6); font-weight: 700; }
.strip-valeur { font-size: 16px; font-weight: 700; color: #fff; font-variant-numeric: tabular-nums; }
.strip-valeur small { font-size: 12px; font-weight: 500; color: rgba(255, 255, 255, 0.6); }
.strip-detail { font-size: 12px; color: rgba(255, 255, 255, 0.6); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.strip-barre { height: 6px; border-radius: 99px; background: rgba(255, 255, 255, 0.18); overflow: hidden; }
.strip-barre span { display: block; height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--accent), #ffb08a); }
.detail-badges { display: flex; flex-wrap: wrap; gap: 8px; }
.badge {
  display: inline-flex; align-items: center; gap: 7px; padding: 6px 11px; border-radius: 9px; font-size: 12.5px;
  background: rgba(12, 8, 18, 0.72); border: 1px solid rgba(255, 255, 255, 0.16); color: #fff; cursor: pointer;
}
.badge:hover:not(:disabled):not(.fixe) { border-color: rgba(255, 255, 255, 0.35); }
.badge.fixe, .badge:disabled { cursor: default; }
.badge .dim { color: rgba(255, 255, 255, 0.6); }
.pastille { width: 8px; height: 8px; border-radius: 50%; background: rgba(255, 255, 255, 0.4); flex: none; }
.pastille.pos { background: #6ee7a8; }
.pastille.mixte { background: #f0b73f; }
.pastille.neg { background: #ff8a80; }
.score { font-family: var(--mono); }
.score.pos { color: #6ee7a8; }
.score.mixte { color: #f0b73f; }
.score.neg { color: #ff8a80; }
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
.detail-main { min-width: 0; }
.titre-ligne { display: flex; align-items: baseline; gap: 10px; margin-bottom: 16px; }
.titre-ligne h4 { margin: 0; }
.etiquette { font-size: 11.5px; padding: 2px 8px; border-radius: 6px; background: var(--surface-2); color: var(--text-dim); }
.detail-desc { font-size: 15.5px; line-height: 1.7; color: var(--text-dim); max-width: 90ch; margin: 0; }
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
.detail-aside { position: sticky; top: 24px; display: flex; flex-direction: column; gap: 16px; }
.stat-card { background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 22px; }
.stat-play {
  font-family: var(--mono); font-size: 44px; font-weight: 700; letter-spacing: -0.03em; color: var(--text);
  line-height: 1; font-variant-numeric: tabular-nums;
}
.stat-play span { font-size: 20px; color: var(--text-faint); }
.stat-play.texte { font-size: 34px; white-space: nowrap; }
.stat-label { font-size: 12px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; margin-top: 8px; }
.stat-rows { margin-top: 22px; display: flex; flex-direction: column; }
.stat-row {
  display: flex; justify-content: space-between; align-items: center; padding: 12px 0;
  border-top: 1px solid var(--border); font-size: 13.5px;
}
.stat-row .k { color: var(--text-dim); display: flex; align-items: center; gap: 9px; }
.stat-row .k svg { width: 15px; height: 15px; color: var(--text-faint); }
.stat-row .v { font-family: var(--mono); color: var(--text); font-variant-numeric: tabular-nums; text-align: right; }
.stat-rows.seules { margin-top: 0; }
.stat-rows.seules .stat-row:first-child { border-top: none; padding-top: 0; }
.liens-card {
  display: flex; flex-direction: column; align-items: flex-start; gap: 9px; padding: 16px 18px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 18px;
}
.liens-titre { font-size: 11px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; }
.liens-card .lien { background: none; border: none; padding: 0; color: var(--accent); font-size: 13px; font-weight: 600; text-align: left; }
.liens-card .lien:hover { text-decoration: underline; }
@media (max-width: 980px) {
  .detail-body { grid-template-columns: minmax(0, 1fr); }
  .detail-aside { position: static; }
}
@media (max-width: 820px) {
  .detail-header, .detail-body { padding-left: 22px; padding-right: 22px; }
}
</style>
