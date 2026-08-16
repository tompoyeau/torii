<script setup lang="ts">
import { onMounted, ref } from "vue";
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

const { closeSettings } = useUi();
const { reload } = useLibrary();

const steamConnected = ref(false);
const steamId = ref<string | null>(null);
const busy = ref(false);
const message = ref("");

const showAdvanced = ref(false);
const steamKey = ref("");

const gogConnected = ref(false);
const gogBusy = ref(false);
const gogMessage = ref("");

const epicConnected = ref(false);
const epicBusy = ref(false);
const epicMessage = ref("");

const eaConnected = ref(false);
const eaBusy = ref(false);
const eaMessage = ref("");

const bnetConnected = ref(false);
const bnetBusy = ref(false);
const bnetMessage = ref("");

onMounted(async () => {
  const s = await getSettings();
  if (s) {
    steamConnected.value = s.steamConnected;
    steamId.value = s.steamId ?? null;
    gogConnected.value = s.gogConnected;
    epicConnected.value = s.epicConnected;
    eaConnected.value = s.eaConnected;
    bnetConnected.value = s.battlenetConnected;
  }
});

async function onConnect() {
  busy.value = true;
  message.value = "Connexion… une fenêtre Steam s'est ouverte, connecte-toi.";
  try {
    const s = await connectSteam();
    steamConnected.value = s.steamConnected;
    steamId.value = s.steamId ?? null;
    message.value = "Compte Steam connecté — actualisation de la bibliothèque…";
    reload();
    // On ferme pour laisser voir la progression (barre du haut).
    closeSettings();
  } catch (err) {
    message.value = String(err);
  } finally {
    busy.value = false;
  }
}

function onResync() {
  message.value = "Resynchronisation…";
  reload();
  closeSettings();
}

async function onDisconnect() {
  busy.value = true;
  const s = await disconnectSteam();
  steamConnected.value = s.steamConnected;
  steamId.value = s.steamId ?? null;
  steamKey.value = "";
  message.value = "Compte Steam déconnecté.";
  busy.value = false;
  reload();
}

async function onSaveKey() {
  busy.value = true;
  const s = await setSteamKey(steamKey.value);
  busy.value = false;
  if (s) {
    steamConnected.value = s.steamConnected;
    steamId.value = s.steamId ?? null;
    steamKey.value = "";
    message.value = "Clé enregistrée — actualisation…";
    reload();
  } else {
    message.value = "Indisponible hors de l'application Tauri.";
  }
}

async function onConnectGog() {
  gogBusy.value = true;
  gogMessage.value = "Connexion… une fenêtre GOG s'est ouverte, connecte-toi.";
  try {
    const s = await connectGog();
    gogConnected.value = s.gogConnected;
    gogMessage.value = "Compte GOG connecté — actualisation de la bibliothèque…";
    reload();
    closeSettings();
  } catch (err) {
    gogMessage.value = String(err);
  } finally {
    gogBusy.value = false;
  }
}

function onResyncGog() {
  gogMessage.value = "Resynchronisation…";
  reload();
  closeSettings();
}

async function onDisconnectGog() {
  gogBusy.value = true;
  const s = await disconnectGog();
  gogConnected.value = s.gogConnected;
  gogMessage.value = "Compte GOG déconnecté.";
  gogBusy.value = false;
  reload();
}

async function onConnectEpic() {
  epicBusy.value = true;
  epicMessage.value = "Connexion… une fenêtre Epic s'est ouverte, connecte-toi.";
  try {
    const s = await connectEpic();
    epicConnected.value = s.epicConnected;
    epicMessage.value = "Compte Epic connecté — actualisation de la bibliothèque…";
    reload();
    closeSettings();
  } catch (err) {
    epicMessage.value = String(err);
  } finally {
    epicBusy.value = false;
  }
}

function onResyncEpic() {
  epicMessage.value = "Resynchronisation…";
  reload();
  closeSettings();
}

async function onDisconnectEpic() {
  epicBusy.value = true;
  const s = await disconnectEpic();
  epicConnected.value = s.epicConnected;
  epicMessage.value = "Compte Epic déconnecté.";
  epicBusy.value = false;
  reload();
}

// EA — la connexion récupère toute la bibliothèque et la met en cache. La
// « resynchronisation » relance le même flux (re-fetch ; login seulement si la
// session web a expiré) car le token EA ne se rafraîchit pas silencieusement.
async function onConnectEa() {
  eaBusy.value = true;
  eaMessage.value = "Connexion… une fenêtre EA s'est ouverte, connecte-toi.";
  try {
    const s = await connectEa();
    eaConnected.value = s.eaConnected;
    eaMessage.value = "Compte EA connecté — actualisation de la bibliothèque…";
    reload();
    closeSettings();
  } catch (err) {
    eaMessage.value = String(err);
  } finally {
    eaBusy.value = false;
  }
}

