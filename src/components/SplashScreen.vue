<script setup lang="ts">
import { useLibrary } from "../composables/useLibrary";

const { booted } = useLibrary();
</script>

<template>
  <transition name="splash">
    <div v-if="!booted" class="splash">
      <div class="mark">
        <svg viewBox="0 0 24 24" fill="#1a0f0c">
          <path d="M6.6 7.3 L8.7 7.3 L9.1 19.6 L6.2 19.6 Z" />
          <path d="M15.3 7.3 L17.4 7.3 L17.8 19.6 L14.9 19.6 Z" />
          <rect x="11.2" y="8.9" width="1.6" height="2.6" />
          <rect x="4.5" y="11.1" width="15" height="2.1" rx="0.4" />
          <path d="M2.5 5 Q12 7.5 21.5 5 L21.5 7.4 Q12 9.9 2.5 7.4 Z" />
          <path d="M2.5 5 L1.3 4 L0.9 5.7 L2.5 7.4 Z" />
          <path d="M21.5 5 L22.7 4 L23.1 5.7 L21.5 7.4 Z" />
        </svg>
      </div>
      <div class="name">Torii</div>
      <div class="spin" />
      <div class="msg">Chargement de ta bibliothèque…</div>
    </div>
  </transition>
</template>

<style scoped>
.splash {
  position: fixed; inset: 0; z-index: 400;
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px;
  background: var(--bg); background-image: var(--bg-grad);
}
.mark {
  width: 76px; height: 76px; border-radius: 22px;
  background: linear-gradient(140deg, var(--accent), #ff9a6b);
  display: grid; place-items: center; box-shadow: 0 14px 40px -12px var(--accent);
  animation: pop 0.5s ease;
}
.mark svg { width: 42px; height: 42px; }
.name { font-size: 22px; font-weight: 700; letter-spacing: -0.02em; color: var(--text); }
.spin {
  margin-top: 6px; width: 22px; height: 22px; border-radius: 50%;
  border: 3px solid color-mix(in srgb, var(--accent) 28%, transparent);
  border-top-color: var(--accent); animation: spin 0.7s linear infinite;
}
.msg { font-size: 13px; color: var(--text-faint); font-family: var(--mono); }

@keyframes spin { to { transform: rotate(360deg); } }
@keyframes pop { from { transform: scale(0.85); opacity: 0.4; } to { transform: scale(1); opacity: 1; } }

.splash-leave-active { transition: opacity 0.35s ease; }
.splash-leave-to { opacity: 0; }
</style>
