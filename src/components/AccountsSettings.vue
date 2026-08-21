<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { useUi } from "../composables/useUi";
import { useLibrary } from "../composables/useLibrary";
import {
  connectBattlenet,
  connectEa,
  connectEpic,
  connectGog,
  connectSteam,
  disconnectBattlenet,
  disconnectEa,
  disconnectEpic,
  disconnectGog,
  disconnectSteam,
  getSettings,
  setSteamKey,
} from "../lib/tauri";
import type { Settings } from "../types";
import LauncherAccount from "./LauncherAccount.vue";

const { closeSettings } = useUi();
const { reload } = useLibrary();

/** Un launcher connectable : tout ce qui change d'une carte de compte à l'autre. */
interface AccountDef {
  key: string;
  /** Nom affiché sur la carte. */
  name: string;
  /** Nom court utilisé dans les messages d'état. */
  short: string;
  /** Couleur de la pastille (variable CSS de la plateforme). */
  color: string;
  hint: string;
  syncedHint: string;
  connectLabel: string;
  /**
   * EA et Battle.net n'ont pas de rafraîchissement silencieux : leur bibliothèque est
   * un instantané pris à la connexion, donc « Resynchroniser » repasse par le flux de
   * connexion (qui ne redemande le login que si la session web a expiré).
   */
  resyncReconnects?: boolean;
  connect: () => Promise<Settings>;
  disconnect: () => Promise<Settings>;
  /** Lit l'état de ce launcher dans la réponse du backend. */
  isConnected: (s: Settings) => boolean;
}

const ACCOUNTS: AccountDef[] = [
  {
    key: "steam",
    name: "Steam",
    short: "Steam",
    color: "var(--steam)",
    hint:
      "Connecte-toi à ton compte Steam pour importer toute ta bibliothèque " +
      "(installés ou non) et ta wishlist. Aucune clé, aucun mot de passe transmis à Torii — " +
      "tu te connectes dans la fenêtre officielle de Steam.",
    syncedHint: "Bibliothèque synchronisée.",
    connectLabel: "Se connecter avec Steam",
    connect: connectSteam,
    disconnect: disconnectSteam,
    isConnected: (s) => s.steamConnected,
  },
  {
    key: "epic",
    name: "Epic Games",
    short: "Epic",
    color: "var(--epic)",
    hint:
      "Connecte-toi à ton compte Epic pour importer toute ta bibliothèque " +
      "(installés ou non). Aucun mot de passe transmis à Torii — tu te " +
      "connectes dans la fenêtre officielle d'Epic Games.",
    syncedHint: "Bibliothèque Epic synchronisée.",
    connectLabel: "Se connecter avec Epic",
    connect: connectEpic,
    disconnect: disconnectEpic,
    isConnected: (s) => s.epicConnected,
  },
  {
    key: "ea",
    name: "EA",
    short: "EA",
    color: "var(--ea)",
    hint:
      "Connecte-toi à ton compte EA pour importer toute ta bibliothèque " +
      "(installés ou non). Aucun mot de passe transmis à Torii — tu te " +
      "connectes dans la fenêtre officielle d'EA.",
    syncedHint: "Bibliothèque EA synchronisée.",
    connectLabel: "Se connecter avec EA",
    resyncReconnects: true,
    connect: connectEa,
    disconnect: disconnectEa,
    isConnected: (s) => s.eaConnected,
  },
  {
    key: "battlenet",
    name: "Battle.net",
    short: "Battle.net",
    color: "var(--battlenet)",
    hint:
      "Connecte-toi à ton compte Battle.net pour importer ta bibliothèque " +
      "Blizzard. Aucun mot de passe transmis à Torii — tu te connectes dans la " +
      "fenêtre officielle de Battle.net.",
    syncedHint: "Bibliothèque Battle.net synchronisée.",
    connectLabel: "Se connecter avec Battle.net",
    resyncReconnects: true,
    connect: connectBattlenet,
    disconnect: disconnectBattlenet,
    isConnected: (s) => s.battlenetConnected,
  },
  {
    key: "gog",
    name: "GOG",
    short: "GOG",
    color: "var(--gog)",
    hint:
      "Connecte-toi à ton compte GOG pour importer toute ta bibliothèque " +
      "(installés ou non). Aucun mot de passe transmis à Torii — tu te " +
      "connectes dans la fenêtre officielle de GOG.",
    syncedHint: "Bibliothèque GOG synchronisée.",
    connectLabel: "Se connecter avec GOG",
    connect: connectGog,
    disconnect: disconnectGog,
    isConnected: (s) => s.gogConnected,
  },
];

/** État d'affichage d'une carte. */
interface AccountState {
  connected: boolean;
  busy: boolean;
  message: string;
}

