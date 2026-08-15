<script setup lang="ts">
import { computed } from "vue";
import type { PlatformId } from "../types";

const props = defineProps<{ platform: PlatformId }>();

/**
 * Vrais logos des launchers, déposés dans `assets/launchers/<platform>.<ext>`
 * (png/svg/webp). Chargés à la compilation par Vite ; si un fichier existe pour
 * la plateforme, on affiche le vrai logo, sinon on retombe sur l'icône SVG dessinée.
 * Pour ajouter/remplacer un logo : glisser le fichier nommé d'après l'identifiant
 * de plateforme (steam/epic/gog/riot/ubisoft/ea/battlenet/manual).
 */
const assets = import.meta.glob("../assets/launchers/*.{png,svg,webp,jpg,jpeg}", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;
const realIcons: Partial<Record<PlatformId, string>> = {};
for (const [path, url] of Object.entries(assets)) {
  const name = path.split("/").pop()?.replace(/\.\w+$/, "");
  if (name) realIcons[name as PlatformId] = url;
}
const realIcon = computed(() => realIcons[props.platform]);

const ICONS: Record<PlatformId, string> = {
  steam:
    '<svg viewBox="0 0 24 24" fill="var(--steam)"><path d="M12 2a10 10 0 0 0-9.9 8.7l5.3 2.2a2.8 2.8 0 0 1 1.6-.5h.1l2.4-3.4v-.1a3.8 3.8 0 1 1 3.8 3.8h-.1l-3.4 2.4v.1a2.8 2.8 0 0 1-5.6.2l-3.8-1.6A10 10 0 1 0 12 2ZM8.4 17.2l-1.2-.5a2.1 2.1 0 0 0 3.9-.9 2.1 2.1 0 0 0-2.9-2l1.3.5a1.6 1.6 0 1 1-1.1 3Zm7.8-6.6a2.5 2.5 0 1 1 0-5.1 2.5 2.5 0 0 1 0 5.1Zm0-.9a1.6 1.6 0 1 0 0-3.2 1.6 1.6 0 0 0 0 3.2Z"/></svg>',
  epic:
    '<svg viewBox="0 0 24 24" fill="var(--epic)"><path d="M5 2h14a1 1 0 0 1 1 1v13.5l-8 4.5-8-4.5V3a1 1 0 0 1 1-1Zm3 4v9h1.6v-3.5h1.2a2.5 2.5 0 0 0 0-5H8Zm1.6 1.5h1a1 1 0 0 1 0 2h-1v-2Zm4 -1.5v9h4v-1.5h-2.4v-2.2H16v-1.4h-1.8V7.5H16.6V6h-3Z"/></svg>',
  gog:
    '<svg viewBox="0 0 24 24" fill="var(--gog)"><circle cx="12" cy="12" r="10"/><path d="M8 8.5h3.2v2.2H9.6v2.6h1.6v-1H10v-1.3h2.6V15H8V8.5Zm5.4 0H16v4.9h-1.3V10H14V15h-1.3V8.5h.7Z" fill="#14101c"/></svg>',
  // Poing blanc sur fond rouge (marque Riot).
  riot:
    '<svg viewBox="0 0 24 24"><rect x="1.5" y="1.5" width="21" height="21" rx="5" fill="#e23636"/><g fill="#fff"><rect x="7.55" y="7.6" width="2" height="4.3" rx="1"/><rect x="9.85" y="7" width="2" height="4.9" rx="1"/><rect x="12.15" y="7.2" width="2" height="4.7" rx="1"/><rect x="14.45" y="7.9" width="2" height="4" rx="1"/><rect x="7" y="10.6" width="10" height="6.9" rx="2.2"/><path d="M7.1 12.1c-1.2 0-2.1.6-2.1 1.5 0 .8.7 1.2 1.6 1.2.9 0 1.5-.5 1.5-1.4v-1.3z"/></g></svg>',
  // Launcher Ubisoft Connect : spirale blanche sur disque bleu (lisible en petit).
  ubisoft:
    '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9.6" fill="var(--ubisoft)"/><path d="M13.4 12A1.8 1.8 0 0 1 9.8 12 2.8 2.8 0 0 0 15.4 12 4.1 4.1 0 0 1 7.2 12" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round"/></svg>',
  ea:
    '<svg viewBox="0 0 24 24" fill="var(--ea)"><text x="12" y="16.5" font-size="11" font-weight="800" text-anchor="middle" font-family="Segoe UI, system-ui, sans-serif">EA</text></svg>',
  // Nouveau logo Battle.net : orbe bleue + volute blanche.
  battlenet:
    '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9.6" fill="var(--battlenet)"/><path d="M8.3 15.4c-1-2.9.5-6 3.5-6.6 1.9-.4 3.3.7 3.3 2.1 0 1.5-1.2 2.2-3 2.2M15.7 8.6c1 2.9-.5 6-3.5 6.6-1.9.4-3.3-.7-3.3-2.1 0-1.5 1.2-2.2 3-2.2" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round"/></svg>',
  manual:
    '<svg viewBox="0 0 24 24" fill="var(--manual)"><path d="M4 5a2 2 0 0 1 2-2h5l2 2h5a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5Zm8 4a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z"/></svg>',
};

const svg = computed(() => ICONS[props.platform]);
</script>

<template>
  <span class="platform-icon">
    <img v-if="realIcon" :src="realIcon" :alt="platform" class="platform-img" />
    <span v-else class="platform-svg" v-html="svg" />
  </span>
</template>

<style scoped>
.platform-icon {
  display: inline-flex;
  line-height: 0;
}
.platform-icon :deep(svg),
.platform-svg {
  width: 100%;
  height: 100%;
}
.platform-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
</style>
