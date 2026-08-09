<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useFriendsCommon } from "../composables/useFriendsCommon";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { gradientFor } from "../lib/covers";
import GameCard from "./GameCard.vue";
import type { CommonGame, FriendLib, Game } from "../types";

const {
  friends, readable, privateCount, shownGames, selected,
  loading, loaded, steamConnected, refresh, toggleFriend, clearSelection, isSelected,
} = useFriendsCommon();
const { games: libGames } = useLibrary();
const { openSettings, openGame } = useUi();

onMounted(() => {
  if (!loaded.value) void refresh();
});

/** Index de la bibliothèque : « steam:appid » → jeu réel (id direct ou source fusionnée). */
const libIndex = computed(() => {
  const m = new Map<string, Game>();
  for (const g of libGames.value) {
    m.set(g.id, g);
    for (const s of g.sources ?? []) {
      if (s.launchTarget) m.set(`${s.platform}:${s.launchTarget}`, g);
    }
  }
  return m;
});

/** Jeu de la bibliothèque correspondant, ou une carte synthétique (repli). */
function gameFor(cg: CommonGame): Game {
  const found = libIndex.value.get(cg.id);
  if (found) return found;
  return {
    id: cg.id,
    title: cg.title,
    platform: "steam",
    cover: gradientFor(cg.id),
    coverUrl: cg.coverUrl ?? undefined,
    installed: false,
  } as Game;
}

/** Table steamId → ami (pour retrouver avatars/pseudos des possesseurs). */
const byId = computed(() => {
  const m = new Map<string, FriendLib>();
  for (const f of friends.value) m.set(f.steamId, f);
  return m;
});
/** Amis (objets) qui possèdent un jeu donné. */
function owners(g: CommonGame): FriendLib[] {
  return g.owners.map((id) => byId.value.get(id)).filter((f): f is FriendLib => !!f);
}

const failed = ref(new Set<string>());
function onImgError(url?: string | null) {
  if (url) failed.value = new Set(failed.value).add(url);
}
function avatarOk(url: string): boolean {
  return !!url && !failed.value.has(url);
}
function initials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}

const selCount = computed(() => selected.value.size);
</script>

<template>
  <div class="common">
    <div class="sec-head">
      <h2>Jeux en commun</h2>
      <span class="n">{{ shownGames.length }} jeu{{ shownGames.length > 1 ? "x" : "" }}</span>
      <span v-if="loading" class="spin" title="Actualisation…" />
      <span class="spacer" />
      <button class="chip refresh" :disabled="loading" title="Recalculer" @click="refresh(true)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
        Actualiser
      </button>
    </div>

    <!-- Steam non connecté -->
    <div v-if="loaded && !steamConnected" class="empty">
      <p>Connecte ton compte Steam pour voir les jeux que tu partages avec tes amis.</p>
      <button class="btn-connect" @click="openSettings()">Ouvrir les réglages</button>
    </div>

    <!-- Chargement initial -->
    <div v-else-if="!loaded && loading" class="empty">
      <span class="spin big" />
      <p>Analyse des bibliothèques de tes amis…</p>
      <p class="dim">Ça peut prendre quelques secondes la première fois.</p>
    </div>

    <!-- Aucun ami lisible -->
    <div v-else-if="loaded && !readable.length" class="empty">
      <p>Impossible de lire la bibliothèque de tes amis.</p>
      <p class="dim">Leurs profils Steam sont peut-être privés (« Détails des jeux »).</p>
    </div>

    <template v-else>
      <!-- Sélecteur multi-amis -->
      <div class="picker">
        <button class="fchip" :class="{ on: selCount === 0 }" @click="clearSelection()">
          <span class="all">Tous</span>
        </button>
        <button
          v-for="f in readable"
          :key="f.steamId"
          class="fchip"
          :class="{ on: isSelected(f.steamId) }"
          :title="`${f.name} · ${f.commonCount} en commun`"
          @click="toggleFriend(f.steamId)"
        >
          <span class="av">
            <img v-if="avatarOk(f.avatarUrl)" :src="f.avatarUrl" alt="" loading="lazy" @error="onImgError(f.avatarUrl)" />
            <span v-else class="av-fb">{{ initials(f.name) }}</span>
          </span>
          <span class="fname">{{ f.name }}</span>
          <span class="fcount">{{ f.commonCount }}</span>
        </button>
        <span v-if="privateCount" class="priv" :title="`${privateCount} ami(s) au profil privé`">
          🔒 {{ privateCount }} privé{{ privateCount > 1 ? "s" : "" }}
        </span>
      </div>

      <p class="hint">
        <template v-if="selCount === 0">Tes jeux, triés par nombre d'amis qui les possèdent aussi.</template>
        <template v-else>Jeux que <strong>vous possédez tous</strong> ({{ selCount }} ami{{ selCount > 1 ? "s" : "" }} + toi).</template>
      </p>

      <div v-if="!shownGames.length" class="empty small">
        <p>Aucun jeu en commun avec cette sélection.</p>
        <button class="btn-ghost" @click="clearSelection()">Réinitialiser</button>
      </div>

      <div v-else class="grid">
        <div v-for="g in shownGames" :key="g.id" class="cell">
          <GameCard :game="gameFor(g)" @open="openGame(gameFor(g).id)" />
          <div class="note" :title="owners(g).map((o) => o.name).join(', ')">
            <span class="avs">
              <span v-for="o in owners(g).slice(0, 5)" :key="o.steamId" class="oav">
                <img v-if="avatarOk(o.avatarUrl)" :src="o.avatarUrl" alt="" loading="lazy" @error="onImgError(o.avatarUrl)" />
                <span v-else class="oav-fb">{{ initials(o.name) }}</span>
              </span>
              <span v-if="g.owners.length > 5" class="more">+{{ g.owners.length - 5 }}</span>
            </span>
            <span class="ncount">{{ g.owners.length }} ami{{ g.owners.length > 1 ? "s" : "" }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.common { min-width: 0; }
.sec-head { display: flex; align-items: center; gap: 12px; margin-bottom: 18px; }
.sec-head h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.sec-head .n { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
.sec-head .spacer { flex: 1; }
.chip.refresh {
  display: inline-flex; align-items: center; gap: 6px; padding: 6px 13px; border-radius: 99px; font-size: 12.5px;
  color: var(--text-dim); background: var(--surface); border: 1px solid var(--border);
}
.chip.refresh svg { width: 14px; height: 14px; }
.chip.refresh:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); }
.chip.refresh:disabled { opacity: 0.6; }

