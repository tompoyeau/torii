import { reactive, toRefs } from "vue";

/**
 * Gestion des mises à jour de l'application (plugin `updater` de Tauri 2).
 * L'app interroge le manifeste `latest.json` publié sur GitHub Releases ;
 * si une version plus récente signée existe, elle est téléchargée puis installée
 * et l'app redémarre. Tout est silencieux/no-op hors contexte Tauri (dev navigateur).
 */
type UpdateStatus =
  | "idle" // rien à signaler (ou pas encore vérifié)
  | "checking"
  | "available" // une mise à jour attend l'accord de l'utilisateur
  | "downloading"
  | "ready" // installée, redémarrage imminent
  | "error";

interface UpdaterState {
  status: UpdateStatus;
  /** Version proposée (ex : « 0.2.0 »). */
  version: string | null;
  /** Notes de version (corps du manifeste), si fournies. */
  notes: string | null;
  /** Progression du téléchargement (0 → 1), ou null si taille inconnue. */
  progress: number | null;
  error: string | null;
}

const state = reactive<UpdaterState>({
  status: "idle",
  version: null,
  notes: null,
  progress: null,
  error: null,
});

// L'objet Update en attente entre la vérification et l'installation.
let pending: import("@tauri-apps/plugin-updater").Update | null = null;

/**
 * Vérifie la présence d'une mise à jour. `silent` (défaut) n'affiche rien si
 * l'app est à jour ou hors Tauri — utilisé au démarrage.
 */
async function check(silent = true) {
  state.error = null;
  if (state.status === "downloading") return;
  state.status = "checking";
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (update) {
      pending = update;
      state.version = update.version;
      state.notes = update.body ?? null;
      state.status = "available";
    } else {
      state.status = "idle";
    }
  } catch (e) {
    // Hors Tauri (dev navigateur) : pas d'updater → on reste discret.
    pending = null;
    if (silent) {
      state.status = "idle";
    } else {
      state.status = "error";
      state.error = String(e);
    }
  }
}

/** Télécharge et installe la mise à jour en attente, puis redémarre l'app. */
async function install() {
  if (!pending) return;
  state.status = "downloading";
  state.progress = null;
  try {
    let total = 0;
    let downloaded = 0;
    await pending.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          total = event.data.contentLength ?? 0;
          state.progress = total ? 0 : null;
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          state.progress = total ? Math.min(1, downloaded / total) : null;
          break;
        case "Finished":
          state.progress = 1;
          break;
      }
    });
    state.status = "ready";
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch (e) {
    state.status = "error";
    state.error = String(e);
  }
}

/** Ferme la bannière sans installer (rappel à la prochaine ouverture de l'app). */
function dismiss() {
  if (state.status === "available") state.status = "idle";
}

export function useUpdater() {
  return { ...toRefs(state), check, install, dismiss };
}
