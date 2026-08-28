<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useFriends } from "../composables/useFriends";
import { useFriendList, type UnifiedFriend } from "../composables/useFriendList";
import type { ToriiStatus } from "../types";
import { useLibrary } from "../composables/useLibrary";
import { useTorii } from "../composables/useTorii";
import { useUi } from "../composables/useUi";
import { useFriendLibrary } from "../composables/useFriendLibrary";
import { showToast } from "../composables/useToast";
import { openExternal } from "../lib/tauri";
import ToriiPanel from "./ToriiPanel.vue";
import LibraryInvite from "./LibraryInvite.vue";

const { loaded, steamConnected, refresh } = useFriends();
const { inGame, online, offline, activeCount, loading } = useFriendList();
const {
  account, circle, connected: toriiConnected, presenceMode,
  refresh: refreshTorii, invite, respond, setPresenceMode, removeFriend,
} = useTorii();
const { launchOrInstall } = useLibrary();
const { openSettings, openGame, showFriendLibrary } = useUi();
const { hasLibrary, devicesOf, refreshIndex } = useFriendLibrary();

/**
 * Cet ami partage-t-il sa bibliothèque ? Le bouton n'apparaît que là où il mène quelque
 * part : proposer « voir sa bibliothèque » pour tomber sur un écran vide serait pire que
 * de ne rien proposer du tout.
 */
function canSeeLibrary(f: UnifiedFriend): boolean {
  return hasLibrary(f.toriiId);
}

function libraryHint(f: UnifiedFriend): string {
  const total = devicesOf(f.toriiId ?? "").reduce((n, d) => n + d.gameCount, 0);
  return `Voir les ${total} jeux que ${f.name} possède, tous launchers confondus.`;
}

/** La présence Torii arrive seule (battement de cœur) ; seul Steam doit être sondé. */
let timer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  void refresh();
  void refreshTorii();
  // Un ami a pu activer le partage depuis le démarrage : on relit l'index en arrivant
  // ici, plutôt que de le sonder en boucle pour une donnée qui bouge une fois par mois.
  void refreshIndex();
  timer = setInterval(() => void refresh(), 60_000);
});
onBeforeUnmount(() => clearInterval(timer));

/* ── Ajout d'un ami ────────────────────────────────────────────────────────── */

const addOpen = ref(false);
const friendCode = ref("");
const addBusy = ref(false);
const addError = ref<string | null>(null);

async function onInvite() {
  const code = friendCode.value.trim();
  if (!code || addBusy.value) return;
  addBusy.value = true;
  addError.value = null;
  try {
    await invite(code);
    friendCode.value = "";
    addOpen.value = false;
    showToast("Demande envoyée.");
  } catch (e) {
    addError.value = e instanceof Error ? e.message : String(e);
  } finally {
    addBusy.value = false;
  }
}

async function copyCode() {
  if (!account.value) return;
  try {
    await navigator.clipboard.writeText(account.value.friendCode);
    showToast("Ton code d'ami est copié.");
  } catch {
    showToast("Copie impossible ; note le code à la main.");
  }
}

/* ── Affichage d'un ami ────────────────────────────────────────────────────── */

function stateLabel(f: UnifiedFriend): string {
  switch (f.state) {
    case "online": return "En ligne";
    case "away": return "Absent";
    case "in-game": return f.gameName ?? "En jeu";
    default: return "Hors ligne";
  }
}

/**
 * Pourquoi cette personne est-elle « hors ligne » ? Pour un ami Torii, ça veut dire
 * qu'il n'a pas l'application ouverte — pas qu'il ne joue pas. On ne l'affiche qu'en
 * infobulle : c'est une nuance utile, pas une information de premier plan.
 */
function offlineHint(f: UnifiedFriend): string {
  if (f.source === "both") {
    return "Hors ligne sur Steam et Torii fermé : elle joue peut-être sans qu'on le voie.";
  }
  return f.source === "torii"
    ? "Cette personne n'a pas Torii ouvert : elle joue peut-être sans qu'on le voie."
    : "Hors ligne sur Steam.";
}

/**
 * D'où vient cette ligne. Ce n'est pas de la décoration : les deux sources ne savent pas
 * la même chose, et ça change ce qu'on peut attendre de la ligne. Un ami Steam ne montre
 * que ses jeux Steam ; un ami Torii montre tous ses launchers, mais seulement quand il a
 * l'application ouverte. J'avais réduit ça à un ⛩ sans explication — illisible.
 */
