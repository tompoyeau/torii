<script setup lang="ts">
/**
 * La bibliothèque partagée d'un ami : ce qu'il **possède**, tous launchers confondus.
 *
 * Le tri par défaut met en avant ce qu'il a et que tu n'as pas — c'est la question qu'on
 * se pose en ouvrant la bibliothèque de quelqu'un (« à quoi je pourrais jouer / qu'est-ce
 * qu'il me conseille »), pas l'inventaire de ce qu'on possède déjà tous les deux.
 */
import { computed, ref, watch } from "vue";
import { useFriendLibrary } from "../composables/useFriendLibrary";
import { useFriendList } from "../composables/useFriendList";
import { useStore } from "../composables/useStore";
import { useUi } from "../composables/useUi";
import { gradientFor } from "../lib/covers";
import { platformName } from "../data/platforms";
import GameCard from "./GameCard.vue";
import PlatformIcon from "./PlatformIcon.vue";
import type { Game, LibGame, PlatformId } from "../types";

const { friendLibraryId, showFriendProfile, showStore, openGame } = useUi();
const { games, loading, error, loadedId, devicesOf, load, mine, commonCount } = useFriendLibrary();
const { inGame, online, offline } = useFriendList();

/** L'ami concerné, retrouvé dans la liste unifiée (avatar, pseudo, état). */
const friend = computed(() => {
  const id = friendLibraryId.value;
  if (!id) return null;
  return [...inGame.value, ...online.value, ...offline.value].find((f) => f.toriiId === id) ?? null;
});

/** Nom de repli : l'index porte le pseudo même quand la personne n'est pas dans le cercle. */
const friendName = computed(
  () => friend.value?.name ?? devicesOf(friendLibraryId.value ?? "")[0]?.displayName ?? "Cet ami",
);

const devices = computed(() => devicesOf(friendLibraryId.value ?? ""));

// Charge dès qu'on ouvre une bibliothèque, et à chaque changement d'ami.
watch(
  friendLibraryId,
  (id) => {
    if (id && id !== loadedId.value) void load(id);
  },
  { immediate: true },
);

const query = ref("");
type Filtre = "tous" | "manquants" | "communs";
const filtre = ref<Filtre>("tous");

const FILTRES: { key: Filtre; label: string }[] = [
  { key: "tous", label: "Tous" },
  { key: "manquants", label: "Que tu n'as pas" },
  { key: "communs", label: "Vous l'avez tous les deux" },
];

const shown = computed(() => {
  const q = query.value.trim().toLowerCase();
  return games.value.filter((g) => {
    if (q && !g.title.toLowerCase().includes(q)) return false;
    if (filtre.value === "communs") return !!mine(g);
    if (filtre.value === "manquants") return !mine(g);
    return true;
  });
});

/**
 * Carte à afficher. Si tu possèdes le jeu, c'est TA fiche qui est montrée (jaquette,
 * temps de jeu, bouton Jouer au clic) ; sinon une carte synthétique à partir du peu que
 * l'ami a partagé — même mécanique que `CommonView`.
 */
function cardFor(g: LibGame): Game {
  const found = mine(g);
  if (found) return found;
  return {
    id: `friend:${g.key}`,
    title: g.title,
    platform: g.platforms[0] ?? "manual",
    cover: gradientFor(g.key),
    coverUrl: g.cover ?? undefined,
    installed: false,
  } as Game;
}

/**
 * Un jeu qu'on possède ouvre SA fiche ; un jeu qu'on n'a pas ouvre son **comparatif de
 * prix**, par-dessus la bibliothèque de l'ami.
 *
 * 🔑 C'est la seule suite qui a du sens ici : on parcourt la bibliothèque de quelqu'un
 * précisément pour trouver ce qu'on n'a pas, et jusqu'ici le clic ne faisait rien du tout.
 * Repli identique à `GameDetail.viewInStore` : sans correspondance exacte, `openForTitle`
 * bascule en recherche, et il faut alors montrer la Boutique où sont les résultats.
 *
 * `useStore()` est appelé ici et pas au montage de la vue : l'instancier charge la vitrine,
 * et ouvrir la bibliothèque d'un ami n'est pas une raison d'aller chercher des promotions.
 */
async function onOpen(g: LibGame) {
  const found = mine(g);
  if (found) {
    openGame(found.id);
    return;
  }
  if (!(await useStore().openForTitle(g.title))) showStore();
}

function platformsLabel(g: LibGame): string {
  return g.platforms.map((p) => platformName(p as never)).join(", ");
}

/** « il y a 3 j » — une bibliothèque partagée peut dater, et ça se dit. */
function sinceLabel(timestamp: number): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - timestamp);
  if (s < 3600) return "à l'instant";
  if (s < 86400) return `il y a ${Math.round(s / 3600)} h`;
  return `il y a ${Math.round(s / 86400)} j`;
}

/** La plus récente des mises à jour, tous appareils confondus. */
const updatedAt = computed(() =>
  devices.value.reduce((max, d) => Math.max(max, d.updatedAt), 0),
);

/**
 * Jeux empruntés au groupe familial Steam. Le dire dans l'en-tête évite de laisser croire
 * qu'un compteur de 464 jeux décrit 464 achats : une partie appartient à quelqu'un d'autre
 * et peut disparaître du jour au lendemain.
 */
