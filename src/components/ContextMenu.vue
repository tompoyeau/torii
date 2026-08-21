<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useContextMenu } from "../composables/useContextMenu";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { useTorii } from "../composables/useTorii";
import { openInstallDir, uninstallGame } from "../lib/tauri";

const { ctx, closeContext } = useContextMenu();
const { setFavorite, setHidden, removeManual, launchOrInstall } = useLibrary();
const { openGame, openEditGame } = useUi();
const { connected: toriiConnected, isMuted, setMuted } = useTorii();

const menuEl = ref<HTMLElement | null>(null);
// Position réellement appliquée (le clic sert d'ancrage, puis on rabat le menu
// dans le viewport une fois sa taille connue).
const pos = ref({ left: 0, top: 0 });

/** Rabat le menu à l'intérieur de la fenêtre à partir du point de clic. */
async function place() {
  await nextTick();
  const el = menuEl.value;
  if (!el) return;
  const { offsetWidth: w, offsetHeight: h } = el;
  const margin = 8;
  const left = Math.min(ctx.x, window.innerWidth - w - margin);
  const top = Math.min(ctx.y, window.innerHeight - h - margin);
  pos.value = { left: Math.max(margin, left), top: Math.max(margin, top) };
}

watch(
  () => ctx.open,
  (open) => {
    if (open) place();
  },
);

const game = computed(() => ctx.game);
const isManual = computed(() => game.value?.platform === "manual");

function onPlay() {
  if (game.value) launchOrInstall(game.value);
  closeContext();
}
function onFavorite() {
  if (game.value) setFavorite(game.value.id, !game.value.favorite);
  closeContext();
}
function onHide() {
  if (game.value) setHidden(game.value.id, !game.value.hidden);
  closeContext();
}
async function onUninstall() {
  if (!game.value) return;
  const g = game.value;
  closeContext();
  if (g.platform === "manual") await removeManual(g.id);
  else await uninstallGame(g);
}
function onDetail() {
  if (game.value) openGame(game.value.id);
  closeContext();
}
/** N'annonce plus (ou de nouveau) ce jeu aux amis Torii. */
function onToggleMuted() {
  if (game.value) void setMuted(game.value.id, !isMuted(game.value.id));
  closeContext();
}

/** Jeu ajouté à la main : ouvre la modale pré-remplie pour corriger ses informations. */
function onEdit() {
  if (game.value) openEditGame(game.value.id);
  closeContext();
}
function onOpenFolder() {
  if (game.value?.installDir) openInstallDir(game.value.installDir);
  closeContext();
}

// Fermeture au clavier (Échap) ou au défilement de la page.
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && ctx.open) closeContext();
}
function onScroll() {
  if (ctx.open) closeContext();
}
onMounted(() => {
  document.addEventListener("keydown", onKey);
  window.addEventListener("scroll", onScroll, true);
});
onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKey);
  window.removeEventListener("scroll", onScroll, true);
});
</script>

<template>
  <div v-if="ctx.open && game" class="ctx-backdrop" @click="closeContext" @contextmenu.prevent="closeContext">
    <div
      ref="menuEl"
      class="ctx-menu"
      :style="{ left: pos.left + 'px', top: pos.top + 'px' }"
      @click.stop
      @contextmenu.prevent.stop
    >
      <div class="ctx-title">{{ game.title }}</div>
      <button class="ctx-item" @click="onPlay">
        <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
        <span>Jouer</span>
      </button>
      <button class="ctx-item" @click="onDetail">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 5a2 2 0 0 1 2-2h9l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2Z" /><path d="M9 9h6M9 13h6M9 17h3" /></svg>
        <span>Voir la fiche</span>
      </button>
      <button v-if="game.installed && game.installDir" class="ctx-item" @click="onOpenFolder">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" /></svg>
        <span>Ouvrir l'emplacement du fichier</span>
      </button>
      <div class="ctx-sep" />
      <button class="ctx-item" @click="onFavorite">
        <svg viewBox="0 0 24 24" :fill="game.favorite ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 4.5l2.3 4.7 5.2.8-3.8 3.7.9 5.1L12 16.9l-4.6 2.4.9-5.1L4.5 10l5.2-.8z" /></svg>
        <span>{{ game.favorite ? "Retirer des favoris" : "Ajouter aux favoris" }}</span>
      </button>
      <button class="ctx-item" @click="onHide">
        <svg v-if="game.hidden" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" /><circle cx="12" cy="12" r="3" /></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 3l18 18M10.6 10.7a2 2 0 0 0 2.8 2.8" /><path d="M9.4 5.2A9.3 9.3 0 0 1 12 5c5 0 9 4.5 9 7a12 12 0 0 1-2.2 3M6.1 6.2A12.7 12.7 0 0 0 3 12c0 2.5 4 7 9 7a9.4 9.4 0 0 0 3.6-.7" /></svg>
        <span>{{ game.hidden ? "Réafficher" : "Masquer ce jeu" }}</span>
      </button>
      <template v-if="isManual || game.installed">
        <button v-if="toriiConnected" class="ctx-item" @click="onToggleMuted">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 3l18 18" /><path d="M9 8v8a1 1 0 0 0 1.6.8L14 14" /><path d="M14 10V6.2a1 1 0 0 0-1.6-.8L10 7" /><path d="M18 8a5 5 0 0 1 .8 5.5" /></svg>
          <span>{{ game && isMuted(game.id) ? "Diffuser ce jeu aux amis" : "Ne pas diffuser ce jeu" }}</span>
        </button>
        <div class="ctx-sep" />
        <button v-if="isManual" class="ctx-item" @click="onEdit">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 20h4L19 9a2.1 2.1 0 0 0-3-3L5 17Z" /><path d="M14.5 7.5 16.5 9.5" /></svg>
          <span>Modifier les informations</span>
        </button>
        <button class="ctx-item danger" @click="onUninstall">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m2 0v14a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6M10 11v6M14 11v6" /></svg>
          <span>{{ isManual ? "Retirer de la bibliothèque" : "Désinstaller" }}</span>
        </button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.ctx-backdrop { position: fixed; inset: 0; z-index: 250; }
.ctx-menu {
  position: fixed; min-width: 210px; max-width: 260px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px;
  box-shadow: var(--shadow-hero); padding: 6px; display: flex; flex-direction: column; gap: 1px;
}
.ctx-title {
  font-size: 12.5px; font-weight: 700; color: var(--text); padding: 7px 10px 6px;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; letter-spacing: -0.01em;
}
.ctx-sep { height: 1px; background: var(--border); margin: 4px 6px; }
.ctx-item {
  display: flex; align-items: center; gap: 11px; padding: 9px 10px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13.5px; text-align: left;
  width: 100%; cursor: pointer;
}
.ctx-item svg { width: 16px; height: 16px; flex: none; color: var(--text-dim); }
.ctx-item:hover { background: var(--surface-2); }
.ctx-item.danger { color: #ff6b6b; }
.ctx-item.danger svg { color: #ff6b6b; }
.ctx-item.danger:hover { background: color-mix(in srgb, #ff6b6b 15%, transparent); }
</style>
