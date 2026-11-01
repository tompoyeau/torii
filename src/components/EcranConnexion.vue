<script setup lang="ts">
import { onMounted } from "vue";
import { useTorii } from "../composables/useTorii";

/**
 * Le fond de la version web tant qu'on n'est pas connecté. Le formulaire, lui, reste
 * `ToriiSignInDialog` : il ne doit exister qu'un seul parcours de connexion, et une seule étape
 * de pseudo impossible à contourner.
 */
const { openSignIn } = useTorii();
onMounted(openSignIn);
</script>

<template>
  <div class="ecran">
    <div class="marque">
      <svg viewBox="0 0 24 24" fill="#1a0f0c" aria-hidden="true">
        <path d="M6.6 7.3 L8.7 7.3 L9.1 19.6 L6.2 19.6 Z" />
        <path d="M15.3 7.3 L17.4 7.3 L17.8 19.6 L14.9 19.6 Z" />
        <rect x="11.2" y="8.9" width="1.6" height="2.6" />
        <rect x="4.5" y="11.1" width="15" height="2.1" rx="0.4" />
        <path d="M2.5 5 Q12 7.5 21.5 5 L21.5 7.4 Q12 9.9 2.5 7.4 Z" />
        <path d="M2.5 5 L1.3 4 L0.9 5.7 L2.5 7.4 Z" />
        <path d="M21.5 5 L22.7 4 L23.1 5.7 L21.5 7.4 Z" />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.ecran { min-height: 100vh; padding-top: 48px; display: flex; justify-content: center; }
.marque {
  width: 56px; height: 56px; border-radius: 16px; display: grid; place-items: center;
  background: linear-gradient(140deg, var(--accent), #ff9a6b); box-shadow: 0 10px 28px -8px var(--accent);
}
.marque svg { width: 32px; height: 32px; }
</style>
