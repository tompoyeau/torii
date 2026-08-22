<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useTorii } from "../composables/useTorii";
import { useScrollLock } from "../composables/useScrollLock";
import { showToast } from "../composables/useToast";

/**
 * Création de compte et connexion, en trois temps : adresse, code, pseudo.
 *
 * 🔑 L'étape du pseudo ne se ferme pas. Ni croix, ni Échap, ni clic à côté. Ce n'est pas
 * une brimade : tant que le pseudo n'est pas choisi, **le compte n'existe pas encore**
 * côté serveur. Abandonner ici ne laisse donc rien derrière soi — c'est justement ce qui
 * permet de verrouiller la sortie sans piéger personne : fermer Torii annule tout.
 */

const { signInOpen, closeSignIn, requestCode, verify, completeSignup, abandonSignup } = useTorii();

type Etape = "email" | "code" | "pseudo";
const etape = ref<Etape>("email");
const email = ref("");
const code = ref("");
const pseudo = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
/** Code rendu directement par le serveur en mode développement (aucun e-mail ne part). */
const devCode = ref<string | null>(null);

/** L'étape du pseudo verrouille la fenêtre : rien ne doit pouvoir la refermer. */
const verrouille = computed(() => etape.value === "pseudo");

useScrollLock(signInOpen);

const champ = ref<HTMLInputElement | null>(null);

/** Repart d'une feuille blanche à chaque ouverture. */
watch(signInOpen, (ouvert) => {
  if (!ouvert) return;
  etape.value = "email";
  email.value = "";
  code.value = "";
  pseudo.value = "";
  devCode.value = null;
  error.value = null;
});

/** Le curseur suit l'étape : personne ne devrait avoir à cliquer dans le champ suivant. */
watch([signInOpen, etape], async () => {
  if (!signInOpen.value) return;
  await nextTick();
  champ.value?.focus();
});

const emailValide = computed(() => /\S+@\S+\.\S+/.test(email.value.trim()));
const codeValide = computed(() => code.value.trim().length === 6);
const pseudoValide = computed(() => pseudo.value.trim().length >= 2);

function fermer() {
  if (verrouille.value) return;
  closeSignIn();
}

