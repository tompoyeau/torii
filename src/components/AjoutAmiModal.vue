<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useTorii } from "../composables/useTorii";
import { useModaleDeNavigation } from "../composables/useNavigation";
import { useScrollLock } from "../composables/useScrollLock";
import { useFocusTrap } from "../composables/useFocusTrap";
import { showToast } from "../composables/useToast";
import { t } from "../i18n";

/**
 * Ajouter un ami Torii, dans une fenêtre : son code d'ami à saisir, et le sien à donner en
 * retour. Avant, un panneau s'ouvrait dans la page et la poussait vers le bas.
 */
const props = defineProps<{ ouvert: boolean }>();
const emit = defineEmits<{ fermer: [] }>();

const { account, invite } = useTorii();

const code = ref("");
const busy = ref(false);
const erreur = ref<string | null>(null);

const modale = ref<HTMLElement | null>(null);
const champ = ref<HTMLInputElement | null>(null);
const estOuvert = computed(() => props.ouvert);

useScrollLock(estOuvert);
useFocusTrap(modale, estOuvert);

/** Repart d'un champ vide à chaque ouverture, le curseur dedans. */
watch(
  () => props.ouvert,
  async (o) => {
    if (!o) return;
    code.value = "";
    erreur.value = null;
    await nextTick();
    champ.value?.focus();
  },
);

function fermer() {
  if (!busy.value) emit("fermer");
}

async function envoyer() {
  const c = code.value.trim();
  if (!c || busy.value) return;
  busy.value = true;
  erreur.value = null;
  try {
    await invite(c);
    showToast(t("amis.demandeEnvoyee"));
    emit("fermer");
  } catch (e) {
    erreur.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}

async function copier() {
  if (!account.value) return;
  try {
    await navigator.clipboard.writeText(account.value.friendCode);
    showToast(t("amis.codeCopie"));
  } catch {
    showToast(t("amis.copieImpossible"));
  }
}

/** Échap ferme la fenêtre, et rien d'autre : `useRaccourcis` ignore un Échap déjà pris. */
function surTouche(e: KeyboardEvent) {
  if (e.key === "Escape" && props.ouvert) {
    e.preventDefault();
    fermer();
  }
}
onMounted(() => document.addEventListener("keydown", surTouche));
onBeforeUnmount(() => document.removeEventListener("keydown", surTouche));
useModaleDeNavigation({ ouverte: () => props.ouvert, fermer });
</script>

<template>
  <div v-if="ouvert" class="modal-backdrop" @click.self="fermer">
    <div ref="modale" class="modal" role="dialog" aria-modal="true" tabindex="-1" :aria-label="t('amis.ajouterAmi')">
      <div class="modal-head">
        <h3>{{ t("amis.ajouterAmi") }}</h3>
        <button class="modal-close" :aria-label="t('commun.fermer')" @click="fermer">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
      </div>

      <form class="bloc" @submit.prevent="envoyer">
        <label class="etiquette" for="code-ami">{{ t("amis.sonCode") }}</label>
        <div class="ligne">
          <input
            id="code-ami"
            ref="champ"
            v-model="code"
            placeholder="ABCD2345"
            maxlength="12"
            spellcheck="false"
            autocomplete="off"
          />
          <button type="submit" class="btn-primary" :disabled="busy || !code.trim()">
            {{ busy ? t("amis.envoi") : t("amis.envoyer") }}
          </button>
        </div>
        <p v-if="erreur" class="erreur" role="alert">{{ erreur }}</p>
      </form>

      <div v-if="account" class="bloc mien">
        <span class="etiquette">{{ t("amis.tonCode") }}</span>
        <button class="code-chip" :title="t('amis.copier')" @click="copier">
          {{ account.friendCode }}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" /></svg>
        </button>
        <p class="aide">{{ t("amis.tonCodeAide") }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Même dessin que la fenêtre de connexion Torii (`ToriiSignInDialog`). */
.modal-backdrop {
  position: fixed; inset: 0; z-index: 260; display: grid; place-items: center; padding: 5vh 20px;
  background: rgba(6, 4, 10, 0.62); backdrop-filter: blur(5px);
}
.modal {
  width: 100%; max-width: 440px; background: var(--surface); border: 1px solid var(--border);
  border-radius: 18px; box-shadow: var(--shadow-hero); padding: 24px;
}
.modal-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 18px; }
.modal-head h3 { font-size: 19px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.modal-close {
  display: grid; place-items: center; width: 30px; height: 30px; border-radius: 9px;
  background: none; border: 1px solid transparent; color: var(--text-faint); cursor: pointer;
}
.modal-close:hover { color: var(--text); background: var(--surface-2); }
.modal-close svg { width: 16px; height: 16px; }

.bloc { display: flex; flex-direction: column; }
.mien { margin-top: 20px; padding-top: 18px; border-top: 1px solid var(--border); align-items: flex-start; }
.etiquette {
  display: block; font-size: 11.5px; font-weight: 700; letter-spacing: 0.06em;
  text-transform: uppercase; color: var(--text-faint); margin-bottom: 8px;
}
.ligne { display: flex; gap: 8px; }
.ligne input {
  flex: 1; min-width: 0; padding: 10px 13px; border-radius: 10px;
  background: var(--bg); border: 1px solid var(--border); color: var(--text);
  font-family: var(--mono); font-size: 15px; letter-spacing: 0.14em; text-transform: uppercase;
}
.ligne input:focus { border-color: var(--accent); outline: none; }
.erreur { margin: 8px 0 0; font-size: 12.5px; color: #ff6b6b; }
.aide { margin: 10px 0 0; font-size: 12.5px; line-height: 1.45; color: var(--text-faint); }

.btn-primary {
  padding: 10px 16px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
}
.btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.code-chip {
  display: inline-flex; align-items: center; gap: 9px; cursor: pointer;
  padding: 9px 15px; border-radius: 10px; font-family: var(--mono); font-size: 16px;
  letter-spacing: 0.16em; font-weight: 600;
  background: var(--surface-2); border: 1px dashed var(--border-strong); color: var(--text);
}
.code-chip:hover { border-color: var(--accent); color: var(--accent); }
.code-chip svg { width: 14px; height: 14px; opacity: 0.7; }
</style>
