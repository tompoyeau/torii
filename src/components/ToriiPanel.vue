<script setup lang="ts">
import { computed, ref } from "vue";
import { useTorii } from "../composables/useTorii";
import { showToast } from "../composables/useToast";

/**
 * Bandeau du réseau Torii, en tête du panneau Amis.
 *
 * Trois états : déconnecté (formulaire en deux temps, e-mail puis code), connecté
 * (code d'ami + ajout), et les demandes reçues quand il y en a.
 */

const {
  account, circle, connected, prefs, requestCode, verify, invite, respond, rotateCode, setPrefs,
} = useTorii();

/** Étape du formulaire de connexion. */
const step = ref<"idle" | "email" | "code">("idle");
const email = ref("");
const code = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
/**
 * Code rendu directement par le serveur quand il tourne en mode développement : aucun
 * e-mail ne part alors, autant le montrer plutôt que de faire attendre pour rien.
 */
const devCode = ref<string | null>(null);

const canSendEmail = computed(() => /\S+@\S+\.\S+/.test(email.value.trim()));
const canVerify = computed(() => code.value.trim().length === 6);

async function onSendCode() {
  if (!canSendEmail.value || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    devCode.value = await requestCode(email.value);
    step.value = "code";
    code.value = "";
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

async function onVerify() {
  if (!canVerify.value || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    await verify(email.value, code.value);
    step.value = "idle";
    devCode.value = null;
    showToast("Compte Torii connecté.");
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

/* ── Ajout d'un ami ────────────────────────────────────────────────────────── */

const adding = ref(false);
const friendCode = ref("");

async function onInvite() {
  const value = friendCode.value.trim();
  if (!value || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    await invite(value);
    friendCode.value = "";
    adding.value = false;
    showToast("Demande envoyée.");
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

async function onCopyCode() {
  if (!account.value) return;
  try {
    await navigator.clipboard.writeText(account.value.friendCode);
    showToast("Code d'ami copié.");
  } catch {
    showToast("Copie impossible ; note le code à la main.");
  }
}

async function onRotate() {
  if (busy.value) return;
  busy.value = true;
  try {
    await rotateCode();
    showToast("Nouveau code d'ami : l'ancien ne fonctionne plus.");
  } finally {
    busy.value = false;
  }
}

async function onToggleShare() {
  await setPrefs({ sharePresence: !prefs.value.sharePresence });
}

/** Les erreurs remontées par le pont portent le message du serveur, lisible tel quel. */
function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
</script>

<template>
  <!-- Déconnecté : invitation à créer un compte, puis formulaire en deux temps -->
  <div v-if="!connected" class="torii-bar">
    <div class="pitch">
      <span class="title">Réseau Torii</span>
      <span class="sub">Vois à quoi jouent tes amis, quel que soit leur launcher.</span>
    </div>

    <button v-if="step === 'idle'" class="btn-primary" @click="step = 'email'">
      Se connecter
    </button>

    <form v-else-if="step === 'email'" class="inline-form" @submit.prevent="onSendCode">
      <input
        v-model="email"
        type="email"
        placeholder="ton@email.fr"
        autocomplete="email"
        spellcheck="false"
      />
      <button type="submit" class="btn-primary" :disabled="!canSendEmail || busy">
        {{ busy ? "Envoi…" : "Recevoir un code" }}
      </button>
      <button type="button" class="btn-ghost-sm" @click="step = 'idle'">Annuler</button>
    </form>

    <form v-else class="inline-form" @submit.prevent="onVerify">
      <input
        v-model="code"
        inputmode="numeric"
        maxlength="6"
        placeholder="123456"
        class="code-input"
        autocomplete="one-time-code"
      />
      <button type="submit" class="btn-primary" :disabled="!canVerify || busy">
        {{ busy ? "Vérification…" : "Valider" }}
      </button>
      <button type="button" class="btn-ghost-sm" @click="step = 'email'">Changer d'adresse</button>
    </form>
  </div>

  <!-- Connecté : code d'ami, ajout, partage de présence -->
  <div v-else class="torii-bar">
    <div class="pitch">
      <span class="title">{{ account?.displayName }}</span>
      <span class="sub">
        Ton code d'ami :
        <button class="code-chip" title="Copier" @click="onCopyCode">{{ account?.friendCode }}</button>
        <button class="btn-ghost-sm" title="En générer un nouveau" @click="onRotate">Renouveler</button>
      </span>
    </div>

    <button
      class="btn-share"
      :class="{ on: prefs.sharePresence }"
      :title="prefs.sharePresence
        ? 'Tes amis voient à quoi tu joues'
        : 'Personne ne voit ce que tu fais'"
      @click="onToggleShare"
    >
      <span class="share-dot" />
      {{ prefs.sharePresence ? "Visible" : "Invisible" }}
    </button>

    <form v-if="adding" class="inline-form" @submit.prevent="onInvite">
      <input
        v-model="friendCode"
        placeholder="Code d'ami"
        maxlength="12"
        spellcheck="false"
        class="code-input wide"
      />
      <button type="submit" class="btn-primary" :disabled="busy">Ajouter</button>
      <button type="button" class="btn-ghost-sm" @click="adding = false">Annuler</button>
    </form>
    <button v-else class="btn-primary" @click="adding = true">Ajouter un ami</button>
  </div>

  <p v-if="error" class="torii-error">{{ error }}</p>

  <p v-if="devCode" class="torii-dev">
    Serveur en mode développement : aucun e-mail ne part. Ton code est
    <strong>{{ devCode }}</strong>.
  </p>

  <!-- Demandes reçues -->
  <section v-if="circle.incoming.length" class="requests">
    <h4>Demandes d'amis <span>{{ circle.incoming.length }}</span></h4>
    <div v-for="p in circle.incoming" :key="p.id" class="request">
      <span class="req-name">{{ p.displayName }}</span>
      <button class="btn-primary sm" @click="respond(p.id, true)">Accepter</button>
      <button class="btn-ghost-sm" @click="respond(p.id, false)">Refuser</button>
    </div>
  </section>
</template>

<style scoped>
.torii-bar {
  display: flex; align-items: center; gap: 14px; flex-wrap: wrap;
  padding: 14px 16px; margin-bottom: 18px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
}
.pitch { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; }
.pitch .title { font-size: 14.5px; font-weight: 700; letter-spacing: -0.01em; }
.pitch .sub {
  display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
  font-size: 12.5px; color: var(--text-faint);
}

.inline-form { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.inline-form input {
  padding: 8px 12px; border-radius: 10px; font-size: 13.5px; font-family: inherit;
  background: var(--bg); border: 1px solid var(--border); color: var(--text); min-width: 190px;
}
.inline-form input:focus { outline: none; border-color: var(--accent); }
.code-input {
  font-family: var(--mono); letter-spacing: 0.22em; text-align: center; min-width: 130px;
}
.code-input.wide { letter-spacing: 0.12em; min-width: 160px; }

.btn-primary {
  padding: 9px 16px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
}
.btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.btn-primary.sm { padding: 6px 12px; font-size: 12.5px; }

.btn-ghost-sm {
  padding: 6px 10px; border-radius: 9px; font-size: 12.5px; cursor: pointer;
  background: none; border: 1px solid var(--border); color: var(--text-dim);
}
.btn-ghost-sm:hover { color: var(--text); border-color: var(--border-strong); }

.code-chip {
  font-family: var(--mono); font-size: 12.5px; letter-spacing: 0.12em; cursor: pointer;
  padding: 3px 9px; border-radius: 7px;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text);
}
.code-chip:hover { border-color: var(--accent); color: var(--accent); }

/* Partage de présence : l'état doit se lire d'un coup d'œil, sans survol. */
.btn-share {
  display: inline-flex; align-items: center; gap: 8px; cursor: pointer;
  padding: 8px 14px; border-radius: 99px; font-size: 13px; font-weight: 600;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim);
}
.btn-share .share-dot {
  width: 8px; height: 8px; border-radius: 50%; background: var(--text-faint);
}
.btn-share.on { color: #3ad07f; border-color: color-mix(in srgb, #3ad07f 45%, transparent); }
.btn-share.on .share-dot { background: #3ad07f; box-shadow: 0 0 8px #3ad07f; }

.torii-error {
  margin: -8px 0 16px; padding: 9px 13px; border-radius: 9px; font-size: 12.5px; color: #ff6b6b;
  background: color-mix(in srgb, #ff6b6b 12%, transparent);
}
.torii-dev {
  margin: -8px 0 16px; padding: 9px 13px; border-radius: 9px; font-size: 12.5px;
  color: var(--text-dim); background: var(--surface-2); border: 1px dashed var(--border-strong);
}
.torii-dev strong { font-family: var(--mono); letter-spacing: 0.16em; color: var(--text); }

.requests { margin-bottom: 22px; }
.requests h4 {
  font-size: 12px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--text-faint); margin: 0 0 10px;
}
.requests h4 span { font-family: var(--mono); }
.request {
  display: flex; align-items: center; gap: 10px; padding: 9px 12px; margin-bottom: 6px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 11px;
}
.req-name { flex: 1; font-size: 13.5px; font-weight: 600; }
</style>
