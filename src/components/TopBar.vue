<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useUi } from "../composables/useUi";
import { useTheme } from "../composables/useTheme";
import { useLibrary } from "../composables/useLibrary";
import { useFriends } from "../composables/useFriends";
import { useFriendList } from "../composables/useFriendList";
import { useTorii } from "../composables/useTorii";
import { useNavigation } from "../composables/useNavigation";
import { avecRaccourci } from "../composables/useRaccourcis";
import { steamMe } from "../lib/tauri";
import type { SteamProfile } from "../types";
import { t } from "../i18n";

const { section, query, openAddGame, showFriends, openSettings, settingsOpen } = useUi();

/**
 * Sections où la recherche du bandeau n'a rien à faire.
 *
 * 🔑 Elle filtre **TA** bibliothèque : sur un écran qui n'affiche pas tes jeux, y taper ne
 * produit rien de visible, ce qui se lit comme une panne. La Boutique était déjà écartée
 * pour cette raison (elle a sa propre barre) ; la bibliothèque d'un ami a la sienne aussi,
 * et sa page profil ne montre rien qui se filtre.
 *
 * « En commun » et la Wishlist s'y ajoutent : elles listent respectivement les jeux de tes
 * amis et ceux que tu convoites — deux jeux de données que le magasin de la bibliothèque
 * ne contient pas. Chacune a désormais **sa propre** recherche, comme la Boutique.
 */
const SANS_RECHERCHE = ["store", "friendLibrary", "friendProfile", "common", "wishlist"];
const chercheDansLaBiblio = computed(() => !SANS_RECHERCHE.includes(section.value));

/**
 * Vide la recherche d un geste, et RESTITUE LE FOCUS au champ : sans ca, effacer au
 * clavier laisse le focus sur une croix qui vient de disparaitre, donc nulle part.
 * (La croix n existe que sur ce champ : les recherches locales sont en `type=search`,
 * ou Chromium dessine la sienne.)
 */
const champ = ref<HTMLInputElement | null>(null);
function effacerRecherche() {
  query.value = "";
  champ.value?.focus();
}
const { toggle: toggleTheme } = useTheme();
const { account: toriiAccount, connected: toriiConnected } = useTorii();
const { loading, reload } = useLibrary();
const { refresh: refreshFriends } = useFriends();
// La pastille compte les amis des DEUX sources, dédoublonnés.
const { activeCount: friendsOnline } = useFriendList();

// Identité de l'utilisateur (Steam) affichée dans l'en-tête. Masquée si non connecté.
const me = ref<SteamProfile | null>(null);
onMounted(async () => {
  me.value = await steamMe();
  // Charge les amis dès le démarrage pour alimenter le badge de l'en-tête.
  refreshFriends();
});
/**
 * Le bouton d'identité mène aux réglages du compte Torii — c'est là que se règlent le
 * pseudo, ce que voient les amis et le lien avec Steam. Avant, il ouvrait le profil
 * Steam dans le navigateur : sortir de l'application n'est pas ce qu'on attend en
 * cliquant sur son propre nom.
 */
function openMyAccount() {
  openSettings("torii");
}

/**
 * Précédent / suivant, comme dans un navigateur. L'infobulle dit où mène chaque flèche
 * (« Revenir à : Boutique ») : les deux piles ne se voient pas autrement.
 */
const { peutReculer, peutAvancer, libellePrecedent, libelleSuivant, reculer, avancer } = useNavigation();
const ctrl = computed(() => t("reglages.raccourcis.touches.ctrl"));
const alt = computed(() => t("reglages.raccourcis.touches.alt"));
const titrePrecedent = computed(() =>
  avecRaccourci(
    libellePrecedent.value ? t("bibliotheque.barre.precedentVers", { lieu: libellePrecedent.value }) : t("bibliotheque.barre.precedent"),
    alt.value, "←",
  ),
);
const titreSuivant = computed(() =>
  avecRaccourci(
    libelleSuivant.value ? t("bibliotheque.barre.suivantVers", { lieu: libelleSuivant.value }) : t("bibliotheque.barre.suivant"),
    alt.value, "→",
  ),
);

/** Ce qu'on affiche : le pseudo Torii s'il existe, sinon le nom Steam. */
const myName = computed(() => toriiAccount.value?.displayName ?? me.value?.name ?? t("bibliotheque.barre.monCompte"));
const myInitials = computed(() => myName.value.trim().slice(0, 2).toUpperCase());
</script>

