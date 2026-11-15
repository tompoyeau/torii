<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useUi } from "../composables/useUi";
import { useLibrary } from "../composables/useLibrary";
import { useFriends } from "../composables/useFriends";
import { useFriendList } from "../composables/useFriendList";
import { useTorii } from "../composables/useTorii";
import { useNavigation } from "../composables/useNavigation";
import { avecRaccourci } from "../composables/useRaccourcis";
import { MODE_WEB } from "../lib/mode";
import { steamMe } from "../lib/tauri";
import type { SteamProfile } from "../types";
import { t } from "../i18n";

const ui = useUi();
const { section, query, openAddGame, showFriends, openSettings, settingsOpen } = ui;

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
const SANS_RECHERCHE = ["store", "friendLibrary", "friendProfile", "common", "wishlist", "web"];
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
const { account: toriiAccount, connected: toriiConnected } = useTorii();
const { loading, reload } = useLibrary();
const { refresh: refreshFriends } = useFriends();
// La pastille compte les amis des DEUX sources, dédoublonnés.
const { activeCount: friendsOnline } = useFriendList();

// Identité de l'utilisateur (Steam) affichée dans l'en-tête. Masquée si non connecté.
const me = ref<SteamProfile | null>(null);
onMounted(async () => {
  if (MODE_WEB) return; // Ni Steam ni amis sur le web : le nom affiché est celui du compte Torii.
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
const { peutReculer, peutAvancer, libellePrecedent, libelleSuivant, reculer, avancer, quitterLesSurcouches, coucheDuDessus } = useNavigation();

/**
 * Une fiche (jeu, Boutique) est ouverte : la barre reste PAR-DESSUS, en verre dépoli, la
 * bannière passant dessous, pour changer de section ou revenir en arrière.
 */
const surFiche = computed(() => {
  const couche = coucheDuDessus();
  return couche === "jeu" || couche === "produit";
});

/**
 * Sa hauteur réelle, pour que les fiches posent leur bouton retour juste en dessous
 * (`--hauteur-barre`). Mesurée : elle change avec le zoom et la police.
 */
const barre = ref<HTMLElement | null>(null);
let observateur: ResizeObserver | null = null;
onMounted(() => {
  if (!barre.value) return;
  observateur = new ResizeObserver(([entree]) => {
    const hauteur = Math.round(entree.borderBoxSize?.[0]?.blockSize ?? entree.target.getBoundingClientRect().height);
    document.documentElement.style.setProperty("--hauteur-barre", `${hauteur}px`);
  });
  observateur.observe(barre.value);
});
onBeforeUnmount(() => observateur?.disconnect());

/**
 * Le champ filtre la bibliothèque, cachée sous la fiche : y entrer referme d'abord la fiche,
 * comme Ctrl+F, sinon on taperait sans rien voir changer.
 *
 * ⚠️ En se fermant, la fiche rend le focus à la carte qui l'avait ouverte (`useFocusTrap`) :
 * on le reprend au tour suivant, sans quoi la frappe partirait sur la carte.
 */
async function surFocusRecherche() {
  if (!surFiche.value) return;
  quitterLesSurcouches();
  await nextTick();
  champ.value?.focus();
}

/**
 * Les grandes sections, en onglets : c'est ici que vit la navigation depuis que la barre
 * latérale a disparu. Bibliothèque revient sur le filtre où on l'avait laissée.
 *
 * Changer de section referme d'abord ce qui est ouvert par-dessus (Paramètres…), en une
 * seule étape d'historique : le retour y ramène. Ordre = celui des raccourcis Ctrl + chiffre
 * (`useRaccourcis.ts`). Wishlist et En commun vivent sur le PC : pas sur le web.
 */
const ONGLETS = computed(() => [
  { id: "library", label: t("bibliotheque.laterale.bibliotheque"), touche: "1", aller: ui.showLibrary, tuto: "bibliotheque" },
  { id: "store", label: t("bibliotheque.laterale.boutique"), touche: "2", aller: ui.showStore, tuto: "boutique" },
  ...(MODE_WEB ? [] : [
    { id: "wishlist", label: t("bibliotheque.laterale.wishlist"), touche: "3", aller: ui.showWishlist, tuto: "wishlist" },
    { id: "common", label: t("bibliotheque.laterale.enCommun"), touche: null, aller: ui.showCommon, tuto: null },
  ]),
].map((o) => ({ ...o, titre: o.touche ? avecRaccourci(o.label, ctrl.value, o.touche) : o.label })));

function allerA(destination: () => void) {
  quitterLesSurcouches();
  destination();
}
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
  <div ref="barre" class="topbar" :class="{ web: MODE_WEB, 'sur-fiche': surFiche }">
    <!-- Sur téléphone, la barre latérale et son logo ne sont plus là. -->
    <img v-if="MODE_WEB" class="marque" src="/torii.svg" alt="Torii" />
    <div class="logo" aria-hidden="true">
      <svg viewBox="0 0 24 24" fill="currentColor">
        <path d="M6.6 7.3 L8.7 7.3 L9.1 19.6 L6.2 19.6 Z" />
        <path d="M15.3 7.3 L17.4 7.3 L17.8 19.6 L14.9 19.6 Z" />
        <rect x="11.2" y="8.9" width="1.6" height="2.6" />
        <rect x="4.5" y="11.1" width="15" height="2.1" rx="0.4" />
        <path d="M2.5 5 Q12 7.5 21.5 5 L21.5 7.4 Q12 9.9 2.5 7.4 Z" />
        <path d="M2.5 5 L1.3 4 L0.9 5.7 L2.5 7.4 Z" />
        <path d="M21.5 5 L22.7 4 L23.1 5.7 L21.5 7.4 Z" />
      </svg>
    </div>
    <!-- Sur téléphone, la barre du bas (`BarreMobile`) tient ce rôle. -->
    <nav class="onglets" :aria-label="t('web.navigation')">
      <button
        v-for="o in ONGLETS"
        :key="o.id"
        class="onglet"
        :class="{ on: section === o.id }"
        :aria-current="section === o.id ? 'page' : undefined"
        :aria-label="o.label"
        :title="o.titre"
        :data-tuto="o.tuto ?? undefined"
        @click="allerA(o.aller)"
      >
        <svg v-if="o.id === 'library'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" /></svg>
        <svg v-else-if="o.id === 'store'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M4 8h16l-1 4a3 3 0 0 1-3 2.4H8A3 3 0 0 1 5 12Z" /><path d="M4 8l1.4-3.4A2 2 0 0 1 7.2 3.4h9.6a2 2 0 0 1 1.8 1.2L20 8" /><path d="M6 14.4V20a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1v-5.6" /></svg>
        <svg v-else-if="o.id === 'wishlist'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M12 20s-7-4.3-7-9.3A3.7 3.7 0 0 1 12 8a3.7 3.7 0 0 1 7 2.7c0 5-7 9.3-7 9.3Z" /></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="8.5" cy="12" r="5.2" /><circle cx="15.5" cy="12" r="5.2" /></svg>
        <span class="onglet-nom">{{ o.label }}</span>
      </button>
    </nav>
    <!-- Sur une page web, ses propres flèches (dans sa barre) servent les deux historiques. -->
    <div v-if="section !== 'web'" class="historique">
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
      <input ref="champ" v-model="query" data-recherche type="text" @focus="surFocusRecherche" :placeholder="t('bibliotheque.barre.rechercher')" autocomplete="off" />
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
    <button
      v-if="!MODE_WEB"
      class="icon-btn"
      :title="avecRaccourci(t('bibliotheque.barre.ajouterJeu'), ctrl, 'N')"
      :aria-label="t('bibliotheque.barre.ajouterJeu')"
      @click="openAddGame"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
    </button>
    <button v-if="!MODE_WEB" class="icon-btn friends-btn" data-tuto="amis" :class="{ active: section === 'friends' }" :title="avecRaccourci(t('bibliotheque.barre.amis'), ctrl, '4')" @click="allerA(showFriends)">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 5.2a3 3 0 0 1 0 5.6M17.5 20a5.5 5.5 0 0 0-3-4.9" /></svg>
      <span v-if="friendsOnline" class="friends-badge">{{ friendsOnline }}</span>
    </button>
    <button class="icon-btn resynchro" :class="{ 'hors-biblio': !chercheDansLaBiblio }" :title="avecRaccourci(t('bibliotheque.barre.resynchroniser'), 'F5')" :aria-label="t('bibliotheque.barre.resynchroniser')" :disabled="loading" @click="reload()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
    </button>
    <button class="icon-btn reglages" :class="{ active: settingsOpen }" :title="avecRaccourci(t('bibliotheque.barre.parametres'), ctrl, ',')" :aria-label="t('bibliotheque.barre.parametres')" @click="openSettings()">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><circle cx="12" cy="12" r="3.1" /><path d="M19.4 13a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5v.2a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1.1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H2a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1.1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H8a1.6 1.6 0 0 0 1-1.5V2a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V8a1.6 1.6 0 0 0 1.5 1h.2a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" /></svg>
    </button>
    <button class="me" :title="t('bibliotheque.barre.monCompteTorii', { nom: myName })" :aria-label="t('bibliotheque.barre.monCompteTorii', { nom: myName })" @click="openMyAccount">
      <img v-if="me?.avatarUrl" class="me-avatar" :src="me.avatarUrl" :alt="myName" />
      <span v-else class="me-avatar fallback">{{ myInitials }}</span>
      <!-- Pastille discrète : le compte Torii est actif, la présence peut circuler. Le nom
           est dans l'infobulle : la barre porte désormais les onglets. -->
      <span v-if="toriiConnected" class="me-torii" :title="t('bibliotheque.barre.toriiConnecte')">⛩</span>
    </button>
  </div>
</template>

<style scoped>
/* Du verre dépoli sur toute sa hauteur, sur toutes les vues : la même teinte et le même liseré
   que le bouton retour des fiches (`.detail-back`). Ce qui défile dessous passe en flou ; flou
   plus fort que le bouton, car derrière une barre entière, c'est souvent du texte. */
.topbar {
  position: sticky; top: 0; z-index: 20; display: flex; align-items: center; gap: 12px;
  padding: 18px 34px 16px;
  background: rgba(12, 8, 18, 0.5); backdrop-filter: blur(16px) saturate(140%);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}
:root[data-theme="light"] .topbar { background: rgba(255, 255, 255, 0.6); border-bottom-color: var(--border); }
/* Les commandes de la barre laissent voir le verre au lieu de faire des pavés opaques. */
.topbar :is(.icon-btn, .search input, .onglets, .me) {
  background: color-mix(in srgb, var(--surface) 45%, transparent);
  border-color: rgba(255, 255, 255, 0.16);
}
:root[data-theme="light"] .topbar :is(.icon-btn, .search input, .onglets, .me) { border-color: var(--border); }
.topbar :is(.icon-btn, .me):hover { border-color: var(--border-strong); }
/* Firefox (version web) n'a pas de flou (`style.css`) : une teinte plus dense garde la barre lisible. */
@supports (-moz-appearance: none) {
  .topbar { background: rgba(12, 8, 18, 0.88); }
  :root[data-theme="light"] .topbar { background: rgba(255, 255, 255, 0.9); }
}
/* Par-dessus les fiches (100 et 110), sous les Paramètres (200). */
.topbar.sur-fiche { z-index: 150; }
.logo {
  width: 38px; height: 38px; border-radius: 11px; flex: none;
  background: linear-gradient(140deg, var(--accent), #ff9a6b); color: var(--accent-ink);
  display: grid; place-items: center; box-shadow: 0 6px 18px -6px var(--accent);
}
.logo svg { width: 21px; height: 21px; }

/* Les onglets des sections : une commande segmentée, l'onglet courant en couleur d'accent. */
.onglets {
  display: flex; gap: 2px; padding: 3px; flex: none;
  border-radius: 13px; border: 1px solid var(--border); background: var(--surface);
}
.onglet {
  display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px;
  border: none; border-radius: 10px; background: none; color: var(--text-dim);
  font-size: 13px; font-weight: 600; font-family: inherit; cursor: pointer;
  transition: color 0.15s, background 0.15s;
}
.onglet svg { width: 16px; height: 16px; flex: none; }
.onglet:hover { color: var(--text); background: var(--surface-2); }
.onglet.on { background: var(--accent); color: var(--accent-ink); }
/* Fenêtre étroite (940 px au minimum) : les icônes seules, le nom passe dans l'infobulle.
   Le champ de recherche garde ainsi plus de 250 px. */
@media (max-width: 1180px) {
  .onglet { padding: 0 10px; }
  .onglet-nom { display: none; }
  .enrich-pill { padding: 6px; font-size: 0; gap: 0; }
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
@container (max-width: 320px) {
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
  flex: none; width: 38px; height: 38px; border-radius: 11px; border: 1px solid var(--border);
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
.me {
  position: relative; flex: none; display: grid; place-items: center; width: 38px; height: 38px; padding: 0;
  border-radius: 50%; border: 1px solid var(--border); background: var(--surface);
  color: var(--text-dim); font-family: inherit; cursor: pointer;
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
.me-torii {
  position: absolute; right: -4px; bottom: -3px; width: 16px; height: 16px; border-radius: 50%;
  display: grid; place-items: center; background: var(--bg); color: var(--accent); font-size: 10px; line-height: 1;
}
.marque { display: none; }

/* Web sur téléphone : le logo et le compte, le reste est dans la barre du bas. La barre du haut
   défile avec la page : l'écran est trop petit pour lui laisser sa hauteur en permanence.
   Le retour passe par le geste du téléphone et par le bouton des fiches. */
@media (max-width: 820px) {
  /* Les mêmes marges que la page (`.main`, `AppShell.vue`). */
  .topbar { padding-inline: max(18px, env(safe-area-inset-left, 0px)) max(18px, env(safe-area-inset-right, 0px)); }
  /* La barre défile avec la page : pas de verre, il n'y aurait rien dessous. */
  .topbar.web, :root[data-theme="light"] .topbar.web {
    position: static; gap: 8px; padding-block: 12px; background: none; backdrop-filter: none; border-bottom: none;
  }
  .topbar.web .marque { display: block; width: 34px; height: 34px; flex: none; }
  .topbar .logo,
  .topbar .onglets,
  .topbar.web .historique,
  .topbar.web .hint,
  .topbar.web .reglages,
  .topbar.web .resynchro.hors-biblio,
  .topbar.web .me-torii { display: none; }
  .topbar.web .search + .topbar-spacer { display: none; }
  .topbar.web .search { flex: 1 1 0; max-width: none; }
  .topbar.web .search input { padding-right: 38px; }
  /* Seul le rond qui tourne : le texte prendrait la place du champ. */
  .topbar.web .enrich-pill { padding: 6px; font-size: 0; gap: 0; }
}
</style>
