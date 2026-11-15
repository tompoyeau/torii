<script setup lang="ts">
import { computed } from "vue";
import { useLibrary } from "../composables/useLibrary";
import { useUi } from "../composables/useUi";
import { useStore } from "../composables/useStore";
import TopBar from "./TopBar.vue";
import VueBibliotheque from "./VueBibliotheque.vue";
import StoreView from "./StoreView.vue";
import FriendsView from "./FriendsView.vue";
import CommonView from "./CommonView.vue";
import FriendProfileView from "./FriendProfileView.vue";
import FriendLibraryView from "./FriendLibraryView.vue";
import WishlistView from "./WishlistView.vue";
import PageWeb from "./PageWeb.vue";

const { games } = useLibrary();
const { section, selectedGameId } = useUi();
const { selectedGameId: produitOuvert } = useStore();

/**
 * Une fiche (jeu, Boutique) recouvre le contenu : ce qu'elle cache sort de la tabulation et
 * des lecteurs d'écran (`inert`). La barre du haut, hors de `<main>`, reste par-dessus la
 * fiche et joignable — c'est pourquoi les fiches n'enferment pas le focus.
 */
const contenuRecouvert = computed(
  () => produitOuvert.value != null || (selectedGameId.value != null && games.value.some((g) => g.id === selectedGameId.value)),
);

/**
 * 🔑 LES ONGLETS RESTENT EN MÉMOIRE (`<KeepAlive>`). Changer d'onglet détruisait l'écran quitté
 * et reconstruisait l'autre : revenir à la Bibliothèque refaisait toutes les cartes (~200 ms
 * pour 364 jeux, avant même les images), et Contacts rechargeait tout à chaque visite. Gardés,
 * ils reviennent tels qu'on les a laissés, défilement compris.
 *
 * Restent hors du cache : la page web (une vue native posée sur la fenêtre, qui doit disparaître
 * avec elle) et les pages d'un ami, qui dépendent de l'ami ouvert.
 */
const HORS_CACHE = ["PageWeb", "FriendProfileView", "FriendLibraryView"];
</script>

<template>
  <div class="shell">
    <TopBar />
    <main class="main" :class="{ 'sur-page': section === 'web' }" :inert="contenuRecouvert">
      <KeepAlive :exclude="HORS_CACHE">
        <PageWeb v-if="section === 'web'" />
        <StoreView v-else-if="section === 'store'" />
        <!-- Contacts : une seule page. Les amis, et les jeux en commun dans sa colonne principale ;
             la section `common` (historique) y mène et descend jusqu'à eux. -->
        <FriendsView v-else-if="section === 'friends' || section === 'common'">
          <CommonView />
        </FriendsView>
        <FriendProfileView v-else-if="section === 'friendProfile'" />
        <FriendLibraryView v-else-if="section === 'friendLibrary'" />
        <WishlistView v-else-if="section === 'wishlist'" />
        <VueBibliotheque v-else />
      </KeepAlive>
    </main>
  </div>
</template>

<style scoped>
.main { min-width: 0; padding: 0 34px 60px; }
/* La page web prend toute la hauteur sous la barre : pas de marge qui ferait défiler. */
.main.sur-page { padding-bottom: 0; }

@media (max-width: 820px) {
  .main {
    padding: 0 max(18px, env(safe-area-inset-right, 0px)) calc(40px + var(--barre-mobile)) max(18px, env(safe-area-inset-left, 0px));
  }
}
</style>