/**
 * Pastilles de source, **une par canal**, qui disent à la fois d'où vient la relation ET
 * si la personne y est connectée en ce moment.
 *
 * 🔑 Une seule pastille « Torii + Steam » ne suffisait pas : quelqu'un d'ami des deux
 * côtés peut être en ligne sur Steam avec Torii fermé — auquel cas Torii ne voit rien de
 * ce qu'il joue — et l'inverse est tout aussi courant. La pastille éteinte le dit.
 */
function etatSource(canal: "steam" | "torii", etat: ToriiStatus | null) {
  const ou = canal === "steam" ? "sur Steam" : "sur Torii";
  switch (etat) {
    case "in-game":
      return { live: true, phrase: `En jeu, vu ${ou}` };
    case "online":
      return { live: true, phrase: `En ligne ${ou}` };
    case "away":
      return { live: true, phrase: `Absent ${ou}` };
    default:
      return {
        live: false,
        phrase: canal === "steam"
          ? "Hors ligne sur Steam"
          : "Torii fermé : ce qu'il joue hors Steam reste invisible",
      };
  }
}

/** Les canaux par lesquels on connaît cette personne, avec leur état courant. */
function sources(f: UnifiedFriend) {
  const liste: { key: "steam" | "torii"; label: string; live: boolean; title: string }[] = [];
  if (f.toriiState !== null) {
    const e = etatSource("torii", f.toriiState);
    liste.push({
      key: "torii",
      label: "Torii",
      live: e.live,
      title: `${e.phrase}. Ami Torii : tu vois ses jeux quel que soit son launcher, tant qu'il a Torii ouvert.`,
    });
  }
  if (f.steamState !== null) {
    const e = etatSource("steam", f.steamState);
    liste.push({
      key: "steam",
      label: "Steam",
      live: e.live,
      title: `${e.phrase}. Ami Steam : cette liste vient de Steam et se gère depuis Steam.`,
    });
  }
  return liste;
}

const failed = ref(new Set<string>());
function avatar(f: UnifiedFriend): string | null {
  return f.avatarUrl && !failed.value.has(f.avatarUrl) ? f.avatarUrl : null;
}
function onAvatarError(url: string) {
  failed.value = new Set(failed.value).add(url);
}
function initials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase();
}

function openProfile(f: UnifiedFriend) {
  if (f.profileUrl && f.profileUrl !== "#") openExternal(f.profileUrl);
}

/** Le même jeu, chez moi : je le lance s'il est installé, sinon j'ouvre sa fiche. */
function onSameGame(f: UnifiedFriend) {
  const g = f.ownedGame;
  if (!g) return;
  if (g.installed) launchOrInstall(g);
  else openGame(g.id);
}

/* ── Retirer un ami ────────────────────────────────────────────────────────── */

/**
 * Seuls les amis **Torii** se retirent d'ici. Une liste d'amis Steam appartient à Steam :
 * Torii la lit, il ne la modifie pas. Sans identifiant Torii, pas de bouton — plutôt
 * qu'un bouton qui échouerait.
 */
function canRemove(f: UnifiedFriend): boolean {
  return !!f.toriiId;
}

/**
 * Ce que le clic va vraiment faire. Deux conséquences que personne ne devine :
 * le lien est supprimé **des deux côtés** (tu disparais aussi de sa liste), et pour un
 * ami des deux bords, seul le lien Torii saute — Steam n'y est pour rien.
 */
function removeHint(f: UnifiedFriend): string {
  const base = `Retirer ${f.name} de tes amis Torii. Vous disparaîtrez de la liste l'un de l'autre.`;
  return f.source === "both"
    ? `${base} Vous resterez amis sur Steam, et tu continueras de le voir par là.`
    : base;
}

/**
 * Confirmation en place, dans la ligne elle-même. Retirer quelqu'un est irréversible
 * (il faudra une nouvelle demande, acceptée des deux côtés) : ça ne doit pas tenir à un
 * clic mal visé. Pas de fenêtre modale pour autant — elle ferait perdre de vue QUI on
 * s'apprête à retirer.
 */
const confirmKey = ref<string | null>(null);
const removing = ref<string | null>(null);