const state = reactive<Record<string, AccountState>>(
  Object.fromEntries(
    ACCOUNTS.map((a) => [a.key, { connected: false, busy: false, message: "" }]),
  ),
);

const showAdvanced = ref(false);
const steamKey = ref("");

function applySettings(s: Settings) {
  for (const a of ACCOUNTS) state[a.key].connected = a.isConnected(s);
}

onMounted(async () => {
  const s = await getSettings();
  if (s) applySettings(s);
});

async function onConnect(a: AccountDef) {
  const st = state[a.key];
  st.busy = true;
  st.message = `Connexion… une fenêtre ${a.short} s'est ouverte, connecte-toi.`;
  try {
    const s = await a.connect();
    applySettings(s);
    st.message = `Compte ${a.short} connecté — actualisation de la bibliothèque…`;
    reload();
    // On ferme pour laisser voir la progression (barre du haut).
    closeSettings();
  } catch (err) {
    st.message = String(err);
  } finally {
    st.busy = false;
  }
}

function onResync(a: AccountDef) {
  if (a.resyncReconnects) {
    void onConnect(a);
    return;
  }
  state[a.key].message = "Resynchronisation…";
  reload();
  closeSettings();
}

async function onDisconnect(a: AccountDef) {
  const st = state[a.key];
  st.busy = true;
  const s = await a.disconnect();
  applySettings(s);
  if (a.key === "steam") steamKey.value = "";
  st.message = `Compte ${a.short} déconnecté.`;
  st.busy = false;
  reload();
}

/** Chemin avancé Steam : enregistre (ou efface) la clé API. */
async function onSaveKey() {
  const st = state.steam;
  st.busy = true;
  const s = await setSteamKey(steamKey.value);
  st.busy = false;
  if (s) {
    applySettings(s);
    steamKey.value = "";
    st.message = "Clé enregistrée — actualisation…";
    reload();
  } else {
    st.message = "Indisponible hors de l'application Torii.";
  }
}
</script>

<template>
  <div>
    <section class="group">
      <div class="group-label">Comptes — jeux possédés (installés ou non)</div>

      <LauncherAccount
        v-for="a in ACCOUNTS"
        :key="a.key"
        :name="a.name"
        :color="a.color"
        :connected="state[a.key].connected"
        :busy="state[a.key].busy"
        :hint="a.hint"
        :synced-hint="a.syncedHint"
        :connect-label="a.connectLabel"
        :resync-busy-label="a.resyncReconnects ? 'Actualisation…' : undefined"
        :message="state[a.key].message"
        @connect="onConnect(a)"
        @resync="onResync(a)"
        @disconnect="onDisconnect(a)"
      >
        <!-- Steam : connexion par clé API, chemin avancé replié. -->
        <template v-if="a.key === 'steam'" #extra>
          <button class="advanced-toggle" @click="showAdvanced = !showAdvanced">
            {{ showAdvanced ? "▾" : "▸" }} Utiliser une clé API (avancé)
          </button>
          <div v-if="showAdvanced" class="row advanced">
            <input
              v-model="steamKey"
              type="password"
              placeholder="Clé API Steam (32 caractères)"
              autocomplete="off"
              @keyup.enter="onSaveKey"
            />
            <button
              class="btn-secondary"
              :disabled="state.steam.busy || !steamKey.trim()"
              @click="onSaveKey"
            >
              Enregistrer
            </button>
          </div>
        </template>
      </LauncherAccount>
    </section>

    <p class="footnote">Les identifiants sont stockés localement, sur cette machine uniquement.</p>
  </div>
</template>

<style scoped>
.group-label {
  font-size: 11px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint);
  font-weight: 700; margin-bottom: 14px;
}
/* Styles du chemin avancé Steam : le contenu de slot est compilé dans la portée du
   parent, il est donc habillé ici et non dans LauncherAccount. */
.row { display: flex; gap: 7px; flex-wrap: wrap; align-items: center; }
.row.advanced { margin-top: 7px; }
.row input {
  flex: 1; min-width: 200px; padding: 8px 12px; border-radius: 9px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text); font-size: 13px; font-family: var(--mono);
}
.row input:focus { outline: none; border-color: var(--border-strong); }
.btn-secondary {
  padding: 8px 13px; border-radius: 9px; border: 1px solid var(--border); background: var(--surface);
  color: var(--text-dim); font-weight: 600; font-size: 12.5px;
}
.btn-secondary:disabled { opacity: 0.5; cursor: default; }
.advanced-toggle {
  margin-top: 9px; background: none; border: none; color: var(--text-faint);
  font-size: 11.5px; font-family: var(--mono); padding: 2px 0;
}
.advanced-toggle:hover { color: var(--text-dim); }
.footnote { font-size: 11.5px; color: var(--text-faint); text-align: center; margin: 16px 0 4px; }
</style>
