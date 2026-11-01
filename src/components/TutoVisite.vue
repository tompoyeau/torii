<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useFocusTrap } from "../composables/useFocusTrap";
import { useModaleDeNavigation, useNavigation } from "../composables/useNavigation";
import { useTuto } from "../composables/useTuto";
import { useUi } from "../composables/useUi";
import { t } from "../i18n";

/**
 * La visite : une bulle sur chaque endroit à connaître, éléments repérés par `data-tuto`.
 *
 * 🔑 ELLE MONTRE, ELLE N'OUVRE RIEN. Afficher la Boutique pour la présenter coûterait des appels au
 * comparateur, dont le quota est partagé par tout le monde, à chaque nouvel utilisateur.
 */
const { terminer } = useTuto();
const { quitterLesSurcouches } = useNavigation();
const { showLibrary } = useUi();

const ETAPES = computed(() =>
  (["bibliotheque", "boutique", "wishlist", "amis"] as const).map((cible) => ({
    cible,
    titre: t(`tuto.visite.${cible}.titre`),
    texte: t(`tuto.visite.${cible}.texte`),
  })),
);

/** Une fenêtre trop étroite cache la barre latérale : on saute ce qu'on ne peut pas montrer. */
const elementDe = (cible: string) => {
  const el = document.querySelector<HTMLElement>(`[data-tuto="${cible}"]`);
  return el && el.offsetParent !== null ? el : null;
};

const visibles = ref<{ cible: string; titre: string; texte: string }[]>([]);
const index = ref(0);
const etape = computed(() => visibles.value[index.value] ?? null);
const derniere = computed(() => index.value >= visibles.value.length - 1);

const bulle = ref<HTMLElement | null>(null);
const bouton = ref<HTMLElement | null>(null);
const trou = ref<{ left: number; top: number; width: number; height: number } | null>(null);
const position = ref({ left: 0, top: 0 });

const MARGE = 6;
const ECART = 16;

/** Le trou autour de la cible, et la bulle à sa droite ou, faute de place, en dessous. */
async function placer() {
  const el = etape.value && elementDe(etape.value.cible);
  if (!el) return;
  el.scrollIntoView({ block: "nearest" });
  const r = el.getBoundingClientRect();
  trou.value = { left: r.left - MARGE, top: r.top - MARGE, width: r.width + 2 * MARGE, height: r.height + 2 * MARGE };
  await nextTick();
  const largeur = bulle.value?.offsetWidth ?? 340;
  const hauteur = bulle.value?.offsetHeight ?? 180;
  const borne = (v: number, max: number) => Math.max(ECART, Math.min(v, max - ECART));
  const aDroite = r.right + MARGE + ECART + largeur <= window.innerWidth - ECART;
  position.value = aDroite
    ? { left: r.right + MARGE + ECART, top: borne(r.top, window.innerHeight - hauteur) }
    : { left: borne(r.right - largeur, window.innerWidth - largeur), top: borne(r.bottom + MARGE + ECART, window.innerHeight - hauteur) };
  // Entrée fait toujours avancer, et le focus ne se perd pas quand « Passer » disparaît.
  bouton.value?.focus();
}

function suivant() {
  if (derniere.value) {
    terminer();
    return;
  }
  index.value++;
  void placer();
}

useFocusTrap(bulle, computed(() => trou.value !== null));
useModaleDeNavigation({ ouverte: () => true, fermer: terminer });

onMounted(async () => {
  // Un fond connu, et rien par-dessus la barre du haut.
  quitterLesSurcouches();
  showLibrary();
  await nextTick();
  visibles.value = ETAPES.value.filter((e) => elementDe(e.cible));
  if (!visibles.value.length) {
    terminer();
    return;
  }
  await placer();
  window.addEventListener("resize", placer);
});
onBeforeUnmount(() => window.removeEventListener("resize", placer));
</script>

<template>
  <!-- Le voile ne laisse rien cliquer : on suit la visite ou on la passe. -->
  <div class="voile" @keydown.esc.stop="terminer">
    <div v-if="trou" class="trou" :style="{ left: `${trou.left}px`, top: `${trou.top}px`, width: `${trou.width}px`, height: `${trou.height}px` }" />
    <div
      v-if="etape && trou"
      ref="bulle"
      class="bulle"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      :aria-label="etape.titre"
      :style="{ left: `${position.left}px`, top: `${position.top}px` }"
    >
      <h3 aria-live="polite">{{ etape.titre }}</h3>
      <p>{{ etape.texte }}</p>
      <div class="pied">
        <span class="numero">{{ t("tuto.visite.bulle", { n: index + 1, total: visibles.length }) }}</span>
        <span class="espace" />
        <button v-if="!derniere" class="discret" @click="terminer">{{ t("tuto.passer") }}</button>
        <button ref="bouton" class="btn-play suivant" @click="suivant">{{ derniere ? t("tuto.visite.terminer") : t("tuto.suivant") }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.voile { position: fixed; inset: 0; z-index: 240; }
/* Tout ce qui entoure le trou est assombri par son ombre. */
.trou {
  position: fixed; border-radius: 12px; box-shadow: 0 0 0 9999px rgba(6, 4, 10, 0.72), 0 0 0 2px var(--accent);
  transition: left 0.25s, top 0.25s, width 0.25s, height 0.25s; pointer-events: none;
}
.bulle {
  position: fixed; width: min(340px, calc(100vw - 32px)); padding: 18px 20px 16px; border-radius: 16px;
  background: var(--surface); border: 1px solid var(--border-strong); box-shadow: var(--shadow-hero); outline: none;
  transition: left 0.25s, top 0.25s;
}
h3 { margin: 0 0 6px; font-size: 17px; font-weight: 800; letter-spacing: -0.02em; }
p { margin: 0 0 14px; font-size: 13.5px; line-height: 1.55; color: var(--text-dim); }
.pied { display: flex; align-items: center; gap: 12px; }
.espace { flex: 1; }
.numero { font-family: var(--mono); font-size: 11.5px; color: var(--text-faint); }
.discret { background: none; border: none; padding: 6px 0; font-size: 12.5px; color: var(--text-faint); }
.discret:hover { color: var(--text-dim); }
.suivant { padding: 9px 18px; font-size: 13.5px; box-shadow: none; }
</style>