async function onRemove(f: UnifiedFriend) {
  if (!f.toriiId || removing.value) return;
  removing.value = f.key;
  try {
    await removeFriend(f.toriiId);
    showToast(
      f.source === "both"
        ? `${f.name} a été retiré de tes amis Torii. Vous restez amis sur Steam.`
        : `${f.name} a été retiré de tes amis, des deux côtés.`,
    );
  } catch (e) {
    showToast(`Retrait impossible : ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    removing.value = null;
    confirmKey.value = null;
  }
}

/** Les gens joignables tout de suite : en jeu ou disponibles. */
const disponibles = computed(() => online.value);
const showOffline = ref(false);

/* ── Ce qu'on laisse voir ──────────────────────────────────────────────────── */

const MODES = [
  {
    key: "detailed" as const,
    label: "Jeu visible",
    hint: "Tes amis voient à quoi tu joues et depuis quand.",
  },
  {
    key: "online" as const,
    label: "En ligne",
    hint: "Ils te savent connecté, sans savoir à quoi tu joues.",
  },
  {
    key: "offline" as const,
    label: "Invisible",
    hint: "Personne ne voit rien. Tu vois toujours tes amis.",
  },
];

const presenceOpen = ref(false);
const currentMode = computed(
  () => MODES.find((m) => m.key === presenceMode.value) ?? MODES[2],
);

async function choosePresence(mode: (typeof MODES)[number]["key"]) {
  presenceOpen.value = false;
  await setPresenceMode(mode);
}

/** Ferme le menu au clic ailleurs et à Échap, comme les autres menus de l'app. */
function onDocClick() {
  presenceOpen.value = false;
  confirmKey.value = null;
}
function onEsc(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  presenceOpen.value = false;
  confirmKey.value = null;
}
onMounted(() => {
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onEsc);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("keydown", onEsc);
});
</script>

<template>
  <div class="friends">
    <header class="head">
      <div class="head-title">
        <h2>Amis</h2>
        <span class="count">{{ activeCount }} en ligne</span>
        <span v-if="loading" class="spin" title="Actualisation…" />
      </div>

      <div class="head-actions">
        <!-- Ce que les autres voient de toi : lisible sans cliquer, changeable en deux
             clics, et chaque choix explique sa conséquence. -->
        <div v-if="toriiConnected" class="presence-wrap" @click.stop>
          <button
            class="visible-pill"
            :class="presenceMode"
            :aria-expanded="presenceOpen"
            @click="presenceOpen = !presenceOpen"
          >
            <span class="pill-dot" />
            {{ currentMode.label }}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
          </button>
          <div v-if="presenceOpen" class="presence-menu">
            <p class="menu-head">Ce que tes amis voient</p>
            <button
              v-for="m in MODES"
              :key="m.key"
              class="menu-item"
              :class="{ on: m.key === presenceMode }"
              @click="choosePresence(m.key)"
            >
              <span class="menu-dot" :class="m.key" />
              <span class="menu-text">
                <span class="menu-label">{{ m.label }}</span>
                <span class="menu-hint">{{ m.hint }}</span>
              </span>
              <svg v-if="m.key === presenceMode" class="menu-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><path d="M5 13l4 4L19 7" /></svg>
            </button>
          </div>
        </div>
        <button v-if="toriiConnected" class="btn-add" @click="addOpen = !addOpen">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
          Ajouter un ami
        </button>
        <button class="icon-btn" :disabled="loading" title="Actualiser" @click="refresh()">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
        </button>
      </div>
    </header>

    <!-- Ajout : le code d'ami n'apparaît qu'ici, au moment où il sert -->
    <div v-if="addOpen && toriiConnected" class="add-panel">
      <form class="add-form" @submit.prevent="onInvite">
        <label class="add-label" for="friend-code">Son code d'ami</label>
        <div class="add-row">
          <input
            id="friend-code"
            v-model="friendCode"
            placeholder="ABCD2345"
            maxlength="12"
            spellcheck="false"
            autocomplete="off"
          />
          <button type="submit" class="btn-primary" :disabled="addBusy || !friendCode.trim()">
            {{ addBusy ? "Envoi…" : "Envoyer" }}
          </button>
        </div>
        <p v-if="addError" class="add-error">{{ addError }}</p>
      </form>
      <div class="add-mine">
        <span class="add-label">Le tien, à lui donner</span>
        <button class="code-chip" title="Copier" @click="copyCode">
          {{ account?.friendCode }}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" /></svg>
        </button>
      </div>
    </div>

    <!-- Connexion Torii : proposée seulement tant qu'il n'y a pas de compte -->
    <ToriiPanel v-if="!toriiConnected" />
    <!-- Connecté mais bibliothèque jamais partagée : on le propose ici, une fois. -->
    <LibraryInvite v-else />

    <!-- Demandes reçues -->
    <section v-if="circle.incoming.length" class="requests">
      <h3>{{ circle.incoming.length }} demande{{ circle.incoming.length > 1 ? "s" : "" }} d'ami</h3>
      <div v-for="p in circle.incoming" :key="p.id" class="request">
        <span class="req-avatar">{{ initials(p.displayName) }}</span>
        <span class="req-name">{{ p.displayName }}</span>
        <button class="btn-primary sm" @click="respond(p.id, true)">Accepter</button>
        <button class="btn-ghost sm" @click="respond(p.id, false)">Refuser</button>
      </div>
    </section>

    <!-- Aucune source connectée -->
    <div v-if="loaded && !steamConnected && !toriiConnected" class="empty">
      <p class="empty-title">Personne à afficher pour l'instant</p>
      <p>Connecte ton compte Steam, ou crée un compte Torii pour voir tes amis quel que soit leur launcher.</p>
      <button class="btn-primary" @click="openSettings()">Ouvrir les réglages</button>
    </div>

    <div v-else-if="!loaded && loading" class="empty">
      <span class="spin big" />
      <p>Récupération de tes amis…</p>
    </div>

    <template v-else>
      <!-- ── En jeu : la section qui répond à « avec qui je joue ? » ── -->
      <section v-if="inGame.length" class="block">
        <h3 class="block-title ingame">En jeu <span>{{ inGame.length }}</span></h3>
        <div class="cards">
          <div v-for="f in inGame" :key="f.key" class="card" :class="{ same: f.ownedGame }">
            <button class="card-who" @click="openProfile(f)">
              <span class="avatar lg">
                <img v-if="avatar(f)" :src="avatar(f)!" alt="" loading="lazy" @error="onAvatarError(f.avatarUrl)" />
                <span v-else class="avatar-fallback">{{ initials(f.name) }}</span>
                <span class="dot in-game" />
              </span>
              <span class="who-text">
                <span class="who-name">{{ f.name }}</span>
                <!-- Toujours « en ce moment », quelle que soit la source. La durée
                     n'était connue que des amis Torii : deux cartes côte à côte ne
                     disaient pas la même chose, ce qui se lisait comme une information
                     manquante plutôt que comme une différence de source. -->
                <span class="who-when">
                  en ce moment
                  <span
                    v-for="s in sources(f)"
                    :key="s.key"
                    class="src"
                    :class="[s.key, { off: !s.live }]"
                    :title="s.title"
                  >{{ s.label }}</span>
                </span>
              </span>
            </button>
            <button
              v-if="canSeeLibrary(f) && confirmKey !== f.key"
              class="icon-lib card-lib"
              :title="libraryHint(f)"
              :aria-label="libraryHint(f)"
              @click.stop="showFriendLibrary(f.toriiId!)"
            ><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><rect x="3.5" y="4" width="6" height="16" rx="1.4" /><rect x="11.5" y="4" width="6" height="16" rx="1.4" /><path d="M19.5 6.6l1.9 15.2" /></svg></button>
            <button
              v-if="canRemove(f) && confirmKey !== f.key"
              class="icon-remove card-remove"
              :title="removeHint(f)"
              :aria-label="removeHint(f)"
              @click.stop="confirmKey = f.key"
            ><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg></button>
            <div class="card-game">
              <span class="game-name">{{ f.gameName ?? "Un jeu" }}</span>
              <span v-if="f.ownedGame" class="game-same">Tu l'as aussi</span>
            </div>
            <div v-if="confirmKey === f.key" class="confirm card-confirm" @click.stop>
              <span class="c-text">Le retirer ?</span>
              <button class="c-yes" :disabled="removing === f.key" @click="onRemove(f)">Retirer</button>
              <button class="c-no" @click="confirmKey = null">Annuler</button>
            </div>
            <button v-else-if="f.ownedGame" class="btn-play" @click="onSameGame(f)">
              <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
              {{ f.ownedGame.installed ? "Jouer" : "Voir la fiche" }}
            </button>
          </div>
        </div>
      </section>

      <!-- ── Disponibles ── -->
      <section v-if="disponibles.length" class="block">
        <h3 class="block-title">Disponibles <span>{{ disponibles.length }}</span></h3>
        <div class="rows">
          <div v-for="f in disponibles" :key="f.key" class="row-shell">
            <button class="row" @click="openProfile(f)">
              <span class="avatar">
                <img v-if="avatar(f)" :src="avatar(f)!" alt="" loading="lazy" @error="onAvatarError(f.avatarUrl)" />
                <span v-else class="avatar-fallback">{{ initials(f.name) }}</span>
                <span class="dot" :class="f.state" />
              </span>
              <span class="row-name">{{ f.name }}</span>
              <span
                v-for="s in sources(f)"
                :key="s.key"
                class="src"
                :class="[s.key, { off: !s.live }]"
                :title="s.title"
              >{{ s.label }}</span>
              <span class="row-state" :class="f.state">{{ stateLabel(f) }}</span>
            </button>
            <div v-if="confirmKey === f.key" class="confirm" @click.stop>
              <button class="c-yes" :disabled="removing === f.key" @click="onRemove(f)">Retirer</button>
              <button class="c-no" @click="confirmKey = null">Annuler</button>
            </div>
            <template v-else>
              <button
                v-if="canSeeLibrary(f)"
                class="icon-lib"
                :title="libraryHint(f)"
                :aria-label="libraryHint(f)"
                @click.stop="showFriendLibrary(f.toriiId!)"
              ><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><rect x="3.5" y="4" width="6" height="16" rx="1.4" /><rect x="11.5" y="4" width="6" height="16" rx="1.4" /><path d="M19.5 6.6l1.9 15.2" /></svg></button>
              <button
                v-if="canRemove(f)"
                class="icon-remove"
                :title="removeHint(f)"
                :aria-label="removeHint(f)"
                @click.stop="confirmKey = f.key"
              ><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg></button>
            </template>
          </div>
        </div>
      </section>

      <!-- ── Hors ligne : replié, c'est souvent la majorité ── -->
      <section v-if="offline.length" class="block">
        <button class="block-title toggle" @click="showOffline = !showOffline">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" :class="{ open: showOffline }"><path d="M9 6l6 6-6 6" /></svg>
          Hors ligne <span>{{ offline.length }}</span>
        </button>
        <div v-if="showOffline" class="rows">
          <div v-for="f in offline" :key="f.key" class="row-shell">
            <button class="row off" :title="offlineHint(f)" @click="openProfile(f)">
              <span class="avatar">
                <img v-if="avatar(f)" :src="avatar(f)!" alt="" loading="lazy" @error="onAvatarError(f.avatarUrl)" />
                <span v-else class="avatar-fallback">{{ initials(f.name) }}</span>
                <span class="dot offline" />
              </span>
              <span class="row-name">{{ f.name }}</span>
              <span
                v-for="s in sources(f)"
                :key="s.key"
                class="src"
                :class="[s.key, { off: !s.live }]"
                :title="s.title"
              >{{ s.label }}</span>
              <span class="row-state">{{ f.source === "torii" ? "Torii fermé" : "Hors ligne" }}</span>
            </button>
            <div v-if="confirmKey === f.key" class="confirm" @click.stop>
              <button class="c-yes" :disabled="removing === f.key" @click="onRemove(f)">Retirer</button>
              <button class="c-no" @click="confirmKey = null">Annuler</button>
            </div>
            <template v-else>
              <button
                v-if="canSeeLibrary(f)"
                class="icon-lib"
                :title="libraryHint(f)"
                :aria-label="libraryHint(f)"
                @click.stop="showFriendLibrary(f.toriiId!)"
              ><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><rect x="3.5" y="4" width="6" height="16" rx="1.4" /><rect x="11.5" y="4" width="6" height="16" rx="1.4" /><path d="M19.5 6.6l1.9 15.2" /></svg></button>
              <button
                v-if="canRemove(f)"
                class="icon-remove"
                :title="removeHint(f)"
                :aria-label="removeHint(f)"
                @click.stop="confirmKey = f.key"
              ><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg></button>
            </template>
          </div>
        </div>
      </section>

      <!-- ── Aucun ami : c'est ICI que le code d'ami est utile ── -->
      <div v-if="!inGame.length && !disponibles.length && !offline.length" class="empty">
        <p class="empty-title">Pas encore d'amis sur Torii</p>
        <template v-if="toriiConnected">
          <p>Donne ton code à quelqu'un, ou saisis le sien avec « Ajouter un ami ».</p>
          <button class="code-chip big" title="Copier" @click="copyCode">
            {{ account?.friendCode }}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" /></svg>
          </button>
        </template>
        <p v-else-if="steamConnected" class="dim">
          Ta liste d'amis Steam est peut-être privée.
        </p>
      </div>
    </template>
  </div>
</template>

<style scoped>
.friends { min-width: 0; }

/* ── En-tête ─────────────────────────────────────────── */
.head {
  display: flex; align-items: center; justify-content: space-between;
  gap: 16px; flex-wrap: wrap; margin-bottom: 20px;
}
.head-title { display: flex; align-items: center; gap: 12px; }
.head-title h2 { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; margin: 0; }
.count { font-family: var(--mono); font-size: 13px; color: var(--text-faint); }
.head-actions { display: flex; align-items: center; gap: 8px; }

.visible-pill {
  display: inline-flex; align-items: center; gap: 8px; cursor: pointer;
  padding: 7px 14px; border-radius: 99px; font-size: 12.5px; font-weight: 600;
  background: var(--surface); border: 1px solid var(--border); color: var(--text-dim);
}
.visible-pill .pill-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--text-faint); }
.visible-pill svg { width: 13px; height: 13px; opacity: 0.7; }
.visible-pill.detailed { color: #3ad07f; border-color: color-mix(in srgb, #3ad07f 45%, transparent); }
.visible-pill.detailed .pill-dot { background: #3ad07f; box-shadow: 0 0 8px #3ad07f; }
.visible-pill.online { color: var(--text); border-color: var(--border-strong); }
.visible-pill.online .pill-dot { background: var(--text-dim); }

.presence-wrap { position: relative; }
.presence-menu {
  position: absolute; top: calc(100% + 6px); right: 0; z-index: 60; width: 288px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px;
  box-shadow: var(--shadow-hero); padding: 6px;
}
.menu-head {
  font-size: 11px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase;
  color: var(--text-faint); margin: 6px 10px 8px;
}
.menu-item {
  display: flex; align-items: flex-start; gap: 10px; width: 100%; padding: 9px 10px;
  border-radius: 9px; background: none; border: none; cursor: pointer; text-align: left; color: inherit;
}
.menu-item:hover { background: var(--surface-2); }
.menu-item.on { background: var(--accent-soft); }
.menu-dot {
  width: 8px; height: 8px; border-radius: 50%; margin-top: 5px; flex: none;
  background: var(--text-faint);
}
.menu-dot.detailed { background: #3ad07f; }
.menu-dot.online { background: var(--text-dim); }
.menu-text { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.menu-label { font-size: 13.5px; font-weight: 600; }
.menu-hint { font-size: 11.5px; color: var(--text-faint); line-height: 1.4; }
.menu-check { width: 15px; height: 15px; color: var(--accent); flex: none; margin-top: 3px; }

.btn-add {
  display: inline-flex; align-items: center; gap: 7px; cursor: pointer;
  padding: 8px 15px; border-radius: 10px; font-size: 13px; font-weight: 600;
  background: var(--accent); color: var(--accent-ink); border: 1px solid transparent;
}
.btn-add:hover { background: var(--accent-hover); }
.btn-add svg { width: 15px; height: 15px; }

.icon-btn {
  display: grid; place-items: center; width: 34px; height: 34px; border-radius: 10px;
  background: var(--surface); border: 1px solid var(--border); color: var(--text-dim); cursor: pointer;
}
.icon-btn:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong); }
.icon-btn:disabled { opacity: 0.5; }
.icon-btn svg { width: 16px; height: 16px; }

/* ── Ajout d'un ami ──────────────────────────────────── */
.add-panel {
  display: flex; gap: 26px; flex-wrap: wrap; align-items: flex-start;
  padding: 16px 18px; margin-bottom: 20px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
}
.add-form { flex: 1; min-width: 260px; }
.add-label {
  display: block; font-size: 11.5px; font-weight: 700; letter-spacing: 0.06em;
  text-transform: uppercase; color: var(--text-faint); margin-bottom: 7px;
}
.add-row { display: flex; gap: 8px; }
.add-row input {
  flex: 1; min-width: 0; padding: 9px 13px; border-radius: 10px;
  background: var(--bg); border: 1px solid var(--border); color: var(--text);
  font-family: var(--mono); font-size: 14px; letter-spacing: 0.14em; text-transform: uppercase;
}
.add-row input:focus { outline: none; border-color: var(--accent); }
.add-error { margin: 8px 0 0; font-size: 12.5px; color: #ff6b6b; }
.add-mine { display: flex; flex-direction: column; }

.code-chip {
  display: inline-flex; align-items: center; gap: 9px; cursor: pointer;
  padding: 8px 14px; border-radius: 10px; font-family: var(--mono); font-size: 15px;
  letter-spacing: 0.16em; font-weight: 600;
  background: var(--surface-2); border: 1px dashed var(--border-strong); color: var(--text);
}
.code-chip:hover { border-color: var(--accent); color: var(--accent); }
.code-chip svg { width: 14px; height: 14px; opacity: 0.7; }
.code-chip.big { margin-top: 12px; font-size: 18px; padding: 11px 18px; }

/* ── Demandes ────────────────────────────────────────── */
.requests { margin-bottom: 24px; }
.requests h3 {
  font-size: 12px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin: 0 0 10px;
}
.request {
  display: flex; align-items: center; gap: 11px; padding: 10px 13px; margin-bottom: 7px;
  background: var(--surface); border: 1px solid var(--border);
  border-left: 3px solid var(--accent); border-radius: 11px;
}
.req-avatar {
  width: 30px; height: 30px; border-radius: 50%; display: grid; place-items: center;
  background: var(--surface-2); font-size: 11.5px; font-weight: 700; color: var(--text-dim);
}
.req-name { flex: 1; font-size: 14px; font-weight: 600; }

/* ── Blocs ───────────────────────────────────────────── */
.block { margin-bottom: 26px; }
.block-title {
  display: flex; align-items: center; gap: 8px;
  font-size: 12px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--text-faint); margin: 0 0 12px;
  background: none; border: none; padding: 0; cursor: default;
}
.block-title span { font-family: var(--mono); opacity: 0.75; }
.block-title.ingame { color: #3ad07f; }
.block-title.toggle { cursor: pointer; }
.block-title.toggle:hover { color: var(--text-dim); }
.block-title svg { width: 13px; height: 13px; transition: transform 0.15s; }
.block-title svg.open { transform: rotate(90deg); }

/* ── Cartes « en jeu » ───────────────────────────────── */
.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 12px; }
.card {
  position: relative;
  display: flex; flex-direction: column; gap: 12px; padding: 14px 16px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 14px;
}
/* Un jeu qu'on possède aussi : c'est l'information la plus actionnable de l'écran. */
.card.same { border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
.card-who {
  display: flex; align-items: center; gap: 11px; background: none; border: none;
  padding: 0; cursor: pointer; text-align: left; color: inherit; min-width: 0;
}
.who-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.who-name {
  font-size: 14.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.who-when { font-size: 12px; color: var(--text-faint); }
.card-game { display: flex; align-items: baseline; gap: 9px; flex-wrap: wrap; }
.game-name {
  font-size: 17px; font-weight: 700; letter-spacing: -0.015em; line-height: 1.2;
  overflow: hidden; text-overflow: ellipsis;
}
.game-same {
  font-size: 11px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase;
  color: var(--accent); background: var(--accent-soft); padding: 2px 8px; border-radius: 6px;
}
/* 🔑 `margin-top: auto` colle le bouton au bas de la carte. Les cartes d'une même
   ligne ont déjà la hauteur de la plus haute (grille étirée) ; sans ça, un titre de jeu
   sur deux lignes décalait le bouton d'une carte à l'autre. */
.btn-play {
  margin-top: auto;
  display: inline-flex; align-items: center; justify-content: center; gap: 7px; cursor: pointer;
  padding: 9px 14px; border-radius: 10px; font-size: 13px; font-weight: 600;
  background: var(--accent); color: var(--accent-ink); border: 1px solid transparent;
}
.btn-play:hover { background: var(--accent-hover); }
.btn-play svg { width: 14px; height: 14px; }

/* ── Retirer un ami ──────────────────────────────────── */

/* La ligne reste un seul bouton cliquable ; la corbeille vit à côté, pas dedans —
   un bouton dans un bouton n'est pas du HTML valide et ne se clique pas de façon
   fiable. D'où cette coquille. */
.row-shell { display: flex; align-items: center; gap: 4px; }
.row-shell > .row { flex: 1; min-width: 0; }

/* Discrète au repos : on ne met pas une corbeille sous les yeux de quelqu'un qui
   consulte sa liste d'amis. Elle apparaît au survol, et au clavier par le focus. */
.icon-remove {
  flex: none; display: grid; place-items: center; width: 30px; height: 30px;
  border-radius: 9px; cursor: pointer; color: var(--text-faint);
  background: none; border: 1px solid transparent; opacity: 0; transition: opacity 0.12s;
}
.row-shell:hover .icon-remove, .card:hover .icon-remove, .icon-remove:focus-visible { opacity: 1; }
.icon-remove:hover { color: #ff6b6b; background: color-mix(in srgb, #ff6b6b 13%, transparent); }
.icon-remove svg { width: 15px; height: 15px; }
.card-remove { position: absolute; top: 10px; right: 10px; z-index: 1; }

/* Accès à la bibliothèque d'un ami : même gabarit que la corbeille (apparaît au survol),
   mais en couleur d'accent — c'est une action qu'on propose, pas une qu'on redoute. */
.icon-lib {
  flex: none; display: grid; place-items: center; width: 30px; height: 30px;
  border-radius: 9px; cursor: pointer; color: var(--text-faint);
  background: none; border: 1px solid transparent; opacity: 0; transition: opacity 0.12s;
}
.row-shell:hover .icon-lib, .card:hover .icon-lib, .icon-lib:focus-visible { opacity: 1; }
.icon-lib:hover { color: var(--accent); background: color-mix(in srgb, var(--accent) 13%, transparent); }
.icon-lib svg { width: 15px; height: 15px; }
/* Sur une carte « en jeu », elle se range à gauche de la corbeille. */
.card-lib { position: absolute; top: 10px; right: 44px; z-index: 1; }

.confirm { display: flex; align-items: center; gap: 6px; flex: none; }
.card-confirm { margin-top: auto; }
.c-text { font-size: 12.5px; color: var(--text-dim); margin-right: auto; }
.c-yes, .c-no {
  padding: 7px 12px; border-radius: 9px; font-size: 12.5px; font-weight: 600;
  cursor: pointer; font-family: inherit;
}
.c-yes {
  color: #ff6b6b; background: color-mix(in srgb, #ff6b6b 14%, transparent);
  border: 1px solid color-mix(in srgb, #ff6b6b 40%, transparent);
}
.c-yes:hover:not(:disabled) { background: color-mix(in srgb, #ff6b6b 22%, transparent); }
.c-yes:disabled { opacity: 0.5; cursor: default; }
.c-no { color: var(--text-dim); background: none; border: 1px solid var(--border); }
.c-no:hover { color: var(--text); border-color: var(--border-strong); }

/* ── Lignes ──────────────────────────────────────────── */
.rows { display: flex; flex-direction: column; gap: 3px; }
.row {
  display: flex; align-items: center; gap: 12px; width: 100%;
  padding: 9px 12px; border-radius: 11px; cursor: pointer; text-align: left;
  background: none; border: 1px solid transparent; color: inherit;
}
.row:hover { background: var(--surface); border-color: var(--border); }
.row.off { opacity: 0.62; }
.row-name { flex: 1; font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.row-state { font-size: 12.5px; color: var(--text-faint); }
.row-state.online { color: #3ad07f; }

/* Provenance : discrète mais lisible, et survolable pour comprendre la nuance. */
.src {
  flex: none; font-size: 10px; font-weight: 700; letter-spacing: 0.05em;
  padding: 2px 7px; border-radius: 5px; cursor: help; white-space: nowrap;
  background: var(--surface-2); color: var(--text-faint); border: 1px solid transparent;
}
.src.torii { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 32%, transparent); }
.src.steam { color: var(--steam); border-color: color-mix(in srgb, var(--steam) 32%, transparent); }
/* Canal connu mais personne pas connectée dessus : la pastille reste lisible (la
   relation existe toujours) tout en cessant d'affirmer une présence. */
.src.off { color: var(--text-faint); border-color: transparent; opacity: 0.65; }
.who-when { display: flex; align-items: center; gap: 7px; }

/* ── Avatars ─────────────────────────────────────────── */
.avatar {
  position: relative; width: 34px; height: 34px; flex: none; border-radius: 50%;
  overflow: visible; display: grid; place-items: center; background: var(--surface-2);
}
.avatar.lg { width: 42px; height: 42px; }
.avatar img { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; }
.avatar-fallback { font-size: 12px; font-weight: 700; color: var(--text-dim); }
.avatar.lg .avatar-fallback { font-size: 14px; }
.dot {
  position: absolute; right: -1px; bottom: -1px; width: 11px; height: 11px; border-radius: 50%;
  background: var(--text-faint); border: 2px solid var(--bg);
}
.dot.in-game { background: #3ad07f; }
.dot.online { background: #3ad07f; }
.dot.away { background: #e7b667; }
.dot.offline { background: var(--text-faint); }

/* ── Divers ──────────────────────────────────────────── */
.btn-primary {
  padding: 9px 16px; border-radius: 10px; border: 1px solid transparent; cursor: pointer;
  background: var(--accent); color: var(--accent-ink); font-weight: 600; font-size: 13.5px;
}
.btn-primary:hover:not(:disabled) { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.btn-primary.sm, .btn-ghost.sm { padding: 6px 12px; font-size: 12.5px; }
.btn-ghost {
  padding: 8px 14px; border-radius: 10px; cursor: pointer;
  background: none; border: 1px solid var(--border); color: var(--text-dim); font-size: 13px;
}
.btn-ghost:hover { color: var(--text); border-color: var(--border-strong); }

.spin {
  width: 14px; height: 14px; border-radius: 50%; display: inline-block;
  border: 2px solid var(--border-strong); border-top-color: var(--accent);
  animation: spin 0.7s linear infinite;
}
.spin.big { width: 26px; height: 26px; border-width: 3px; margin-bottom: 14px; }
@keyframes spin { to { transform: rotate(360deg); } }

.empty {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  padding: 60px 20px; text-align: center; color: var(--text-faint); font-size: 14px;
}
.empty-title { font-size: 16px; font-weight: 700; color: var(--text); margin: 0 0 6px; }
.empty p { margin: 3px 0; max-width: 46ch; line-height: 1.5; }
.empty .btn-primary { margin-top: 16px; }
.empty .dim { font-size: 12.5px; opacity: 0.8; }
</style>
