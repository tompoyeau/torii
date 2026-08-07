<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { platformName } from "../data/platforms";
import { launchGame, launchSource, uninstallGame } from "../lib/tauri";
import PlatformIcon from "./PlatformIcon.vue";

const { byId, ensureEnriched, enrichingId, setFavorite } = useLibrary();
const { selectedGameId, closeGame } = useUi();

const game = computed(() => byId(selectedGameId.value));

/** Provenances jouables (jeu multi-plateforme) ; vide si mono-source. */
const sources = computed(() => game.value?.sources ?? []);
const multiSource = computed(() => sources.value.length > 1);
const launchMenuOpen = ref(false);

/** Bouton « Jouer » : lance direct si une seule source, sinon ouvre le choix. */
function onPlay() {
  if (multiSource.value) launchMenuOpen.value = !launchMenuOpen.value;
  else if (game.value) launchGame(game.value);
}
function playFrom(platform: string, target?: string) {
  launchSource(platform, target);
  launchMenuOpen.value = false;
}

/** Épingle/retire le jeu affiché des favoris. */
function toggleFavorite() {
  if (game.value) setFavorite(game.value.id, !game.value.favorite);
}

// Menu de désinstallation (engrenage) : ouvert seulement pour un jeu installé.
const uninstallMenuOpen = ref(false);
const uninstalling = ref(false);
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

const achievements = [
  { name: "Premier sang", unlocked: true, rarity: 68 },
  { name: "Explorateur", unlocked: true, rarity: 54 },
  { name: "Sans une égratignure", unlocked: true, rarity: 40 },
  { name: "Collectionneur", unlocked: false, rarity: 26 },
];

const achPct = computed(() => {
  const a = game.value?.achievements;
  return a ? Math.round((a.unlocked / a.total) * 100) : 0;
});

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
                <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>Jouer
                <svg v-if="multiSource" class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
              </button>
              <div v-if="multiSource && launchMenuOpen" class="launch-menu" @click.stop>
                <div class="launch-menu-label">Jouer depuis…</div>
                <button
                  v-for="s in sources"
                  :key="s.platform + (s.launchTarget ?? '')"
                  class="launch-opt"
                  @click="playFrom(s.platform, s.launchTarget)"
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
            <div v-if="game.installed" class="settings-wrap">
              <button
                class="btn-ghost solid"
                title="Options du jeu"
                :aria-expanded="uninstallMenuOpen"
                @click.stop="uninstallMenuOpen = !uninstallMenuOpen"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3" /><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.5-2.3 1a7 7 0 0 0-1.7-1L14.5 2h-5l-.4 2.5a7 7 0 0 0-1.7 1l-2.3-1-2 3.5 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.5 2.3-1a7 7 0 0 0 1.7 1l.4 2.5h5l.4-2.5a7 7 0 0 0 1.7-1l2.3 1 2-3.5-2-1.5a7 7 0 0 0 .1-1Z" /></svg>
              </button>
              <div v-if="uninstallMenuOpen" class="settings-menu" @click.stop>
                <div class="settings-menu-label">Options</div>
                <button class="settings-opt danger" :disabled="uninstalling" @click="onUninstall">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m2 0v14a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6M10 11v6M14 11v6" /></svg>
                  <span>{{ uninstalling ? "Désinstallation…" : "Désinstaller" }}</span>
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
          <div class="detail-section">
            <h4>Captures d'écran</h4>
            <div class="shots no-scrollbar">
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
          </div>
          <div v-if="game.achievements" class="detail-section">
            <h4>Succès récents</h4>
            <div class="ach-list">
              <div v-for="a in achievements" :key="a.name" class="ach">
                <div class="ach-badge" :class="{ locked: !a.unlocked }">
                  <svg v-if="a.unlocked" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M8 21h8M12 17v4M7 4h10v5a5 5 0 0 1-10 0V4ZM7 6H4v2a3 3 0 0 0 3 3M17 6h3v2a3 3 0 0 1-3 3" /></svg>
                  <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></svg>
                </div>
                <div class="ach-info">
                  <div class="ach-name">{{ a.name }}</div>
                  <div class="ach-pct">{{ a.unlocked ? "Débloqué" : "Verrouillé" }} · obtenu par {{ a.rarity }}% des joueurs</div>
                </div>
              </div>
            </div>
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
            <div v-if="game.developer" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h10" /></svg>Développeur</span><span class="v">{{ game.developer }}</span></div>
            <div v-if="game.year" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" /><path d="M8 2v4M16 2v4M3 10h18" /></svg>Sortie</span><span class="v">{{ game.year }}</span></div>
            <div v-if="game.genre" class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h18M3 6h18M3 18h18" /></svg>Genre</span><span class="v">{{ game.genre }}</span></div>
            <div class="stat-row"><span class="k"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 5a2 2 0 0 1 2-2h9l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z" /></svg>Taille</span><span class="v">{{ game.sizeGb ? game.sizeGb + " Go" : "—" }}</span></div>
          </div>
          <div v-if="game.achievements" class="ach-progress">
            <div class="top"><span>Succès</span><span><b>{{ game.achievements.unlocked }} / {{ game.achievements.total }} ({{ achPct }}%)</b></span></div>
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

/* Engrenage « Options du jeu » → menu (désinstallation). Ouvert vers le haut, comme le menu Jouer. */
.settings-wrap { position: relative; }
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
.shots { display: flex; gap: 14px; overflow-x: auto; padding-bottom: 8px; }
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
  width: 44px; height: 44px; border-radius: 12px; flex: none; display: grid; place-items: center;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--accent);
}
.ach-badge.locked { color: var(--text-faint); opacity: 0.6; }
.ach-badge svg { width: 22px; height: 22px; }
.ach-info { flex: 1; min-width: 0; }
.ach-name { font-size: 14px; font-weight: 600; }
.ach-pct { font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); }

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
