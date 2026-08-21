<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useUi } from "../composables/useUi";
import { useLibrary } from "../composables/useLibrary";
import { useScrollLock } from "../composables/useScrollLock";
import { pickFile, pickFolder } from "../lib/tauri";

const { addGameOpen, editGameId, closeAddGame, openGame } = useUi();
const { addManual, updateManual, byId } = useLibrary();

useScrollLock(addGameOpen);

const title = ref("");
const launchTarget = ref("");
const installDir = ref("");
const coverUrl = ref("");
const saving = ref(false);
const error = ref<string | null>(null);

/** Édition d'un jeu existant plutôt que création. */
const editing = computed(() => editGameId.value != null);

const canSave = computed(() => title.value.trim() !== "" && launchTarget.value.trim() !== "");

// (Ré)initialise le formulaire à chaque ouverture : vide en création, pré-rempli en édition.
watch(addGameOpen, (open) => {
  if (!open) return;
  const g = editGameId.value ? byId(editGameId.value) : null;
  title.value = g?.title ?? "";
  launchTarget.value = g?.launchTarget ?? "";
  installDir.value = g?.installDir ?? "";
  // On réaffiche le chemin/l'URL d'origine, pas l'URL `asset://` de rendu.
  coverUrl.value = g?.coverSource ?? "";
  error.value = null;
  saving.value = false;
});

/** Choix de l'exécutable dans l'explorateur Windows. */
async function browseExe() {
  const path = await pickFile("Choisir l'exécutable du jeu", [
    { name: "Programmes", extensions: ["exe", "bat", "cmd", "lnk", "url"] },
    { name: "Tous les fichiers", extensions: ["*"] },
  ]);
  if (path) {
    launchTarget.value = path;
    // Le dossier d'installation se déduit du chemin choisi (sert au suivi de session
    // et à « ouvrir l'emplacement du fichier ») — sans écraser une saisie existante.
    if (!installDir.value) {
      const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
      if (cut > 0) installDir.value = path.slice(0, cut);
    }
    if (!title.value) {
      const file = path.slice(Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/")) + 1);
      title.value = file.replace(/\.[^.]+$/, "");
    }
  }
}

/** Choix du dossier d'installation. */
async function browseDir() {
  const dir = await pickFolder("Choisir le dossier du jeu");
  if (dir) installDir.value = dir;
}

/** Choix d'une image de jaquette sur le disque (une URL reste saisissable à la main). */
async function browseCover() {
  const path = await pickFile("Choisir une jaquette", [
    { name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "avif", "gif", "bmp"] },
  ]);
  if (path) coverUrl.value = path;
}

async function save() {
  if (!canSave.value || saving.value) return;
  saving.value = true;
  error.value = null;
  const input = {
    title: title.value.trim(),
    launchTarget: launchTarget.value.trim(),
    installDir: installDir.value.trim() || null,
    coverUrl: coverUrl.value.trim() || null,
  };
  try {
    const id = editGameId.value;
    const game = id ? await updateManual(id, input) : await addManual(input);
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
    <div class="modal" role="dialog" aria-modal="true" :aria-label="editing ? 'Modifier le jeu' : 'Ajouter un jeu'">
      <div class="modal-head">
        <h3>{{ editing ? "Modifier le jeu" : "Ajouter un jeu" }}</h3>
        <button class="modal-close" aria-label="Fermer" @click="closeAddGame">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
      </div>
      <p class="modal-sub">
        {{ editing
          ? "Corrige les informations de ce jeu ajouté à la main."
          : "Référence un jeu (ou toute application) qui n’apparaît dans aucun launcher." }}
      </p>

      <form class="modal-form" @submit.prevent="save">
        <label class="field">
          <span class="field-label">Titre <em>*</em></span>
          <input v-model="title" type="text" placeholder="Ex : Minecraft" autofocus />
        </label>
        <label class="field">
          <span class="field-label">Exécutable <em>*</em></span>
          <div class="field-row">
            <input v-model="launchTarget" type="text" placeholder="C:\Jeux\MonJeu\jeu.exe" spellcheck="false" />
            <button type="button" class="btn-browse" @click="browseExe">Parcourir…</button>
          </div>
          <span class="field-hint">Le fichier lancé quand tu cliques sur « Jouer » (.exe, .bat, .lnk…).</span>
        </label>
        <label class="field">
          <span class="field-label">Dossier d'installation <i>(optionnel)</i></span>
          <div class="field-row">
            <input v-model="installDir" type="text" placeholder="C:\Jeux\MonJeu" spellcheck="false" />
            <button type="button" class="btn-browse" @click="browseDir">Parcourir…</button>
          </div>
        </label>
        <label class="field">
          <span class="field-label">Jaquette <i>(optionnel)</i></span>
          <div class="field-row">
            <input v-model="coverUrl" type="text" placeholder="https://…/cover.jpg ou un fichier image" spellcheck="false" />
            <button type="button" class="btn-browse" @click="browseCover">Parcourir…</button>
          </div>
          <span class="field-hint">Une image de ton disque ou une adresse web. Sinon un dégradé est généré.</span>
        </label>

        <p v-if="error" class="modal-error">{{ error }}</p>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="closeAddGame">Annuler</button>
          <button type="submit" class="btn-save" :disabled="!canSave || saving">
            {{ saving ? "Enregistrement…" : editing ? "Enregistrer" : "Ajouter le jeu" }}
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
.field-row { display: flex; gap: 8px; align-items: stretch; }
.field-row input { flex: 1; min-width: 0; }
.btn-browse {
  flex: none; padding: 0 14px; border-radius: 10px; font-size: 12.5px; font-weight: 600; cursor: pointer;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim); white-space: nowrap;
}
.btn-browse:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-3); }
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
