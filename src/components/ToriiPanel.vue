<script setup lang="ts">
import { computed, ref } from "vue";
import { useTorii } from "../composables/useTorii";
import { showToast } from "../composables/useToast";

/**
 * Connexion au réseau Torii, en deux temps : adresse puis code à six chiffres.
 *
 * Ce composant ne s'occupe QUE de la connexion. Tout ce qui concerne un compte déjà
 * connecté — code d'ami, ajout, visibilité — vit là où ça sert : dans la vue Amis pour
 * les actions du quotidien, dans les Réglages pour l'administration du compte.
 */

const { account, connected, requestCode, verify, setDisplayName } = useTorii();

const step = ref<"idle" | "email" | "code" | "pseudo">("idle");
const pseudo = ref("");
const email = ref("");
const code = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
/** Code rendu directement par le serveur en mode développement (aucun e-mail ne part). */
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
    const nouveau = await verify(email.value, code.value);
    devCode.value = null;
    if (nouveau) {
      // Inscription : on propose un pseudo AVANT que quoi que ce soit soit publié.
      // Le nom déduit de l'adresse est pré-rempli — autant qu'on voie ce que les autres
      // verraient si on ne changeait rien.
      pseudo.value = account.value?.displayName ?? "";
      step.value = "pseudo";
    } else {
      step.value = "idle";
      showToast("Compte Torii connecté.");
    }
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

/** Enregistre le pseudo choisi à l'inscription (ou garde celui proposé). */
async function onPseudo() {
  const nom = pseudo.value.trim();
  if (busy.value || !nom) return;
  busy.value = true;
  error.value = null;
  try {
    if (nom !== account.value?.displayName) await setDisplayName(nom);
    step.value = "idle";
    showToast(`Bienvenue, ${nom}.`);
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

/** Les ponts `torii*` laissent remonter le message du serveur : il est fait pour être lu. */
function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
</script>

<template>
  <!-- Connecté : une seule ligne de rappel, le reste est ailleurs. -->
  <p v-if="connected" class="signed">
    Connecté en tant que <strong>{{ account?.displayName }}</strong>
  </p>

  <div v-else class="invite">
    <div class="pitch">
      <span class="title">Réseau Torii</span>
      <span class="sub">
        Vois à quoi jouent tes amis, quel que soit leur launcher — et montre-leur ce que
        tu joues, si tu le décides.
      </span>
    </div>

    <button v-if="step === 'idle'" class="btn-primary" @click="step = 'email'">
      Créer un compte ou se connecter
    </button>

    <form v-else-if="step === 'email'" class="form" @submit.prevent="onSendCode">
      <input v-model="email" type="email" placeholder="ton@email.fr" autocomplete="email" spellcheck="false" />
      <button type="submit" class="btn-primary" :disabled="!canSendEmail || busy">
        {{ busy ? "Envoi…" : "Recevoir un code" }}
      </button>
      <button type="button" class="btn-ghost" @click="step = 'idle'">Annuler</button>
      <p class="hint">Pas de mot de passe : un code à six chiffres arrive par e-mail.</p>
    </form>

    <form v-else-if="step === 'pseudo'" class="form" @submit.prevent="onPseudo">
      <input v-model="pseudo" maxlength="40" placeholder="Ton pseudo" spellcheck="false" autofocus />
      <button type="submit" class="btn-primary" :disabled="busy || !pseudo.trim()">
        {{ busy ? "Enregistrement…" : "C'est parti" }}
      </button>
      <p class="hint">
        C'est le nom que verront tes amis. Par défaut il reprend ton adresse e-mail —
        autant en choisir un vrai. Modifiable à tout moment dans les réglages.
      </p>
    </form>

    <form v-else class="form" @submit.prevent="onVerify">
      <input
        v-model="code"
        inputmode="numeric"
        maxlength="6"
        placeholder="123456"
        class="code"
        autocomplete="one-time-code"
      />
      <button type="submit" class="btn-primary" :disabled="!canVerify || busy">
        {{ busy ? "Vérification…" : "Valider" }}
      </button>
      <button type="button" class="btn-ghost" @click="step = 'email'">Changer d'adresse</button>
      <p class="hint">Envoyé à {{ email }}. Regarde aussi tes indésirables.</p>
    </form>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-if="devCode" class="dev">
      Serveur en mode développement : aucun e-mail ne part. Ton code est
      <strong>{{ devCode }}</strong>.
    </p>
  </div>
</template>

<style scoped>
.signed { font-size: 13px; color: var(--text-faint); margin: 0 0 16px; }
.signed strong { color: var(--text); font-weight: 600; }

.invite {
  display: flex; flex-direction: column; gap: 14px;
  padding: 18px 20px; margin-bottom: 22px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
}
.pitch { display: flex; flex-direction: column; gap: 4px; }
.pitch .title { font-size: 15px; font-weight: 700; letter-spacing: -0.01em; }
.pitch .sub { font-size: 13px; color: var(--text-dim); line-height: 1.5; max-width: 60ch; }

.form { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.form input {
  padding: 9px 13px; border-radius: 10px; font-size: 13.5px; font-family: inherit;
  background: var(--bg); border: 1px solid var(--border); color: var(--text); min-width: 210px;
}
.form input:focus { outline: none; border-color: var(--accent); }
.form input.code {
  font-family: var(--mono); font-size: 16px; letter-spacing: 0.3em; text-align: center; min-width: 150px;
}
.hint { flex-basis: 100%; margin: 2px 0 0; font-size: 12px; color: var(--text-faint); }

.btn-primary {
  padding: 9px 16px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
  align-self: flex-start;
}
.btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.btn-ghost {
  padding: 8px 13px; border-radius: 10px; font-size: 12.5px; cursor: pointer;
  background: none; border: 1px solid var(--border); color: var(--text-dim);
}
.btn-ghost:hover { color: var(--text); border-color: var(--border-strong); }

.error {
  margin: 0; padding: 9px 13px; border-radius: 9px; font-size: 12.5px; color: #ff6b6b;
  background: color-mix(in srgb, #ff6b6b 12%, transparent);
}
.dev {
  margin: 0; padding: 9px 13px; border-radius: 9px; font-size: 12.5px;
  color: var(--text-dim); background: var(--surface-2); border: 1px dashed var(--border-strong);
}
.dev strong { font-family: var(--mono); letter-spacing: 0.16em; color: var(--text); }
</style>