<template>
  <div class="topbar">
    <div class="historique">
      <button
        class="icon-btn"
        :disabled="!peutReculer"
        :title="titrePrecedent"
        :aria-label="titrePrecedent"
        @click="reculer()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
      <button
        class="icon-btn"
        :disabled="!peutAvancer"
        :title="titreSuivant"
        :aria-label="titreSuivant"
        @click="avancer()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
      </button>
    </div>
    <label v-if="chercheDansLaBiblio" class="search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7" /><path d="m20 20-3-3" /></svg>
      <input ref="champ" v-model="query" data-recherche type="text" :placeholder="t('bibliotheque.barre.rechercher')" autocomplete="off" />
      <!-- Rappel du raccourci tant que le champ est vide : c'est là qu'on le cherche. -->
      <span v-if="!query" class="hint" aria-hidden="true"><kbd>{{ t("reglages.raccourcis.touches.ctrl") }}</kbd><kbd>F</kbd></span>
      <button
        v-if="query"
        type="button"
        class="clear"
        :title="t('bibliotheque.barre.effacer')"
        :aria-label="t('bibliotheque.barre.effacer')"
        @click="effacerRecherche"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </label>
    <div class="topbar-spacer" />
    <span v-if="loading" class="enrich-pill">
      <span class="spinner" />{{ t("bibliotheque.barre.actualisation") }}
    </span>
    <button class="add-btn" :title="avecRaccourci(t('bibliotheque.barre.ajouterJeu'), ctrl, 'N')" @click="openAddGame">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
      <span>{{ t("bibliotheque.barre.ajouter") }}</span>
    </button>
    <button class="icon-btn friends-btn" :class="{ active: section === 'friends' }" :title="avecRaccourci(t('bibliotheque.barre.amis'), ctrl, '4')" @click="showFriends">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 5.2a3 3 0 0 1 0 5.6M17.5 20a5.5 5.5 0 0 0-3-4.9" /></svg>
      <span v-if="friendsOnline" class="friends-badge">{{ friendsOnline }}</span>
    </button>
    <button class="icon-btn" :title="avecRaccourci(t('bibliotheque.barre.resynchroniser'), 'F5')" :disabled="loading" @click="reload()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
    </button>
    <button class="icon-btn" :title="t('bibliotheque.barre.theme')" @click="toggleTheme">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M20 14.5A8 8 0 1 1 9.5 4 6.5 6.5 0 0 0 20 14.5Z" /></svg>
    </button>
    <button class="icon-btn" :class="{ active: settingsOpen }" :title="avecRaccourci(t('bibliotheque.barre.parametres'), ctrl, ',')" @click="openSettings()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="3.1" /><path d="M19.4 13a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5v.2a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1.1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H2a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1.1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H8a1.6 1.6 0 0 0 1-1.5V2a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V8a1.6 1.6 0 0 0 1.5 1h.2a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" /></svg>
    </button>
    <button class="me" :title="t('bibliotheque.barre.monCompteTorii', { nom: myName })" @click="openMyAccount">
      <img v-if="me?.avatarUrl" class="me-avatar" :src="me.avatarUrl" :alt="myName" />
      <span v-else class="me-avatar fallback">{{ myInitials }}</span>
      <span class="me-name">{{ myName }}</span>
      <!-- Pastille discrète : le compte Torii est actif, la présence peut circuler. -->
      <span v-if="toriiConnected" class="me-torii" :title="t('bibliotheque.barre.toriiConnecte')">⛩</span>
    </button>
  </div>
</template>

<style scoped>
.topbar {
  position: sticky; top: 0; z-index: 20; display: flex; align-items: center; gap: 14px;
  padding: 18px 0 16px;
  background: linear-gradient(var(--bg) 60%, transparent); backdrop-filter: blur(6px);
}
/* Base de 420 px, et pas seulement un maximum : à parts égales avec l'espaceur, le champ
   tombait sous les 250 px une fois les flèches ajoutées, et le texte d'invite passait
   sous le rappel « Ctrl F ». Il ne rétrécit que si la barre manque de place. */
.search { flex: 1 1 420px; min-width: 0; max-width: 420px; position: relative; display: flex; align-items: center; }
/* ⚠️ Enfant DIRECT : la loupe. Sans le `>`, la regle attrapait aussi le SVG de la croix
   et l envoyait se coller a gauche, par-dessus la loupe. */