async function onDisconnectEa() {
  eaBusy.value = true;
  const s = await disconnectEa();
  eaConnected.value = s.eaConnected;
  eaMessage.value = "Compte EA déconnecté.";
  eaBusy.value = false;
  reload();
}

// Battle.net — connexion via cookies de session, récupère la bibliothèque (games-and-subs).
async function onConnectBnet() {
  bnetBusy.value = true;
  bnetMessage.value = "Connexion… une fenêtre Battle.net s'est ouverte, connecte-toi.";
  try {
    const s = await connectBattlenet();
    bnetConnected.value = s.battlenetConnected;
    bnetMessage.value = "Compte Battle.net connecté — actualisation de la bibliothèque…";
    reload();
    closeSettings();
  } catch (err) {
    bnetMessage.value = String(err);
  } finally {
    bnetBusy.value = false;
  }
}

async function onDisconnectBnet() {
  bnetBusy.value = true;
  const s = await disconnectBattlenet();
  bnetConnected.value = s.battlenetConnected;
  bnetMessage.value = "Compte Battle.net déconnecté.";
  bnetBusy.value = false;
  reload();
}
</script>

<template>
  <div>
    <section class="group">
      <div class="group-label">Comptes — jeux possédés (installés ou non)</div>

      <div class="account">
        <div class="account-top">
          <span class="dot" style="background: var(--steam)" />
          <span class="account-name">Steam</span>
          <span class="badge" :class="steamConnected ? 'on' : ''">
            {{ steamConnected ? "Connecté" : "Non connecté" }}
          </span>
        </div>

        <template v-if="steamConnected">
          <p class="hint">Bibliothèque synchronisée.</p>
          <div class="row">
            <button class="btn-primary" :disabled="busy" @click="onResync">Resynchroniser</button>
            <button class="btn-secondary" :disabled="busy" @click="onDisconnect">Déconnecter</button>
          </div>
        </template>

        <template v-else>
          <p class="hint">
            Connecte-toi à ton compte Steam pour importer toute ta bibliothèque
            (installés ou non) et ta wishlist. Aucune clé, aucun mot de passe transmis à Torii —
            tu te connectes dans la fenêtre officielle de Steam.
          </p>
          <div class="row">
            <button class="btn-primary" :disabled="busy" @click="onConnect">
              {{ busy ? "En attente de connexion…" : "Se connecter avec Steam" }}
            </button>
          </div>

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
            <button class="btn-secondary" :disabled="busy || !steamKey.trim()" @click="onSaveKey">
              Enregistrer
            </button>
          </div>
        </template>

        <p v-if="message" class="message">{{ message }}</p>
      </div>

      <div class="account">
        <div class="account-top">
          <span class="dot" style="background: var(--epic)" />
          <span class="account-name">Epic Games</span>
          <span class="badge" :class="epicConnected ? 'on' : ''">
            {{ epicConnected ? "Connecté" : "Non connecté" }}
          </span>
        </div>

        <template v-if="epicConnected">
          <p class="hint">Bibliothèque Epic synchronisée.</p>
          <div class="row">
            <button class="btn-primary" :disabled="epicBusy" @click="onResyncEpic">Resynchroniser</button>
            <button class="btn-secondary" :disabled="epicBusy" @click="onDisconnectEpic">Déconnecter</button>
          </div>
        </template>

        <template v-else>
          <p class="hint">
            Connecte-toi à ton compte Epic pour importer toute ta bibliothèque
            (installés ou non). Aucun mot de passe transmis à Torii — tu te
            connectes dans la fenêtre officielle d'Epic Games.
          </p>
          <div class="row">
            <button class="btn-primary" :disabled="epicBusy" @click="onConnectEpic">
              {{ epicBusy ? "En attente de connexion…" : "Se connecter avec Epic" }}
            </button>
          </div>
        </template>

        <p v-if="epicMessage" class="message">{{ epicMessage }}</p>
      </div>

      <div class="account">
        <div class="account-top">
          <span class="dot" style="background: var(--ea)" />
          <span class="account-name">EA</span>
          <span class="badge" :class="eaConnected ? 'on' : ''">
            {{ eaConnected ? "Connecté" : "Non connecté" }}
          </span>
        </div>

        <template v-if="eaConnected">
          <p class="hint">Bibliothèque EA synchronisée.</p>
          <div class="row">
            <button class="btn-primary" :disabled="eaBusy" @click="onConnectEa">
              {{ eaBusy ? "Actualisation…" : "Resynchroniser" }}
            </button>
            <button class="btn-secondary" :disabled="eaBusy" @click="onDisconnectEa">Déconnecter</button>
          </div>
        </template>

        <template v-else>
          <p class="hint">
            Connecte-toi à ton compte EA pour importer toute ta bibliothèque
            (installés ou non). Aucun mot de passe transmis à Torii — tu te
            connectes dans la fenêtre officielle d'EA.
          </p>
          <div class="row">
            <button class="btn-primary" :disabled="eaBusy" @click="onConnectEa">
              {{ eaBusy ? "En attente de connexion…" : "Se connecter avec EA" }}
            </button>
          </div>
        </template>

        <p v-if="eaMessage" class="message">{{ eaMessage }}</p>
      </div>

      <div class="account">
        <div class="account-top">
          <span class="dot" style="background: var(--battlenet)" />
          <span class="account-name">Battle.net</span>
          <span class="badge" :class="bnetConnected ? 'on' : ''">
            {{ bnetConnected ? "Connecté" : "Non connecté" }}
          </span>
        </div>

        <template v-if="bnetConnected">
          <p class="hint">Bibliothèque Battle.net synchronisée.</p>
          <div class="row">
            <button class="btn-primary" :disabled="bnetBusy" @click="onConnectBnet">
              {{ bnetBusy ? "Actualisation…" : "Resynchroniser" }}
            </button>
            <button class="btn-secondary" :disabled="bnetBusy" @click="onDisconnectBnet">Déconnecter</button>
          </div>
        </template>

        <template v-else>
          <p class="hint">
            Connecte-toi à ton compte Battle.net pour importer ta bibliothèque
            Blizzard. Aucun mot de passe transmis à Torii — tu te connectes dans la
            fenêtre officielle de Battle.net.
          </p>
          <div class="row">
            <button class="btn-primary" :disabled="bnetBusy" @click="onConnectBnet">
              {{ bnetBusy ? "En attente de connexion…" : "Se connecter avec Battle.net" }}
            </button>
          </div>
        </template>

        <p v-if="bnetMessage" class="message">{{ bnetMessage }}</p>
      </div>

      <div class="account">
        <div class="account-top">
          <span class="dot" style="background: var(--gog)" />
          <span class="account-name">GOG</span>
          <span class="badge" :class="gogConnected ? 'on' : ''">
            {{ gogConnected ? "Connecté" : "Non connecté" }}
          </span>
        </div>

        <template v-if="gogConnected">
          <p class="hint">Bibliothèque GOG synchronisée.</p>
          <div class="row">
            <button class="btn-primary" :disabled="gogBusy" @click="onResyncGog">Resynchroniser</button>
            <button class="btn-secondary" :disabled="gogBusy" @click="onDisconnectGog">Déconnecter</button>
          </div>
        </template>

        <template v-else>
          <p class="hint">
            Connecte-toi à ton compte GOG pour importer toute ta bibliothèque
            (installés ou non). Aucun mot de passe transmis à Torii — tu te
            connectes dans la fenêtre officielle de GOG.
          </p>
          <div class="row">
            <button class="btn-primary" :disabled="gogBusy" @click="onConnectGog">
              {{ gogBusy ? "En attente de connexion…" : "Se connecter avec GOG" }}
            </button>
          </div>
        </template>

        <p v-if="gogMessage" class="message">{{ gogMessage }}</p>
      </div>
    </section>

    <p class="footnote">Les identifiants sont stockés localement, sur cette machine uniquement.</p>
  </div>
