<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useUi } from "../composables/useUi";
import { useLibrary } from "../composables/useLibrary";

const { addGameOpen, closeAddGame, openGame } = useUi();
const { addManual } = useLibrary();

const title = ref("");
const launchTarget = ref("");
const installDir = ref("");
const coverUrl = ref("");
const saving = ref(false);
const error = ref<string | null>(null);

const canSave = computed(() => title.value.trim() !== "" && launchTarget.value.trim() !== "");

// Réinitialise le formulaire à chaque ouverture.
watch(addGameOpen, (open) => {
  if (open) {
    title.value = "";
    launchTarget.value = "";
    installDir.value = "";
    coverUrl.value = "";
    error.value = null;
    saving.value = false;
  }
});

async function save() {
  if (!canSave.value || saving.value) return;
  saving.value = true;
  error.value = null;
  try {
    const game = await addManual({
      title: title.value.trim(),
      launchTarget: launchTarget.value.trim(),
      installDir: installDir.value.trim() || null,
      coverUrl: coverUrl.value.trim() || null,
    });
    closeAddGame();
    if (game) openGame(game.id);
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") closeAddGame();
}
</script>

<template>
  <div v-if="addGameOpen" class="modal-backdrop" @click.self="closeAddGame" @keydown="onKey">
    <div class="modal" role="dialog" aria-modal="true" aria-label="Ajouter un jeu">
      <div class="modal-head">
        <h3>Ajouter un jeu</h3>
        <button class="modal-close" aria-label="Fermer" @click="closeAddGame">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
      </div>
      <p class="modal-sub">Référence un jeu (ou toute application) qui n'apparaît dans aucun launcher.</p>

      <form class="modal-form" @submit.prevent="save">
        <label class="field">
          <span class="field-label">Titre <em>*</em></span>
          <input v-model="title" type="text" placeholder="Ex : Minecraft" autofocus />
        </label>
        <label class="field">
          <span class="field-label">Chemin de l'exécutable <em>*</em></span>
          <input v-model="launchTarget" type="text" placeholder="C:\Jeux\MonJeu\jeu.exe" spellcheck="false" />
          <span class="field-hint">Le fichier lancé quand tu cliques sur « Jouer » (.exe, .bat, .lnk…).</span>
        </label>
        <label class="field">
          <span class="field-label">Dossier d'installation <i>(optionnel)</i></span>
          <input v-model="installDir" type="text" placeholder="C:\Jeux\MonJeu" spellcheck="false" />
        </label>
        <label class="field">
          <span class="field-label">URL de la jaquette <i>(optionnel)</i></span>
          <input v-model="coverUrl" type="text" placeholder="https://…/cover.jpg" spellcheck="false" />
          <span class="field-hint">Sinon un dégradé est généré automatiquement.</span>
        </label>

        <p v-if="error" class="modal-error">{{ error }}</p>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="closeAddGame">Annuler</button>
          <button type="submit" class="btn-save" :disabled="!canSave || saving">
            {{ saving ? "Ajout…" : "Ajouter le jeu" }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.modal-backdrop {
  position: fixed; inset: 0; z-index: 260; display: grid; place-items: center; padding: 5vh 20px;
  background: rgba(6, 4, 10, 0.62); backdrop-filter: blur(5px);
}
.modal {
  width: 100%; max-width: 460px; background: var(--surface); border: 1px solid var(--border);
  border-radius: 18px; box-shadow: var(--shadow-hero); padding: 24px;
}
.modal-head { display: flex; align-items: center; justify-content: space-between; }
.modal-head h3 { font-size: 19px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.modal-close {
  width: 34px; height: 34px; border-radius: 10px; display: grid; place-items: center;
  background: none; border: 1px solid var(--border); color: var(--text-dim); cursor: pointer;
}
.modal-close:hover { color: var(--text); border-color: var(--border-strong); }
.modal-close svg { width: 17px; height: 17px; }
.modal-sub { font-size: 13px; color: var(--text-faint); margin: 6px 0 18px; line-height: 1.5; }
.modal-form { display: flex; flex-direction: column; gap: 15px; }
.field { display: flex; flex-direction: column; gap: 6px; }
.field-label { font-size: 12.5px; font-weight: 600; color: var(--text-dim); }
.field-label em { color: var(--accent); font-style: normal; }
.field-label i { color: var(--text-faint); font-style: normal; font-weight: 400; }
.field input {
  width: 100%; padding: 10px 13px; background: var(--bg); border: 1px solid var(--border);
  border-radius: 10px; color: var(--text); font-size: 13.5px; font-family: inherit;
}
.field input:focus { outline: none; border-color: var(--border-strong); background: var(--surface-2); }
.field input::placeholder { color: var(--text-faint); }
.field-hint { font-size: 11.5px; color: var(--text-faint); line-height: 1.4; }
.modal-error {
  font-size: 12.5px; color: #ff6b6b; margin: 0;
  padding: 9px 12px; border-radius: 9px; background: color-mix(in srgb, #ff6b6b 12%, transparent);
}
.modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 6px; }
.btn-cancel, .btn-save {
  padding: 10px 18px; border-radius: 11px; font-size: 13.5px; font-weight: 600; cursor: pointer;
}
.btn-cancel { background: none; border: 1px solid var(--border); color: var(--text-dim); }
.btn-cancel:hover { color: var(--text); border-color: var(--border-strong); }
.btn-save { background: var(--accent); border: 1px solid transparent; color: var(--accent-ink); }
.btn-save:hover:not(:disabled) { background: var(--accent-hover); }
.btn-save:disabled { opacity: 0.5; cursor: default; }
</style>