async function onEmail() {
  if (!emailValide.value || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    devCode.value = await requestCode(email.value);
    code.value = "";
    etape.value = "code";
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

async function onCode() {
  if (!codeValide.value || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    const aChoisirUnPseudo = await verify(email.value, code.value);
    devCode.value = null;
    if (aChoisirUnPseudo) {
      etape.value = "pseudo";
    } else {
      closeSignIn();
      showToast("Compte Torii connecté.");
    }
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

async function onPseudo() {
  if (!pseudoValide.value || busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    const nom = pseudo.value.trim();
    await completeSignup(nom);
    closeSignIn();
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

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && signInOpen.value) fermer();
}
onMounted(() => document.addEventListener("keydown", onKey));
onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKey);
  // Un rechargement à chaud ne doit pas laisser traîner un laissez-passer inutilisable.
  abandonSignup();
});
</script>

<template>
  <div v-if="signInOpen" class="modal-backdrop" @click.self="fermer">
    <div class="modal" role="dialog" aria-modal="true" aria-label="Compte Torii">
      <div class="modal-head">
        <h3>{{ etape === "pseudo" ? "Choisis ton pseudo" : "Compte Torii" }}</h3>
        <button v-if="!verrouille" class="modal-close" aria-label="Fermer" @click="closeSignIn">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
      </div>

      <!-- Où l'on en est. Trois repères valent mieux qu'une fenêtre qui change sans prévenir. -->
      <div class="steps" aria-hidden="true">
        <span :class="{ done: etape !== 'email', now: etape === 'email' }" />
        <span :class="{ done: etape === 'pseudo', now: etape === 'code' }" />
        <span :class="{ now: etape === 'pseudo' }" />
      </div>

      <!-- ── 1. L'adresse ── -->
      <form v-if="etape === 'email'" class="body" @submit.prevent="onEmail">
        <p class="lead">
          Vois à quoi jouent tes amis, quel que soit leur launcher — et montre-leur ce que
          tu joues, si tu le décides.
        </p>
        <label class="field">
          <span>Ton adresse e-mail</span>
          <input
            ref="champ" v-model="email" type="email" placeholder="ton@email.fr"
            autocomplete="email" spellcheck="false"
          />
        </label>
        <p class="hint">Pas de mot de passe : un code à six chiffres arrive par e-mail.</p>
        <div class="actions">
          <button type="submit" class="btn-primary" :disabled="!emailValide || busy">
            {{ busy ? "Envoi…" : "Recevoir un code" }}
          </button>
        </div>
      </form>

      <!-- ── 2. Le code ── -->
      <form v-else-if="etape === 'code'" class="body" @submit.prevent="onCode">
        <p class="lead">
          Un code à six chiffres vient de partir vers <strong>{{ email }}</strong>.
          Regarde aussi tes indésirables.
        </p>
        <label class="field">
          <span>Le code reçu</span>
          <input
            ref="champ" v-model="code" inputmode="numeric" maxlength="6"
            placeholder="123456" class="code" autocomplete="one-time-code"
          />
        </label>
        <p v-if="devCode" class="dev">
          Serveur en mode développement : aucun e-mail ne part. Ton code est
          <strong>{{ devCode }}</strong>.
        </p>
        <div class="actions">
          <button type="button" class="btn-ghost" @click="etape = 'email'">Changer d'adresse</button>
          <button type="submit" class="btn-primary" :disabled="!codeValide || busy">
            {{ busy ? "Vérification…" : "Valider" }}
          </button>
        </div>
      </form>

      <!-- ── 3. Le pseudo : obligatoire, et c'est ici que le compte naît ── -->
      <form v-else class="body" @submit.prevent="onPseudo">
        <p class="lead">
          C'est le nom que verront tes amis. Choisis-le maintenant : ton compte sera créé
          avec.
        </p>
        <label class="field">
          <span>Ton pseudo</span>
          <input
            ref="champ" v-model="pseudo" maxlength="40" placeholder="Ton pseudo"
            spellcheck="false" autocomplete="off"
          />
        </label>
        <p class="hint">Deux caractères minimum. Modifiable à tout moment dans les réglages.</p>
        <div class="actions">
          <button type="submit" class="btn-primary" :disabled="!pseudoValide || busy">
            {{ busy ? "Création…" : "Créer mon compte" }}
          </button>
        </div>
        <p class="locked">
          Tant qu'aucun pseudo n'est choisi, aucun compte n'est créé. Fermer Torii
          maintenant annule l'inscription, sans rien laisser derrière.
        </p>
      </form>

      <p v-if="error" class="error">{{ error }}</p>
    </div>
  </div>
</template>

<style scoped>
.modal-backdrop {
  position: fixed; inset: 0; z-index: 260; display: grid; place-items: center; padding: 5vh 20px;
  background: rgba(6, 4, 10, 0.62); backdrop-filter: blur(5px);
}
.modal {
  width: 100%; max-width: 440px; background: var(--surface); border: 1px solid var(--border);
  border-radius: 18px; box-shadow: var(--shadow-hero); padding: 24px;
}
.modal-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.modal-head h3 { font-size: 19px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.modal-close {
  display: grid; place-items: center; width: 30px; height: 30px; border-radius: 9px;
  background: none; border: 1px solid transparent; color: var(--text-faint); cursor: pointer;
}
.modal-close:hover { color: var(--text); background: var(--surface-2); }
.modal-close svg { width: 16px; height: 16px; }

.steps { display: flex; gap: 6px; margin: 16px 0 18px; }
.steps span { height: 3px; flex: 1; border-radius: 99px; background: var(--surface-2); }
.steps span.done { background: color-mix(in srgb, var(--accent) 45%, transparent); }
.steps span.now { background: var(--accent); }

.body { display: flex; flex-direction: column; gap: 14px; }
.lead { margin: 0; font-size: 13.5px; line-height: 1.55; color: var(--text-dim); }
.lead strong { color: var(--text); font-weight: 600; }

.field { display: flex; flex-direction: column; gap: 6px; }
.field > span { font-size: 12px; font-weight: 600; color: var(--text-faint); }
.field input {
  padding: 11px 13px; border-radius: 10px; font-size: 14px; font-family: inherit;
  background: var(--bg); border: 1px solid var(--border); color: var(--text); width: 100%;
}
.field input:focus { outline: none; border-color: var(--accent); }
.field input.code {
  font-family: var(--mono); font-size: 18px; letter-spacing: 0.32em; text-align: center;
}

.hint { margin: -4px 0 0; font-size: 12px; color: var(--text-faint); line-height: 1.5; }
.actions { display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 4px; }

.btn-primary {
  padding: 10px 18px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
  font-family: inherit;
}
.btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.btn-ghost {
  padding: 9px 14px; border-radius: 10px; font-size: 12.5px; cursor: pointer; font-family: inherit;
  background: none; border: 1px solid var(--border); color: var(--text-dim); margin-right: auto;
}
.btn-ghost:hover { color: var(--text); border-color: var(--border-strong); }

.locked {
  margin: 4px 0 0; padding: 10px 12px; border-radius: 9px; font-size: 12px; line-height: 1.5;
  color: var(--text-faint); background: var(--surface-2);
}
.error {
  margin: 14px 0 0; padding: 10px 13px; border-radius: 9px; font-size: 12.5px; color: #ff6b6b;
  background: color-mix(in srgb, #ff6b6b 12%, transparent);
}
.dev {
  margin: 0; padding: 9px 13px; border-radius: 9px; font-size: 12.5px;
  color: var(--text-dim); background: var(--surface-2); border: 1px dashed var(--border-strong);
}
.dev strong { font-family: var(--mono); letter-spacing: 0.16em; color: var(--text); }
</style>
