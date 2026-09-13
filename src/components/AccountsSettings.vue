<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { t } from "../i18n";
import { useUi } from "../composables/useUi";
import { useLibrary } from "../composables/useLibrary";
import { useTorii } from "../composables/useTorii";
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
const { reconcilierSteam } = useTorii();

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

/**
 * ⚠️ `computed` : les textes des cartes doivent suivre la langue. Les clés et les
 * fonctions de connexion, elles, ne changent pas — `state` reste indexé par `key`.
 */
const ACCOUNTS = computed<AccountDef[]>(() => [
  {
    key: "steam",
    name: "Steam",
    short: "Steam",
    color: "var(--steam)",
    hint: t("comptes.launchers.aideSteam"),
    syncedHint: t("comptes.launchers.synchronisee"),
    connectLabel: t("comptes.launchers.seConnecter", { nom: "Steam" }),
    connect: connectSteam,
    disconnect: disconnectSteam,
    isConnected: (s) => s.steamConnected,
  },
  {
    key: "epic",
    name: "Epic Games",
    short: "Epic",
    color: "var(--epic)",
    hint: t("comptes.launchers.aide", { nom: "Epic", fenetre: t("comptes.launchers.fenetreEpic") }),
    syncedHint: t("comptes.launchers.synchroniseeDe", { nom: "Epic" }),
    connectLabel: t("comptes.launchers.seConnecter", { nom: "Epic" }),
    connect: connectEpic,
    disconnect: disconnectEpic,
    isConnected: (s) => s.epicConnected,
  },
  {
    key: "ea",
    name: "EA",
    short: "EA",
    color: "var(--ea)",
    hint: t("comptes.launchers.aide", { nom: "EA", fenetre: t("comptes.launchers.fenetreEa") }),
    syncedHint: t("comptes.launchers.synchroniseeDe", { nom: "EA" }),
    connectLabel: t("comptes.launchers.seConnecter", { nom: "EA" }),
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
    hint: t("comptes.launchers.aideBattlenet"),
    syncedHint: t("comptes.launchers.synchroniseeDe", { nom: "Battle.net" }),
    connectLabel: t("comptes.launchers.seConnecter", { nom: "Battle.net" }),
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
    hint: t("comptes.launchers.aide", { nom: "GOG", fenetre: t("comptes.launchers.fenetreGog") }),
    syncedHint: t("comptes.launchers.synchroniseeDe", { nom: "GOG" }),
    connectLabel: t("comptes.launchers.seConnecter", { nom: "GOG" }),
    connect: connectGog,
    disconnect: disconnectGog,
    isConnected: (s) => s.gogConnected,
  },
]);

/** État d'affichage d'une carte. */
interface AccountState {
  connected: boolean;
  busy: boolean;
  message: string;
}

const state = reactive<Record<string, AccountState>>(
  Object.fromEntries(
    ACCOUNTS.value.map((a) => [a.key, { connected: false, busy: false, message: "" }]),
  ),
);

const showAdvanced = ref(false);
const steamKey = ref("");

function applySettings(s: Settings) {
  for (const a of ACCOUNTS.value) state[a.key].connected = a.isConnected(s);
}

onMounted(async () => {
  const s = await getSettings();
  if (s) applySettings(s);
});

async function onConnect(a: AccountDef) {
  const st = state[a.key];
  st.busy = true;
  st.message = t("comptes.launchers.connexion", { nom: a.short });
  try {
    const s = await a.connect();
    applySettings(s);
    // Steam vient d'être connecté : si un compte Torii est déjà ouvert, le SteamID doit
    // y remonter maintenant, pas au prochain démarrage.
    if (a.key === "steam") void reconcilierSteam();
    st.message = t("comptes.launchers.connecte", { nom: a.short });
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
  state[a.key].message = t("comptes.launchers.resynchronisation");
  reload();
  closeSettings();
}

async function onDisconnect(a: AccountDef) {
  const st = state[a.key];
  st.busy = true;
  const s = await a.disconnect();
  applySettings(s);
  if (a.key === "steam") steamKey.value = "";
  st.message = t("comptes.launchers.deconnecte", { nom: a.short });
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
    st.message = t("comptes.launchers.cleEnregistree");
    reload();
  } else {
    st.message = t("comptes.launchers.horsApplication");
  }
}
</script>

<template>
  <div>
    <section class="group">
      <div class="group-label">{{ t("comptes.launchers.groupe") }}</div>

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
        :resync-busy-label="a.resyncReconnects ? t('comptes.launchers.actualisation') : undefined"
        :message="state[a.key].message"
        @connect="onConnect(a)"
        @resync="onResync(a)"
        @disconnect="onDisconnect(a)"
      >
        <!-- Steam : connexion par clé API, chemin avancé replié. -->
        <template v-if="a.key === 'steam'" #extra>
          <button class="advanced-toggle" @click="showAdvanced = !showAdvanced">
            {{ showAdvanced ? "▾" : "▸" }} {{ t("comptes.launchers.cleAvancee") }}
          </button>
          <div v-if="showAdvanced" class="row advanced">
            <input
              v-model="steamKey"
              type="password"
              :placeholder="t('comptes.launchers.cleChamp')"
              autocomplete="off"
              @keyup.enter="onSaveKey"
            />
            <button
              class="btn-secondary"
              :disabled="state.steam.busy || !steamKey.trim()"
              @click="onSaveKey"
            >
              {{ t("comptes.launchers.enregistrer") }}
            </button>
          </div>
        </template>
      </LauncherAccount>
    </section>

    <p class="footnote">{{ t("comptes.launchers.stockage") }}</p>
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
.row input:focus { border-color: var(--border-strong); }
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