.spin { width: 14px; height: 14px; border-radius: 50%; border: 2px solid var(--border-strong); border-top-color: var(--accent); animation: spin 0.7s linear infinite; display: inline-block; }
.spin.big { width: 26px; height: 26px; border-width: 3px; margin-bottom: 14px; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Sélecteur d'amis */
.picker { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 12px; }
.fchip {
  display: inline-flex; align-items: center; gap: 7px; padding: 5px 11px 5px 6px; border-radius: 99px;
  background: var(--surface); border: 1px solid var(--border); color: var(--text-dim); cursor: pointer;
  font-size: 13px; transition: all 0.14s;
}
.fchip:hover { border-color: var(--border-strong); color: var(--text); }
.fchip.on { background: color-mix(in srgb, var(--accent) 16%, transparent); border-color: var(--accent); color: var(--text); }
.fchip .all { padding: 0 6px; font-weight: 600; }
.fchip .av { width: 24px; height: 24px; flex: none; border-radius: 50%; overflow: hidden; }
.fchip .av img, .fchip .av-fb { width: 24px; height: 24px; border-radius: 50%; object-fit: cover; display: grid; place-items: center; }
.fchip .av-fb { background: linear-gradient(140deg, #6b6f7a, #3a3d47); color: #fff; font-size: 10px; font-weight: 700; font-family: var(--mono); }
.fchip .fname { max-width: 120px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.fchip .fcount { font-family: var(--mono); font-size: 11px; opacity: 0.7; background: var(--surface-2); padding: 1px 6px; border-radius: 99px; }
.fchip.on .fcount { background: color-mix(in srgb, var(--accent) 25%, transparent); opacity: 1; }
.priv { font-size: 12px; color: var(--text-faint); margin-left: 4px; }

.hint { font-size: 13px; color: var(--text-faint); margin: 0 0 18px; }
.hint strong { color: var(--text-dim); font-weight: 600; }

/* Grille : cartes identiques à la bibliothèque + note des amis dessous */
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(155px, 1fr)); gap: 22px 16px; }
.cell { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
.note {
  display: flex; align-items: center; gap: 8px; padding: 0 2px;
}
.avs { display: flex; align-items: center; }
.oav { width: 20px; height: 20px; border-radius: 50%; overflow: hidden; margin-right: -6px; border: 1.5px solid var(--bg); }
.oav img, .oav-fb { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; display: grid; place-items: center; }
.oav-fb { background: linear-gradient(140deg, #6b6f7a, #3a3d47); color: #fff; font-size: 8px; font-weight: 700; font-family: var(--mono); }
.more { margin-left: 10px; font-family: var(--mono); font-size: 11px; color: var(--text-faint); }
.ncount { font-size: 11.5px; color: var(--text-faint); font-family: var(--mono); }

.empty { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 70px 0; text-align: center; color: var(--text-faint); font-size: 14px; }
.empty.small { padding: 44px 0; }
.empty p { margin: 3px 0; }
.empty .dim { font-size: 12.5px; opacity: 0.8; }
.btn-connect { margin-top: 14px; padding: 9px 18px; border-radius: 11px; background: var(--accent); color: var(--accent-ink); border: none; font-weight: 600; font-size: 13.5px; }
.btn-connect:hover { background: var(--accent-hover); }
.btn-ghost { margin-top: 12px; padding: 7px 15px; border-radius: 10px; background: var(--surface); border: 1px solid var(--border); color: var(--text-dim); font-size: 13px; cursor: pointer; }
.btn-ghost:hover { color: var(--text); border-color: var(--border-strong); }
</style>