</template>

<style scoped>
.group-label {
  font-size: 11px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint);
  font-weight: 700; margin-bottom: 14px;
}
.account {
  border: 1px solid var(--border); border-radius: 12px; padding: 11px 13px; margin-bottom: 7px;
  background: var(--surface-2);
}
.account-top { display: flex; align-items: center; gap: 9px; }
.account-top .dot { width: 8px; height: 8px; border-radius: 50%; }
.account-name { font-weight: 600; font-size: 13.5px; }
.badge {
  margin-left: auto; font-size: 10.5px; font-family: var(--mono); padding: 2px 8px; border-radius: 99px;
  background: var(--surface-3); color: var(--text-faint);
}
.badge.on { background: color-mix(in srgb, var(--manual) 20%, transparent); color: var(--manual); }
.hint { font-size: 12px; line-height: 1.45; color: var(--text-dim); margin: 7px 0; }
.hint code { font-family: var(--mono); color: var(--text); }
.row { display: flex; gap: 7px; flex-wrap: wrap; align-items: center; }
.row.advanced { margin-top: 7px; }
.row input {
  flex: 1; min-width: 200px; padding: 8px 12px; border-radius: 9px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text); font-size: 13px; font-family: var(--mono);
}
.row input:focus { outline: none; border-color: var(--border-strong); }
.btn-primary {
  padding: 8px 15px; border-radius: 9px; border: none; background: var(--accent);
  color: var(--accent-ink); font-weight: 700; font-size: 12.5px;
}
.btn-primary:disabled { opacity: 0.6; cursor: default; }
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
.message { font-size: 12px; color: var(--accent); margin: 9px 0 0; line-height: 1.4; }
.footnote { font-size: 11.5px; color: var(--text-faint); text-align: center; margin: 16px 0 4px; }
</style>
