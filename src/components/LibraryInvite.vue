<script setup lang="ts">
/**
 * Invitation, **une seule fois**, à partager sa bibliothèque avec ses amis.
 *
 * Le partage est éteint par défaut, et c'est volontaire : ce qu'on possède ne quitte pas
 * sa machine parce qu'on a installé une mise à jour. Mais un interrupteur enfoui dans les
 * Réglages que rien ne signale est un interrupteur que personne n'active — d'où cette
 * proposition, à l'endroit exact où elle a un sens (la vue Amis).
 *
 * 🔑 Une proposition se fait une fois. « Non merci » est définitif (mémorisé dans les
 * préférences), sur le même principe que `steam_auto_linked` côté Rust : un défaut se
 * propose, il ne se réimpose pas.
 *
 * ⚠️ N'apparaît que si la synchronisation n'a **jamais** été activée. Quelqu'un qui
 * synchronise pour son mobile sans partager avec ses amis a fait un choix délibéré dans
 * les Réglages ; ce n'est pas à ce bandeau de le remettre en question.
 */
import { ref } from "vue";
import { usePreferences } from "../composables/usePreferences";
import { useTorii } from "../composables/useTorii";
import { showToast } from "../composables/useToast";

const { prefs } = usePreferences();
const { prefs: toriiPrefs, connected, setLibrarySync, setShareLibrary, librarySyncing } = useTorii();

const error = ref<string | null>(null);

/**
 * 🔑 Les deux usages sont proposés **séparément**, parce qu'ils le sont réellement :
 * *synchroniser* dépose la bibliothèque sur son compte (pour la retrouver sur ses autres
 * appareils), *partager* autorise en plus les amis à la lire. Ne proposer que le second
 * obligeait à passer par les Réglages pour découvrir qu'on pouvait avoir l'un sans
 * l'autre — et laissait croire qu'emporter sa bibliothèque impliquait de la montrer.
 */
async function activer(avecLesAmis: boolean) {
  error.value = null;
  try {
    await setLibrarySync(true);
    if (avecLesAmis) await setShareLibrary(true);
    showToast(
      avecLesAmis
        ? "Ta bibliothèque est partagée avec tes amis Torii."
        : "Ta bibliothèque est synchronisée. Tes amis ne la voient pas.",
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function refuser() {
  // `prefs` est réactif et persisté par un `watch` : l'écriture directe suffit.
  prefs.libraryInviteDismissed = true;
}
</script>

<template>
  <div
    v-if="connected && !toriiPrefs.syncLibrary && !prefs.libraryInviteDismissed"
    class="invite"
  >
    <div class="pitch">
      <span class="title">Ta bibliothèque, ailleurs que sur ce PC</span>
      <span class="sub">
        Torii peut la déposer sur ton compte pour que tu la retrouves sur tes autres
        appareils — et, si tu le veux, la montrer à tes amis quel que soit leur launcher.
        Les deux se règlent séparément. Les jeux masqués et ceux que tu ne diffuses pas en
        sont exclus dans tous les cas, et tu peux tout couper d'un clic.
      </span>
    </div>
    <div class="actions">
      <button class="btn-primary" :disabled="librarySyncing" @click="activer(true)">
        {{ librarySyncing ? "Envoi…" : "Partager avec mes amis" }}
      </button>
      <button class="btn-second" :disabled="librarySyncing" @click="activer(false)">
        Seulement mes appareils
      </button>
      <button class="btn-ghost" @click="refuser">Non merci</button>
    </div>
    <p v-if="error" class="err" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.invite {
  /* `stretch` et non `flex-start` : sans ça, le bloc de texte se rétrécit au contenu et
     ne prend pas la largeur du bandeau. */
  display: flex; flex-direction: column; gap: 14px; align-items: stretch;
  padding: 18px 20px; margin-bottom: 22px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
}
.pitch { display: flex; flex-direction: column; gap: 4px; }
.pitch .title { font-size: 15px; font-weight: 700; letter-spacing: -0.01em; }
/* Pas de `max-width` : le texte suit la largeur du bandeau. Une colonne de 60 caractères
   laissait une bande vide à droite, alors que le bandeau, lui, va jusqu'au bord. */
.pitch .sub { font-size: 13px; color: var(--text-dim); line-height: 1.5; }
.actions { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }

.btn-primary {
  padding: 9px 16px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
  font-family: inherit;
}
.btn-primary:hover { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.6; cursor: default; }

/* Deuxième chemin, aussi légitime que le premier : visible, sans voler l'accent. */
.btn-second {
  padding: 9px 16px; border-radius: 10px; cursor: pointer; font-family: inherit;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text);
  font-weight: 600; font-size: 13.5px;
}
.btn-second:hover { border-color: var(--text-faint); }
.btn-second:disabled { opacity: 0.6; cursor: default; }

.btn-ghost {
  padding: 9px 14px; border-radius: 10px; cursor: pointer; font-family: inherit;
  background: none; border: 1px solid var(--border); color: var(--text-dim);
  font-size: 13.5px;
}
.btn-ghost:hover { color: var(--text); border-color: var(--text-faint); }
.err { font-size: 12.5px; color: #ff6b6b; margin: 0; }
</style>