.search > svg { position: absolute; left: 13px; width: 16px; height: 16px; color: var(--text-faint); }
.search .clear {
  position: absolute; right: 8px;
  display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px;
  background: none; border: none; color: var(--text-faint);
}
.search .clear svg { width: 15px; height: 15px; }
.search .clear:hover { background: var(--surface-2); color: var(--text); }
.search input {
  /* La place de la croix (champ rempli) ou du rappel « Ctrl F » (champ vide) est reservee
     en permanence : sans ca le texte sauterait au moment ou l un remplace l autre,
     c est-a-dire des la premiere lettre tapee. */
  width: 100%; padding: 10px 64px 10px 38px; background: var(--surface);
  border: 1px solid var(--border); border-radius: 11px; color: var(--text);
  font-size: 13.5px; font-family: inherit;
}
.search input::placeholder { color: var(--text-faint); }
/* Rappel « Ctrl F », à la place de la croix (qui n'apparaît qu'une fois le champ rempli). */
.search .hint {
  position: absolute; right: 10px; display: inline-flex; gap: 3px; pointer-events: none;
}
.search .hint kbd {
  padding: 1px 5px; border-radius: 5px; border: 1px solid var(--border); background: var(--surface-2);
  font-family: var(--mono); font-size: 10.5px; color: var(--text-faint);
}
.search input:focus ~ .hint { display: none; }
/* Fenêtre étroite (940 px au minimum) : le champ n'a plus la place des deux, le texte
   d'invite passe avant le rappel. */
.search { container-type: inline-size; }
@container (max-width: 280px) {
  .search .hint { display: none; }
  .search input { padding-right: 42px; }
}
.historique { display: flex; gap: 6px; flex: none; }
.historique .icon-btn:disabled { opacity: 0.4; cursor: default; }
.historique .icon-btn:disabled:hover { color: var(--text-dim); border-color: var(--border); }
.search input:focus { border-color: var(--border-strong); background: var(--surface-2); }
.topbar-spacer { flex: 1; }
.enrich-pill {
  display: inline-flex; align-items: center; gap: 8px; padding: 6px 12px; border-radius: 99px;
  background: var(--accent-soft); color: var(--accent); font-size: 12px; font-weight: 600;
  font-family: var(--mono);
}
.spinner {
  width: 12px; height: 12px; border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-top-color: var(--accent); animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.icon-btn {
  width: 38px; height: 38px; border-radius: 11px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text-dim); display: grid; place-items: center;
  transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover { color: var(--text); border-color: var(--border-strong); }
.icon-btn svg { width: 17px; height: 17px; }
.icon-btn.active {
  color: var(--accent); border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  background: var(--accent-soft);
}
.friends-btn { position: relative; }
.friends-badge {
  position: absolute; top: -5px; right: -5px; min-width: 17px; height: 17px; padding: 0 4px;
  border-radius: 99px; background: #4bbe6b; color: #08210f; font-size: 10px; font-weight: 700;
  font-family: var(--mono); display: grid; place-items: center;
  border: 2px solid var(--bg); box-sizing: border-box;
}
.add-btn {
  display: inline-flex; align-items: center; gap: 7px; height: 38px; padding: 0 15px 0 12px;
  border-radius: 11px; border: 1px solid var(--border); background: var(--surface); color: var(--text-dim);
  font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.add-btn:hover { color: var(--text); border-color: var(--border-strong); }
.add-btn svg { width: 16px; height: 16px; }
.me {
  display: inline-flex; align-items: center; gap: 9px; height: 38px; padding: 0 12px 0 5px;
  border-radius: 99px; border: 1px solid var(--border); background: var(--surface);
  color: var(--text-dim); font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.me:hover { color: var(--text); border-color: var(--border-strong); }
.me-avatar {
  width: 28px; height: 28px; border-radius: 50%; flex: none; object-fit: cover;
  background: var(--surface-3);
}
/* Sans photo Steam, on affiche les initiales : un bouton d'identité sans visage
   ressemble à un bouton cassé. */
.me-avatar.fallback {
  display: grid; place-items: center;
  font-size: 11.5px; font-weight: 700; color: var(--text-dim); letter-spacing: 0.02em;
}
.me-name { max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.me-torii { font-size: 11px; color: var(--accent); margin-left: -2px; }
</style>
