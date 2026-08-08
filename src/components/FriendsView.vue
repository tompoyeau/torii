<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useFriends } from "../composables/useFriends";
import { useUi } from "../composables/useUi";
import { openExternal } from "../lib/tauri";
import type { Friend } from "../types";

const { loading, loaded, steamConnected, inGame, online, offline, activeCount, refresh } = useFriends();
const { openSettings } = useUi();

/** Rafraîchit la présence en direct pendant que la vue est ouverte. */
let timer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  void refresh();
  timer = setInterval(() => void refresh(), 60_000);
});
onBeforeUnmount(() => clearInterval(timer));

const groups = computed(() => [
  { key: "in-game", label: "En jeu", list: inGame.value },
  { key: "online", label: "En ligne", list: online.value },
  { key: "offline", label: "Hors ligne", list: offline.value },
]);

/** Libellé lisible d'un statut. */
function stateLabel(f: Friend): string {
  if (f.state === "in-game") return f.gameName ? `Joue à ${f.gameName}` : "En jeu";
  switch (f.state) {
    case "online": return "En ligne";
    case "away": return "Absent";
    case "busy": return "Occupé";
    case "snooze": return "Inactif";
    default: return "Hors ligne";
  }
}

/** Initiales pour l'avatar de secours. */
function initials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}
const failed = ref(new Set<string>());
function avatar(f: Friend): string | null {
  return f.avatarUrl && !failed.value.has(f.avatarUrl) ? f.avatarUrl : null;
}
function onAvatarError(url: string) {
  failed.value = new Set(failed.value).add(url);
}

function openProfile(f: Friend) {
  if (f.profileUrl && f.profileUrl !== "#") openExternal(f.profileUrl);
}
</script>

<template>
  <div class="friends">
    <div class="sec-head">
      <h2>Amis</h2>
      <span class="n">{{ activeCount }} en ligne</span>
      <span v-if="loading" class="spin" title="Actualisation…" />
      <span class="spacer" />
      <button class="chip refresh" :disabled="loading" title="Actualiser" @click="refresh()">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
        Actualiser
      </button>
    </div>

    <!-- Steam non connecté -->
    <div v-if="loaded && !steamConnected" class="empty">
      <p>Connecte ton compte Steam pour voir tes amis et qui joue à quoi.</p>
      <button class="btn-connect" @click="openSettings()">Ouvrir les réglages</button>
    </div>

    <!-- Chargement initial -->
    <div v-else-if="!loaded && loading" class="empty">
      <span class="spin big" />
      <p>Récupération de tes amis…</p>
    </div>

    <!-- Aucun ami -->
    <div v-else-if="loaded && !inGame.length && !online.length && !offline.length" class="empty">
      <p>Aucun ami à afficher.</p>
      <p class="dim">Ta liste d'amis Steam est peut-être privée.</p>
    </div>

    <template v-else>
      <section v-for="g in groups" :key="g.key" v-show="g.list.length" class="group">
        <h4 :class="g.key">{{ g.label }} <span>{{ g.list.length }}</span></h4>
        <div class="rows">
          <button
            v-for="f in g.list"
            :key="f.steamId"
            class="friend"
            :class="{ ingame: f.state === 'in-game', off: f.state === 'offline' }"
            @click="openProfile(f)"
          >
            <span class="avatar" :class="f.state">
              <img v-if="avatar(f)" :src="avatar(f)!" alt="" loading="lazy" @error="onAvatarError(f.avatarUrl)" />
              <span v-else class="avatar-fallback">{{ initials(f.name) }}</span>
              <span class="dot" :class="f.state" />
            </span>
            <span class="info">
              <span class="name">{{ f.name }}</span>
              <span class="status" :class="f.state">{{ stateLabel(f) }}</span>
            </span>
          </button>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.friends { min-width: 0; }
.sec-head { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; }
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

.group { margin-bottom: 26px; }
.group h4 {
  font-size: 12px; text-transform: uppercase; letter-spacing: 0.1em; font-weight: 700; margin: 0 0 12px;
  color: var(--text-faint); display: flex; align-items: center; gap: 8px;
}
.group h4 span { font-family: var(--mono); opacity: 0.7; }
.group h4.in-game { color: #4bbe6b; }

.rows { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 8px; }
.friend {
  display: flex; align-items: center; gap: 12px; padding: 9px 11px; border-radius: 12px; text-align: left;
  background: none; border: 1px solid transparent; color: inherit; cursor: pointer; transition: background 0.15s;
}
.friend:hover { background: var(--surface-2); }
.friend.ingame { background: color-mix(in srgb, #4bbe6b 9%, transparent); }
.friend.ingame:hover { background: color-mix(in srgb, #4bbe6b 15%, transparent); }
.friend.off { opacity: 0.55; }

.avatar {
  position: relative; width: 42px; height: 42px; flex: none; border-radius: 10px; overflow: visible;
}
.avatar img, .avatar-fallback {
  width: 42px; height: 42px; border-radius: 10px; object-fit: cover; display: grid; place-items: center;
}
.avatar-fallback {
  background: linear-gradient(140deg, #6b6f7a, #3a3d47); color: #fff; font-size: 14px; font-weight: 700;
  font-family: var(--mono);
}
.avatar img { border: 1px solid var(--border); }
/* Anneau de statut coloré */
.avatar.in-game img, .avatar.in-game .avatar-fallback { box-shadow: 0 0 0 2px #4bbe6b; }
.avatar.online img, .avatar.online .avatar-fallback { box-shadow: 0 0 0 2px #56a3d9; }
.avatar.away img, .avatar.away .avatar-fallback,
.avatar.busy img, .avatar.busy .avatar-fallback,
.avatar.snooze img, .avatar.snooze .avatar-fallback { box-shadow: 0 0 0 2px #e0a53a; }
.dot {
  position: absolute; right: -2px; bottom: -2px; width: 12px; height: 12px; border-radius: 50%;
  border: 2px solid var(--bg); background: var(--text-faint);
}
.dot.in-game { background: #4bbe6b; }
.dot.online { background: #56a3d9; }
.dot.away, .dot.busy, .dot.snooze { background: #e0a53a; }

.info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.name { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.status { font-size: 12px; color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.status.in-game { color: #4bbe6b; font-weight: 600; }

.empty { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 70px 0; text-align: center; color: var(--text-faint); font-size: 14px; }
.empty p { margin: 3px 0; }
.empty .dim { font-size: 12.5px; opacity: 0.8; }
.btn-connect {
  margin-top: 14px; padding: 9px 18px; border-radius: 11px; background: var(--accent); color: var(--accent-ink);
  border: none; font-weight: 600; font-size: 13.5px;
}
.btn-connect:hover { background: var(--accent-hover); }
</style>
