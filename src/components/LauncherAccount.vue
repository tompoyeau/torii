<script setup lang="ts">
/**
 * Carte d'un compte de launcher dans les Paramètres : état de connexion, texte
 * d'explication et boutons (se connecter / resynchroniser / déconnecter).
 *
 * Les cinq launchers connectables partagent exactement cette carte ; seuls le nom,
 * la couleur, les textes et le comportement de la resynchronisation changent — d'où
 * les props. Le slot `extra` sert au chemin avancé de Steam (clé API).
 */
defineProps<{
  /** Nom affiché du launcher (« Epic Games », « Battle.net »…). */
  name: string;
  /** Couleur de la pastille (variable CSS de la plateforme). */
  color: string;
  connected: boolean;
  /** Une opération est en cours (connexion, resync, déconnexion). */
  busy: boolean;
  /** Explication affichée tant que le compte n'est pas connecté. */
  hint: string;
  /** Confirmation affichée une fois connecté. */
  syncedHint: string;
  /** Libellé du bouton de connexion (« Se connecter avec GOG »). */
  connectLabel: string;
  /**
   * Libellé du bouton « Resynchroniser » pendant l'opération. EA et Battle.net
   * repassent par la connexion pour se resynchroniser (pas de refresh silencieux),
   * d'où un libellé d'attente différent.
   */
  resyncBusyLabel?: string;
  /** Dernier message d'état (succès, erreur), affiché sous la carte. */
  message: string;
}>();

defineEmits<{
  connect: [];
  resync: [];
  disconnect: [];
}>();
</script>

<template>
  <div class="account">
    <div class="account-top">
      <span class="dot" :style="{ background: color }" />
      <span class="account-name">{{ name }}</span>
      <span class="badge" :class="connected ? 'on' : ''">
        {{ connected ? "Connecté" : "Non connecté" }}
      </span>
    </div>

    <template v-if="connected">
      <p class="hint">{{ syncedHint }}</p>
      <div class="row">
        <button class="btn-primary" :disabled="busy" @click="$emit('resync')">
          {{ busy && resyncBusyLabel ? resyncBusyLabel : "Resynchroniser" }}
        </button>
        <button class="btn-secondary" :disabled="busy" @click="$emit('disconnect')">
          Déconnecter
        </button>
      </div>
    </template>

    <template v-else>
      <p class="hint">{{ hint }}</p>
      <div class="row">
        <button class="btn-primary" :disabled="busy" @click="$emit('connect')">
          {{ busy ? "En attente de connexion…" : connectLabel }}
        </button>
      </div>
      <!-- Chemin avancé propre à un launcher (clé API Steam). -->
      <slot name="extra" />
    </template>

    <p v-if="message" class="message">{{ message }}</p>
  </div>
</template>

<style scoped>
.account {
  border: 1px solid var(--border); border-radius: 12px; padding: 11px 13px; margin-bottom: 7px;
  background: var(--surface-2);
}
.account-top { display: flex; align-items: center; gap: 9px; }
.account-top .dot { width: 8px; height: 8px; border-radius: 50%; }
.account-name { font-weight: 600; font-size: 13.5px; }
.badge {
  margin-left: auto; font-size: 10.5px; font-family: var(--mono); padding: 2px 8px; border-radius: 99px;
  background: var(--surface-3); color: var(--text-faint);
}
.badge.on { background: color-mix(in srgb, var(--manual) 20%, transparent); color: var(--manual); }
.hint { font-size: 12px; line-height: 1.45; color: var(--text-dim); margin: 7px 0; }
.row { display: flex; gap: 7px; flex-wrap: wrap; align-items: center; }
.btn-primary {
  padding: 8px 15px; border-radius: 9px; border: none; background: var(--accent);
  color: var(--accent-ink); font-weight: 700; font-size: 12.5px;
}
.btn-primary:disabled { opacity: 0.6; cursor: default; }
.btn-secondary {
  padding: 8px 13px; border-radius: 9px; border: 1px solid var(--border); background: var(--surface);
  color: var(--text-dim); font-weight: 600; font-size: 12.5px;
}
.btn-secondary:disabled { opacity: 0.5; cursor: default; }
.message { font-size: 12px; color: var(--accent); margin: 9px 0 0; line-height: 1.4; }
</style>
