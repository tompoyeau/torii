<script setup lang="ts">
import { useTorii } from "../composables/useTorii";

/**
 * Invitation à rejoindre le réseau Torii.
 *
 * Ce composant ne contient plus le formulaire : la connexion et l'inscription se passent
 * dans `ToriiSignInDialog`, une fenêtre unique montée à la racine de l'application. Deux
 * écrans affichent cette invitation (Amis et Réglages) ; il ne doit exister qu'un seul
 * parcours d'inscription, et une seule étape de pseudo impossible à contourner.
 */

const { account, connected, openSignIn } = useTorii();
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
    <button class="btn-primary" @click="openSignIn">Créer un compte ou se connecter</button>
  </div>
</template>

<style scoped>
.signed { font-size: 13px; color: var(--text-faint); margin: 0 0 16px; }
.signed strong { color: var(--text); font-weight: 600; }

.invite {
  display: flex; flex-direction: column; gap: 14px; align-items: flex-start;
  padding: 18px 20px; margin-bottom: 22px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
}
.pitch { display: flex; flex-direction: column; gap: 4px; }
.pitch .title { font-size: 15px; font-weight: 700; letter-spacing: -0.01em; }
.pitch .sub { font-size: 13px; color: var(--text-dim); line-height: 1.5; max-width: 60ch; }

.btn-primary {
  padding: 9px 16px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
  font-family: inherit;
}
.btn-primary:hover { background: var(--accent-hover); }
</style>
