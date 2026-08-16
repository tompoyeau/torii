<script setup lang="ts">
import { useToast } from "../composables/useToast";

const { toasts } = useToast();
</script>

<template>
  <div class="toasts">
    <transition-group name="toast">
      <div v-for="t in toasts" :key="t.id" class="toast">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M20 6 9 17l-5-5" /></svg>
        <span>{{ t.message }}</span>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toasts {
  position: fixed; left: 0; right: 0; bottom: 26px; z-index: 300;
  display: flex; flex-direction: column; align-items: center; gap: 8px;
  pointer-events: none;
}
.toast {
  display: inline-flex; align-items: center; gap: 9px;
  background: var(--surface); color: var(--text); border: 1px solid var(--border);
  border-radius: 12px; padding: 11px 16px; font-size: 13.5px; font-weight: 600;
  box-shadow: var(--shadow-hero);
}
.toast svg { width: 17px; height: 17px; color: var(--accent); flex: none; }

.toast-enter-active, .toast-leave-active { transition: opacity 0.2s ease, transform 0.2s ease; }
.toast-enter-from { opacity: 0; transform: translateY(10px); }
.toast-leave-to { opacity: 0; transform: translateY(10px); }
</style>
