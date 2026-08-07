<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useUpdater } from "../composables/useUpdater";

const { status, version, notes, progress, error, check, install, dismiss } = useUpdater();

// Vérifie une mise à jour au lancement de l'app (silencieux si à jour / hors Tauri).
onMounted(() => check(true));

const visible = computed(() =>
  ["available", "downloading", "ready", "error"].includes(status.value),
);
const pct = computed(() => (progress.value != null ? Math.round(progress.value * 100) : null));
</script>

<template>
  <transition name="update-pop">
    <div v-if="visible" class="update-banner" :class="status">
      <!-- Mise à jour disponible : attente de l'accord utilisateur. -->
      <template v-if="status === 'available'">
        <div class="ub-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v12m0 0l-4-4m4 4l4-4" /><path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" /></svg>
        </div>
        <div class="ub-body">
          <div class="ub-title">Mise à jour disponible</div>
          <div class="ub-sub">Torii {{ version }} est prêt à être installé.</div>
          <p v-if="notes" class="ub-notes">{{ notes }}</p>
        </div>
        <div class="ub-actions">
          <button class="ub-btn primary" @click="install">Installer et redémarrer</button>
          <button class="ub-btn ghost" @click="dismiss">Plus tard</button>
        </div>
      </template>

      <!-- Téléchargement en cours. -->
      <template v-else-if="status === 'downloading'">
        <div class="ub-icon spin">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-2.64-6.36" /></svg>
        </div>
        <div class="ub-body">
          <div class="ub-title">Téléchargement de Torii {{ version }}…</div>
          <div class="ub-bar"><div class="ub-fill" :style="{ width: (pct ?? 0) + '%' }" /></div>
          <div class="ub-sub">{{ pct != null ? pct + " %" : "En cours…" }}</div>
        </div>
      </template>

      <!-- Prêt : redémarrage imminent. -->
      <template v-else-if="status === 'ready'">
        <div class="ub-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 6L9 17l-5-5" /></svg>
        </div>
        <div class="ub-body">
          <div class="ub-title">Mise à jour installée</div>
          <div class="ub-sub">Redémarrage de Torii…</div>
        </div>
      </template>

      <!-- Erreur. -->
      <template v-else>
        <div class="ub-icon err">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16h.01" /></svg>
        </div>
        <div class="ub-body">
          <div class="ub-title">Échec de la mise à jour</div>
          <div class="ub-sub ellipsis" :title="error ?? ''">{{ error }}</div>
        </div>
        <div class="ub-actions">
          <button class="ub-btn ghost" @click="dismiss">Fermer</button>
        </div>
      </template>
    </div>
  </transition>
</template>

<style scoped>
.update-banner {
  position: fixed; right: 22px; bottom: 22px; z-index: 280; width: 340px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 16px;
  box-shadow: var(--shadow-hero); padding: 16px; display: flex; gap: 13px; align-items: flex-start;
}
.ub-icon {
  width: 38px; height: 38px; border-radius: 11px; flex: none; display: grid; place-items: center;
  background: var(--accent-soft); color: var(--accent);
}
.ub-icon svg { width: 20px; height: 20px; }
.ub-icon.spin svg { animation: ub-spin 0.9s linear infinite; }
.ub-icon.err { background: color-mix(in srgb, #ff6b6b 15%, transparent); color: #ff6b6b; }
@keyframes ub-spin { to { transform: rotate(360deg); } }
.ub-body { flex: 1; min-width: 0; }
.ub-title { font-size: 14px; font-weight: 700; letter-spacing: -0.01em; }
.ub-sub { font-size: 12.5px; color: var(--text-faint); margin-top: 3px; }
.ub-sub.ellipsis { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ub-notes {
  font-size: 12px; color: var(--text-dim); margin: 8px 0 0; line-height: 1.5;
  max-height: 72px; overflow-y: auto; white-space: pre-line;
}
.ub-actions { display: flex; flex-direction: column; gap: 6px; flex: none; }
.ub-btn { padding: 8px 12px; border-radius: 9px; font-size: 12.5px; font-weight: 600; cursor: pointer; white-space: nowrap; }
.ub-btn.primary { background: var(--accent); color: var(--accent-ink); border: 1px solid transparent; }
.ub-btn.primary:hover { background: var(--accent-hover); }
.ub-btn.ghost { background: none; border: 1px solid var(--border); color: var(--text-dim); }
.ub-btn.ghost:hover { color: var(--text); border-color: var(--border-strong); }
.ub-bar { height: 6px; border-radius: 99px; background: var(--surface-3); overflow: hidden; margin: 8px 0 4px; }
.ub-fill { height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--accent), #ffb08a); transition: width 0.2s; }

.update-pop-enter-active, .update-pop-leave-active { transition: opacity 0.25s, transform 0.25s; }
.update-pop-enter-from, .update-pop-leave-to { opacity: 0; transform: translateY(12px); }
</style>
