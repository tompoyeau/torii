<script setup lang="ts">
import { computed } from "vue";
import type { PlatformId } from "../types";

const props = defineProps<{ platform: PlatformId }>();

const ICONS: Record<PlatformId, string> = {
  steam:
    '<svg viewBox="0 0 24 24" fill="var(--steam)"><path d="M12 2a10 10 0 0 0-9.9 8.7l5.3 2.2a2.8 2.8 0 0 1 1.6-.5h.1l2.4-3.4v-.1a3.8 3.8 0 1 1 3.8 3.8h-.1l-3.4 2.4v.1a2.8 2.8 0 0 1-5.6.2l-3.8-1.6A10 10 0 1 0 12 2ZM8.4 17.2l-1.2-.5a2.1 2.1 0 0 0 3.9-.9 2.1 2.1 0 0 0-2.9-2l1.3.5a1.6 1.6 0 1 1-1.1 3Zm7.8-6.6a2.5 2.5 0 1 1 0-5.1 2.5 2.5 0 0 1 0 5.1Zm0-.9a1.6 1.6 0 1 0 0-3.2 1.6 1.6 0 0 0 0 3.2Z"/></svg>',
  epic:
    '<svg viewBox="0 0 24 24" fill="var(--epic)"><path d="M5 2h14a1 1 0 0 1 1 1v13.5l-8 4.5-8-4.5V3a1 1 0 0 1 1-1Zm3 4v9h1.6v-3.5h1.2a2.5 2.5 0 0 0 0-5H8Zm1.6 1.5h1a1 1 0 0 1 0 2h-1v-2Zm4 -1.5v9h4v-1.5h-2.4v-2.2H16v-1.4h-1.8V7.5H16.6V6h-3Z"/></svg>',
  gog:
    '<svg viewBox="0 0 24 24" fill="var(--gog)"><circle cx="12" cy="12" r="10"/><path d="M8 8.5h3.2v2.2H9.6v2.6h1.6v-1H10v-1.3h2.6V15H8V8.5Zm5.4 0H16v4.9h-1.3V10H14V15h-1.3V8.5h.7Z" fill="#14101c"/></svg>',
  riot:
    '<svg viewBox="0 0 24 24" fill="var(--riot)"><path d="M3.2 4.4 8 6l.5 3 1.3-2.5L12 7.3l.6-2.7L14.9 8l.6-3.1L20.8 7l-1.5 9.4-14.6.2L3.2 4.4Zm2 12.9 13.4-.2.1 1.9-13.2.7-.3-2.4Z"/></svg>',
  ubisoft:
    '<svg viewBox="0 0 24 24" fill="none" stroke="var(--ubisoft)" stroke-width="2" stroke-linecap="round"><path d="M4 12.5A8 8 0 0 1 12 4a8 8 0 0 1 8 8"/><path d="M7.5 14a4.5 4.5 0 0 1 4.5-4.5 4.5 4.5 0 0 1 4.5 4.5"/><circle cx="12" cy="14.5" r="1.4" fill="var(--ubisoft)" stroke="none"/></svg>',
  ea:
    '<svg viewBox="0 0 24 24" fill="var(--ea)"><text x="12" y="16.5" font-size="11" font-weight="800" text-anchor="middle" font-family="Segoe UI, system-ui, sans-serif">EA</text></svg>',
  battlenet:
    '<svg viewBox="0 0 24 24" fill="none" stroke="var(--battlenet)" stroke-width="1.7"><circle cx="12" cy="12" r="8.4"/><path d="M12 3.6c2.2 2.6 2.2 14.2 0 16.8M12 3.6c-2.2 2.6-2.2 14.2 0 16.8M3.6 12h16.8"/></svg>',
  manual:
    '<svg viewBox="0 0 24 24" fill="var(--manual)"><path d="M4 5a2 2 0 0 1 2-2h5l2 2h5a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5Zm8 4a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z"/></svg>',
};

const svg = computed(() => ICONS[props.platform]);
</script>

<template>
  <span class="platform-icon" v-html="svg" />
</template>

<style scoped>
.platform-icon {
  display: inline-flex;
  line-height: 0;
}
.platform-icon :deep(svg) {
  width: 100%;
  height: 100%;
}
</style>
