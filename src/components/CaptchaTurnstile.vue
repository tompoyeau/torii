<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { chargerTurnstile, type Turnstile } from "../lib/turnstile";
import { langueCourante, t } from "../i18n";

/**
 * Le contrôle anti-robot de la connexion web. Émet le jeton à présenter au serveur, ou `null`
 * tant qu'il n'y en a pas de valable : un jeton expire au bout de cinq minutes.
 *
 * ⚠️ UN JETON NE SERT QU'UNE FOIS. La page ne se recharge pas après un refus : le parent appelle
 * `reinitialiser()` pour en obtenir un nouveau avant le prochain essai.
 */
const emit = defineEmits<{ (e: "jeton", valeur: string | null): void }>();

const conteneur = ref<HTMLElement | null>(null);
const echec = ref(false);
let turnstile: Turnstile | null = null;
let widget: string | null = null;
let demonte = false;

function enEchec() {
  echec.value = true;
  emit("jeton", null);
}

async function monter() {
  echec.value = false;
  const cle = import.meta.env.VITE_TURNSTILE_SITE_KEY;
  if (!cle) {
    console.error("[torii] VITE_TURNSTILE_SITE_KEY manquant");
    enEchec();
    return;
  }
  try {
    turnstile = await chargerTurnstile();
  } catch {
    enEchec();
    return;
  }
  if (demonte || !conteneur.value) return;
  widget = turnstile.render(conteneur.value, {
    sitekey: cle,
    action: "connexion",
    // L'interface a son propre thème, qui ne suit pas toujours celui du système.
    theme: document.documentElement.getAttribute("data-theme") ?? "auto",
    language: langueCourante.value,
    size: "flexible",
    callback: (jeton) => {
      echec.value = false;
      emit("jeton", jeton);
    },
    "expired-callback": () => emit("jeton", null),
    "timeout-callback": () => emit("jeton", null),
    "error-callback": enEchec,
  });
}

/** Un nouveau jeton, après que le précédent a servi. */
function reinitialiser() {
  emit("jeton", null);
  if (turnstile && widget) turnstile.reset(widget);
}

function reessayer() {
  if (turnstile && widget) {
    echec.value = false;
    turnstile.reset(widget);
  } else {
    void monter();
  }
}

defineExpose({ reinitialiser });

onMounted(() => void monter());
onBeforeUnmount(() => {
  demonte = true;
  if (turnstile && widget) turnstile.remove(widget);
});
</script>

<template>
  <div class="captcha">
    <div ref="conteneur" />
    <p v-if="echec" class="echec" role="alert">
      {{ t("web.captcha.echec") }}
      <button type="button" class="lien" @click="reessayer">{{ t("web.captcha.reessayer") }}</button>
    </p>
  </div>
</template>

<style scoped>
.captcha { min-height: 65px; }
.echec { margin: 6px 0 0; font-size: 12px; line-height: 1.5; color: var(--text-dim); }
.lien { padding: 0; border: none; background: none; color: var(--accent); font: inherit; cursor: pointer; text-decoration: underline; }
</style>
