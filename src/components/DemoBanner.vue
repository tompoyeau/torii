<script setup lang="ts">
import { ref } from "vue";
import { hasTauriRuntime } from "../lib/tauri";

/**
 * Bandeau de la démo en ligne.
 *
 * 🔑 POURQUOI CE COMPOSANT EXISTE. Le même code tourne à deux endroits : dans
 * l'application installée, et dans un navigateur sur torii-app.fr, où il sert de démo
 * jouable. Dans le navigateur, la bibliothèque est inventée et tout ce qui appelle le
 * natif — lancer un jeu, connecter un compte, écrire un réglage — ne fait rien. Sans
 * bandeau, un visiteur clique « Jouer », rien ne se passe, et il en conclut que
 * l'application est cassée. Le dire franchement coûte une ligne.
 *
 * 🔑 Le test est `hasTauriRuntime()`, pas une variable d'environnement de compilation :
 * la démo est le MÊME build que la préversion de développement (`npm run dev`), donc
 * aucun drapeau ne les distingue. Ce qui les distingue, c'est la présence du pont natif.
 * Corollaire utile : le bandeau ne peut pas apparaître par erreur dans l'application
 * installée, puisque le pont y est toujours là.
 */
const visible = ref(!hasTauriRuntime());
</script>

<template>
  <div v-if="visible" class="demo-banner" role="status">
    <span class="pastille">Démo</span>
    <p>
      Tu essaies Torii dans ton navigateur, sur une bibliothèque inventée.
      <span class="dim">Lancer un jeu ou connecter un compte demande l'application.</span>
    </p>
    <a class="lien" href="../#telecharger">Télécharger Torii</a>
    <button class="fermer" title="Masquer ce bandeau" aria-label="Masquer ce bandeau"
            @click="visible = false">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4">
        <path d="M6 6l12 12M18 6L6 18" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
/* Le bandeau se pose AU-DESSUS de l'interface plutôt que de la décaler : décaler
   obligerait à retoucher toutes les hauteurs calculées de l'application, alors que ce
   composant n'existe que hors de celle-ci. */
.demo-banner {
  position: fixed;
  left: 50%;
  bottom: 18px;
  transform: translateX(-50%);
  z-index: 900;
  display: flex;
  align-items: center;
  gap: 14px;
  max-width: min(760px, calc(100vw - 32px));
  padding: 11px 12px 11px 16px;
  border: 1px solid var(--border-strong, rgba(255, 255, 255, 0.15));
  border-radius: 14px;
  background: var(--surface, #1b1823);
  box-shadow: 0 22px 48px -18px rgba(0, 0, 0, 0.75);
  animation: monte 0.5s 0.4s cubic-bezier(0.2, 0.8, 0.2, 1) both;
}

.pastille {
  flex: none;
  padding: 3px 9px;
  border-radius: 999px;
  background: var(--accent, #ff6b57);
  color: var(--accent-ink, #1a0f0c);
  font-size: 11.5px;
  font-weight: 800;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.demo-banner p { margin: 0; font-size: 13.5px; line-height: 1.45; }
.dim { color: var(--text-dim, #a39daf); }

.lien {
  flex: none;
  padding: 8px 14px;
  border-radius: 10px;
  background: var(--accent, #ff6b57);
  color: var(--accent-ink, #1a0f0c);
  font-size: 13px;
  font-weight: 700;
  text-decoration: none;
  white-space: nowrap;
}
.lien:hover { filter: brightness(1.08); }

.fermer {
  flex: none;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim, #a39daf);
  cursor: pointer;
}
.fermer:hover { background: var(--surface-2, #232030); color: var(--text, #ece9f3); }
.fermer svg { width: 15px; height: 15px; }

@keyframes monte {
  from { opacity: 0; transform: translate(-50%, 14px); }
  to { opacity: 1; transform: translate(-50%, 0); }
}

/* Sur mobile, le bandeau prend deux lignes plutôt que d'écraser son texte. */
@media (max-width: 620px) {
  .demo-banner { flex-wrap: wrap; gap: 9px 12px; bottom: 10px; }
  .demo-banner p { flex: 1 1 100%; order: 3; }
}

@media (prefers-reduced-motion: reduce) {
  .demo-banner { animation: none; }
}
</style>