const familyCount = computed(() => games.value.filter((g) => g.familyShared).length);
</script>

<template>
  <div class="friend-lib">
    <div class="sec-head">
      <!-- Retour au PROFIL et non à la liste : depuis qu'une personne a sa page dans
           Torii, c'est elle le parent de sa bibliothèque. Revenir aux amis d'un bond
           sauterait un étage et perdrait la personne qu'on était en train de regarder. -->
      <button
        class="chip back"
        :title="`Retour au profil de ${friendName}`"
        @click="showFriendProfile(`torii:${friendLibraryId}`)"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M15 6l-6 6 6 6" />
        </svg>
        Profil
      </button>
      <h2>Bibliothèque de {{ friendName }}</h2>
      <span v-if="games.length" class="n">{{ games.length }} jeu{{ games.length > 1 ? "x" : "" }}</span>
    </div>

    <p v-if="devices.length" class="sub">
      {{ commonCount }} en commun avec toi ·
      <template v-if="familyCount">
        {{ familyCount }} via le partage familial Steam ·
      </template>
      {{ devices.length }} appareil{{ devices.length > 1 ? "s" : "" }} ·
      mis à jour {{ sinceLabel(updatedAt) }}
    </p>

    <!-- Recherche et filtres sur la même ligne : ils font le même travail — restreindre
         la liste — et les séparer donnait deux barres d'outils pour une seule intention. -->
    <div v-if="games.length" class="filters">
      <input v-model="query" class="search" type="search" placeholder="Rechercher…" />
      <button
        v-for="f in FILTRES"
        :key="f.key"
        class="chip"
        :class="{ active: filtre === f.key }"
        @click="filtre = f.key"
      >
        {{ f.label }}
      </button>
    </div>

    <p v-if="loading" class="empty">Chargement de sa bibliothèque…</p>
    <p v-else-if="error" class="empty" role="alert">{{ error }}</p>
    <p v-else-if="!games.length" class="empty">
      {{ friendName }} ne partage pas sa bibliothèque.
    </p>
    <p v-else-if="!shown.length" class="empty">
      Aucun jeu ne correspond.
    </p>

    <div v-else class="grid">
      <div v-for="g in shown" :key="g.key" class="cell">
        <!-- `:actions="false"` : ni étoile, ni œil barré, ni clic droit. On est chez
             quelqu'un d'autre — ces gestes touchent NOTRE bibliothèque et n'ont rien à
             faire sur ses jaquettes. -->
        <GameCard :game="cardFor(g)" :actions="false" @open="onOpen(g)" />
        <div class="note">
          <span class="plats" :title="platformsLabel(g)">
            <PlatformIcon v-for="p in g.platforms" :key="p" :platform="(p as PlatformId)" />
          </span>
          <span
            v-if="g.familyShared"
            class="family"
            :title="`${friendName} y a accès par le partage familial Steam : le jeu ne lui appartient pas.`"
          >
            Famille Steam
          </span>
          <span v-if="mine(g)" class="owned">Tu l'as aussi</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.friend-lib { min-width: 0; }
.sec-head { display: flex; align-items: center; gap: 12px; margin-bottom: 6px; }
.sec-head h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.sec-head .n { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
.back { display: inline-flex; align-items: center; gap: 6px; }
.back svg { width: 16px; height: 16px; }
.search {
  width: 220px; padding: 7px 12px; border-radius: 10px;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text);
  font-size: 13px;
}
.search:focus { border-color: var(--accent); }
.sub { font-size: 12.5px; color: var(--text-dim); margin: 0 0 14px; }
.filters { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 18px; }
.empty { color: var(--text-dim); font-size: 14px; padding: 28px 0; }

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(155px, 1fr)); gap: 22px 16px; }
.cell { min-width: 0; }
.note { display: flex; align-items: center; gap: 8px; margin-top: 6px; min-height: 18px; }
.plats { display: inline-flex; align-items: center; gap: 4px; color: var(--text-faint); }
/* ⚠️ Dimensionner `.platform-icon` et pas `svg` : certaines plateformes (Epic, Ubisoft)
   rendent une image, pas un SVG, et l'icône se dimensionne sur son conteneur. Viser le
   `svg` laissait le PNG Epic s'afficher en pleine taille. */
.plats :deep(.platform-icon) { width: 14px; height: 14px; }
/* Le jeu que vous avez tous les deux : c'est l'information qu'on cherche en parcourant
   la bibliothèque de quelqu'un, elle mérite la couleur d'accent. */
.owned {
  font-size: 11px; font-weight: 600; color: var(--accent);
  background: color-mix(in srgb, var(--accent) 14%, transparent);
  border-radius: 99px; padding: 2px 8px; white-space: nowrap;
}
/* Emprunté à la famille Steam : bleu Steam, et non la couleur d'accent — ce n'est pas
   une bonne nouvelle à souligner, c'est une nuance de propriété à ne pas confondre avec
   « il l'a ». */
.family {
  font-size: 11px; font-weight: 600; color: var(--steam);
  background: color-mix(in srgb, var(--steam) 14%, transparent);
  border-radius: 99px; padding: 2px 8px; white-space: nowrap;
}
</style>
