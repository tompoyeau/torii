<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useUi } from "../composables/useUi";
import { useTheme } from "../composables/useTheme";
import { useLibrary } from "../composables/useLibrary";
import { useFriends } from "../composables/useFriends";
import { openExternal, steamMe } from "../lib/tauri";
import type { SteamProfile } from "../types";
import ModeSwitch from "./ModeSwitch.vue";

const { section, query, openAddGame, showFriends, openSettings, settingsOpen } = useUi();
const { toggle: toggleTheme } = useTheme();
const { loading, enriching, enrichProgress, reload } = useLibrary();
const { activeCount: friendsOnline, refresh: refreshFriends } = useFriends();

// Identité de l'utilisateur (Steam) affichée dans l'en-tête. Masquée si non connecté.
const me = ref<SteamProfile | null>(null);
onMounted(async () => {
  me.value = await steamMe();
  // Charge les amis dès le démarrage pour alimenter le badge de l'en-tête.
  refreshFriends();
});
function openMe() {
  if (me.value?.profileUrl) openExternal(me.value.profileUrl);
}
</script>

<template>
  <div class="topbar">
    <label v-if="section !== 'store'" class="search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7" /><path d="m20 20-3-3" /></svg>
      <input v-model="query" type="text" placeholder="Rechercher dans la bibliothèque…" autocomplete="off" />
    </label>
    <div class="topbar-spacer" />
    <span v-if="loading" class="enrich-pill">
      <span class="spinner" />Actualisation…
    </span>
    <span v-else-if="enriching" class="enrich-pill">
      <span class="spinner" />
      <template v-if="enrichProgress">Métadonnées {{ enrichProgress.done }}/{{ enrichProgress.total }}</template>
      <template v-else>Métadonnées…</template>
    </span>
    <button class="add-btn" title="Ajouter un jeu manuellement" @click="openAddGame">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
      <span>Ajouter</span>
    </button>
    <ModeSwitch />
    <button class="icon-btn friends-btn" :class="{ active: section === 'friends' }" title="Amis" @click="showFriends">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 5.2a3 3 0 0 1 0 5.6M17.5 20a5.5 5.5 0 0 0-3-4.9" /></svg>
      <span v-if="friendsOnline" class="friends-badge">{{ friendsOnline }}</span>
    </button>
    <button class="icon-btn" title="Resynchroniser" :disabled="loading" @click="reload()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
    </button>
    <button class="icon-btn" title="Thème clair / sombre" @click="toggleTheme">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M20 14.5A8 8 0 1 1 9.5 4 6.5 6.5 0 0 0 20 14.5Z" /></svg>
    </button>
    <button class="icon-btn" :class="{ active: settingsOpen }" title="Paramètres" @click="openSettings()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="3.1" /><path d="M19.4 13a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5v.2a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1.1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H2a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1.1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H8a1.6 1.6 0 0 0 1-1.5V2a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V8a1.6 1.6 0 0 0 1.5 1h.2a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" /></svg>
    </button>
    <button v-if="me" class="me" :title="`${me.name} — voir le profil Steam`" @click="openMe">
      <img v-if="me.avatarUrl" class="me-avatar" :src="me.avatarUrl" :alt="me.name" />
      <span class="me-name">{{ me.name }}</span>
    </button>
  </div>
</template>

<style scoped>
.topbar {
  position: sticky; top: 0; z-index: 20; display: flex; align-items: center; gap: 14px;
  padding: 18px 0 16px;
  background: linear-gradient(var(--bg) 60%, transparent); backdrop-filter: blur(6px);
}
.search { flex: 1; max-width: 420px; position: relative; display: flex; align-items: center; }
.search svg { position: absolute; left: 13px; width: 16px; height: 16px; color: var(--text-faint); }
.search input {
  width: 100%; padding: 10px 14px 10px 38px; background: var(--surface);
  border: 1px solid var(--border); border-radius: 11px; color: var(--text);
  font-size: 13.5px; font-family: inherit;
}
.search input::placeholder { color: var(--text-faint); }
.search input:focus { outline: none; border-color: var(--border-strong); background: var(--surface-2); }
.topbar-spacer { flex: 1; }
.enrich-pill {
  display: inline-flex; align-items: center; gap: 8px; padding: 6px 12px; border-radius: 99px;
  background: var(--accent-soft); color: var(--accent); font-size: 12px; font-weight: 600;
  font-family: var(--mono);
}
.spinner {
  width: 12px; height: 12px; border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-top-color: var(--accent); animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.icon-btn {
  width: 38px; height: 38px; border-radius: 11px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text-dim); display: grid; place-items: center;
  transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover { color: var(--text); border-color: var(--border-strong); }
.icon-btn svg { width: 17px; height: 17px; }
.icon-btn.active {
  color: var(--accent); border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  background: var(--accent-soft);
}
.friends-btn { position: relative; }
.friends-badge {
  position: absolute; top: -5px; right: -5px; min-width: 17px; height: 17px; padding: 0 4px;
  border-radius: 99px; background: #4bbe6b; color: #08210f; font-size: 10px; font-weight: 700;
  font-family: var(--mono); display: grid; place-items: center;
  border: 2px solid var(--bg); box-sizing: border-box;
}
.add-btn {
  display: inline-flex; align-items: center; gap: 7px; height: 38px; padding: 0 15px 0 12px;
  border-radius: 11px; border: 1px solid var(--border); background: var(--surface); color: var(--text-dim);
  font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.add-btn:hover { color: var(--text); border-color: var(--border-strong); }
.add-btn svg { width: 16px; height: 16px; }
.me {
  display: inline-flex; align-items: center; gap: 9px; height: 38px; padding: 0 12px 0 5px;
  border-radius: 99px; border: 1px solid var(--border); background: var(--surface);
  color: var(--text-dim); font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.me:hover { color: var(--text); border-color: var(--border-strong); }
.me-avatar {
  width: 28px; height: 28px; border-radius: 50%; flex: none; object-fit: cover;
  background: var(--surface-3);
}
.me-name { max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
