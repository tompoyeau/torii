<script setup lang="ts">
import { useUi } from "../composables/useUi";

const { mode, setMode } = useUi();
</script>

<template>
  <div class="segmented">
    <button :class="{ on: mode === 'bureau' }" @click="setMode('bureau')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
        <rect x="3" y="4" width="18" height="14" rx="2" /><path d="M8 21h8" />
      </svg>
      Bureau
    </button>
    <button :class="{ on: mode === 'salon' }" @click="setMode('salon')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
        <path d="M4 8a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v6H4V8Z" /><path d="M2 14h20v3H2z" /><path d="M7 17v2M17 17v2" />
      </svg>
      Salon
    </button>
  </div>
</template>
