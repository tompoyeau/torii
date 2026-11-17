<script setup lang="ts">
import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref } from "vue";
import { useFriends } from "../composables/useFriends";
import { useFriendList, type UnifiedFriend } from "../composables/useFriendList";
import { offlineHintOf, sourcesOf } from "../lib/friendPresence";
import { useLibrary } from "../composables/useLibrary";
import { useTorii } from "../composables/useTorii";
import { useUi } from "../composables/useUi";
import { useFriendLibrary } from "../composables/useFriendLibrary";
import { useStore } from "../composables/useStore";
import { showToast } from "../composables/useToast";
import ToriiPanel from "./ToriiPanel.vue";
import LibraryInvite from "./LibraryInvite.vue";
import AjoutAmiModal from "./AjoutAmiModal.vue";
import { t } from "../i18n";

const { loaded, steamConnected, refresh } = useFriends();
const { inGame, online, offline, activeCount, loading } = useFriendList();
const {
  account, circle, connected: toriiConnected, presenceMode,
  refresh: refreshTorii, respond, setPresenceMode,
} = useTorii();
const { launchOrInstall } = useLibrary();
const { openSettings, openGame, showFriendProfile, showStore } = useUi();
const { openForTitle } = useStore();
const { refreshIndex } = useFriendLibrary();

/**
 * La présence Torii arrive seule (battement de cœur) ; seul Steam doit être sondé.
 *
 * L'écran reste en mémoire entre deux onglets (`<KeepAlive>`, AppShell) : on rafraîchit en
 * arrivant (`onActivated`, appelé aussi au premier affichage) sans effacer la liste déjà là,
 * et le sondage de Steam s'arrête quand on part (`onDeactivated`).
 */
let timer: ReturnType<typeof setInterval> | undefined;
onActivated(() => {
  void refresh();
  void refreshTorii();
  // Un ami a pu activer le partage depuis le démarrage : on relit l'index en arrivant
  // ici, plutôt que de le sonder en boucle pour une donnée qui bouge une fois par mois.
  void refreshIndex();
  clearInterval(timer);
  timer = setInterval(() => void refresh(), 60_000);
});
onDeactivated(() => clearInterval(timer));
onBeforeUnmount(() => clearInterval(timer));

/* ── Ajout d'un ami : la fenêtre `AjoutAmiModal` ───────────────────────────── */

const addOpen = ref(false);

async function copyCode() {
  if (!account.value) return;
  try {
    await navigator.clipboard.writeText(account.value.friendCode);
    showToast(t("amis.codeCopie"));
  } catch {
    showToast(t("amis.copieImpossible"));
  }
}

/* ── Affichage d'un ami ────────────────────────────────────────────────────── */

function stateLabel(f: UnifiedFriend): string {
  switch (f.state) {
    case "online": return t("amis.etats.enLigne");
    case "away": return t("amis.etats.absent");
    case "in-game": return f.gameName ?? t("amis.etats.enJeu");
    default: return t("amis.etats.horsLigne");
  }
}

/**
 * Pourquoi cette personne est-elle « hors ligne » ? Pour un ami Torii, ça veut dire
 * qu'il n'a pas l'application ouverte — pas qu'il ne joue pas. On ne l'affiche qu'en
 * infobulle : c'est une nuance utile, pas une information de premier plan.
 */
const offlineHint = offlineHintOf;

/**
 * Pastilles de source, **une par canal** (`lib/friendPresence.ts`), qui disent à la fois
 * d'où vient la relation ET si la personne y est connectée en ce moment. Partagées avec
 * la page profil : deux écrans qui décriraient différemment la même personne se liraient
 * comme une contradiction.
 */
const sources = sourcesOf;

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

/**
 * Cliquer sur quelqu'un mène à **sa page dans Torii**, qu'il ait un compte ou non.
 *
 * C'est là que vivent sa bibliothèque, sa présence par canal et le retrait d'ami — tout
 * ce qu'une page Steam ne dira jamais. Et pour un ami Steam sans compte Torii, la page est
 * presque vide **exprès** : elle dit ce que Torii ne peut pas savoir de lui, et pourquoi.
 * Le renvoyer vers Steam sans explication laissait croire à une panne.
 */
function openProfile(f: UnifiedFriend) {
  showFriendProfile(f.key);
}

/** Le même jeu, chez moi : je le lance s'il est installé, sinon j'ouvre sa fiche. */
function onSameGame(f: UnifiedFriend) {
  const g = f.ownedGame;
  if (!g) return;
  if (g.installed) launchOrInstall(g);
  else openGame(g.id);
}

/**
 * Un jeu qu'on n'a pas : sa fiche Boutique. Seul le titre est connu (ni Steam ni Torii ne
 * donnent ici d'identifiant), donc `openForTitle` n'ouvre une fiche que sur un titre exact ;
 * sinon il prépare la recherche, et c'est la Boutique qu'on affiche pour ses résultats.
 */
async function voirEnBoutique(f: UnifiedFriend) {
  if (!f.gameName) return;
  if (!(await openForTitle(f.gameName))) showStore();
}

/** Les gens joignables tout de suite : en jeu ou disponibles. */
const disponibles = computed(() => online.value);
const showOffline = ref(false);

/* ── Ce qu'on laisse voir ──────────────────────────────────────────────────── */

/** Mêmes libellés que les Paramètres, lus au même endroit du catalogue. */
const MODES = computed(() => [
  { key: "detailed" as const, label: t("reglages.torii.presence.detaille"), hint: t("reglages.torii.presence.detailleAide") },
  { key: "online" as const, label: t("reglages.torii.presence.enLigne"), hint: t("reglages.torii.presence.enLigneAide") },
  { key: "offline" as const, label: t("reglages.torii.presence.invisible"), hint: t("reglages.torii.presence.invisibleAide") },
]);

const presenceOpen = ref(false);
const currentMode = computed(
  () => MODES.value.find((m) => m.key === presenceMode.value) ?? MODES.value[2],
);

async function choosePresence(mode: (typeof MODES.value)[number]["key"]) {
  presenceOpen.value = false;
  await setPresenceMode(mode);
}

/** Ferme le menu au clic ailleurs et à Échap, comme les autres menus de l'app. */
function onDocClick() {
  presenceOpen.value = false;
}
function onEsc(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  presenceOpen.value = false;
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
        <h1>{{ t("bibliotheque.laterale.contacts") }}</h1>
        <span class="count">{{ t("amis.enLigneCompte", { n: activeCount }) }}</span>
        <span v-if="loading" class="spin" :title="t('amis.actualisation')" />
      </div>

      <div class="head-actions">
        <!-- Ce que les autres voient de toi : lisible sans cliquer, changeable en deux
             clics, et chaque choix explique sa conséquence. -->
        <div v-if="toriiConnected" class="presence-wrap" @click.stop>
          <button
            class="visible-pill"
            :class="presenceMode"
            aria-haspopup="true"
            :aria-expanded="presenceOpen"
            @click="presenceOpen = !presenceOpen"
          >
            <span class="pill-dot" />
            {{ currentMode.label }}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 9l6 6 6-6" /></svg>
          </button>
          <div v-if="presenceOpen" class="presence-menu">
            <p class="menu-head">{{ t("amis.ceQueVoient") }}</p>
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
        <button v-if="toriiConnected" class="btn-add" aria-haspopup="dialog" @click="addOpen = true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M12 5v14M5 12h14" /></svg>
          {{ t("amis.ajouterAmi") }}
        </button>
        <button class="icon-btn" :disabled="loading" :title="t('amis.actualiser')" @click="refresh()">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-2.64-6.36M21 4v5h-5" /></svg>
        </button>
      </div>
    </header>

    <!-- Ajout d'un ami : dans une fenêtre, la page ne bouge pas. -->
    <AjoutAmiModal :ouvert="addOpen && toriiConnected" @fermer="addOpen = false" />

    <!-- Connexion Torii : proposée seulement tant qu'il n'y a pas de compte -->
    <ToriiPanel v-if="!toriiConnected" />
    <!-- Connecté mais bibliothèque jamais partagée : on le propose ici, une fois. -->
    <LibraryInvite v-else />

    <!-- Demandes reçues -->
    <section v-if="circle.incoming.length" class="requests">
      <h3>{{ t("amis.demandes", { n: circle.incoming.length }) }}</h3>
      <div v-for="p in circle.incoming" :key="p.id" class="request">
        <span class="req-avatar">{{ initials(p.displayName) }}</span>
        <span class="req-name">{{ p.displayName }}</span>
        <button class="btn-primary sm" @click="respond(p.id, true)">{{ t("amis.accepter") }}</button>
        <button class="btn-ghost sm" @click="respond(p.id, false)">{{ t("amis.refuser") }}</button>
      </div>
    </section>

    <!-- Aucune source connectée -->
    <div v-if="loaded && !steamConnected && !toriiConnected" class="empty">
      <p class="empty-title">{{ t("amis.personne") }}</p>
      <p>{{ t("amis.personneAide") }}</p>
      <button class="btn-primary" @click="openSettings()">{{ t("amis.ouvrirReglages") }}</button>
    </div>

    <div v-else-if="!loaded && loading" class="empty">
      <span class="spin big" />
      <p>{{ t("amis.recuperation") }}</p>
    </div>

    <!-- Deux colonnes : ce qui se passe (qui joue, à quoi jouer ensemble) au centre, les
         gens à droite. Les jeux en commun arrivent par le slot (`CommonView`, `AppShell`). -->
    <div v-else class="disposition">
     <div class="principal">
      <!-- ── En jeu : la section qui répond à « avec qui je joue ? » ── -->
      <section class="block">
        <h3 class="block-title ingame">{{ t("amis.enJeu") }} <span v-if="inGame.length">{{ inGame.length }}</span></h3>
        <p v-if="!inGame.length" class="calme">{{ t("amis.personneEnJeu") }}</p>
        <div v-else class="cards">
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
                  {{ t("amis.enCeMoment") }}
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
            <div class="card-game">
              <span class="game-name">{{ f.gameName ?? t("amis.unJeu") }}</span>
              <span v-if="f.ownedGame" class="game-same">{{ t("amis.tuLAsAussi") }}</span>
            </div>
            <button v-if="f.ownedGame" class="btn-play" @click="onSameGame(f)">
              <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
              {{ f.ownedGame.installed ? t("bibliotheque.jouer") : t("amis.voirFiche") }}
            </button>
            <!-- Un jeu qu'on n'a pas : sa fiche Boutique, pour voir ce qu'il coûte et le rejoindre. -->
            <button v-else-if="f.gameName" class="btn-play boutique" @click="voirEnBoutique(f)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20.5 13.3 12.8 21a1.5 1.5 0 0 1-2.1 0l-7-7a1.4 1.4 0 0 1-.4-1V4.6A1.5 1.5 0 0 1 4.6 3h8.4a1.4 1.4 0 0 1 1 .4l6.5 6.5a2 2 0 0 1 0 2.8Z" /><circle cx="7.8" cy="7.8" r="1.4" fill="currentColor" stroke="none" /></svg>
              {{ t("fiche.voirBoutique") }}
            </button>
          </div>
        </div>
      </section>

      <slot />
     </div>

     <aside class="cote">
      <!-- ── Disponibles ── -->
      <section v-if="disponibles.length" class="block">
        <h3 class="block-title">{{ t("amis.disponibles") }} <span>{{ disponibles.length }}</span></h3>
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
          </div>
        </div>
      </section>

      <!-- ── Hors ligne : replié, c'est souvent la majorité ── -->
      <section v-if="offline.length" class="block">
        <button class="block-title toggle" @click="showOffline = !showOffline">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" :class="{ open: showOffline }"><path d="M9 6l6 6-6 6" /></svg>
          {{ t("amis.horsLigne") }} <span>{{ offline.length }}</span>
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
              <span class="row-state">{{ f.source === "torii" ? t("amis.toriiFerme") : t("amis.horsLigne") }}</span>
            </button>
          </div>
        </div>
      </section>

      <!-- ── Aucun ami ── -->
      <div v-if="!inGame.length && !disponibles.length && !offline.length" class="vide-cote">
        <p class="empty-title">{{ t("amis.pasEncore") }}</p>
        <p v-if="toriiConnected">{{ t("amis.pasEncoreAide") }}</p>
        <p v-else-if="steamConnected" class="dim">{{ t("amis.listePrivee") }}</p>
      </div>

      <!-- ── Ton code d'ami : toujours sous la main, c'est lui qui fait grandir la liste ── -->
      <div v-if="toriiConnected && account" class="carte-code">
        <span class="add-label">{{ t("amis.tonCode") }}</span>
        <button class="code-chip" :title="t('amis.copier')" @click="copyCode">
          {{ account.friendCode }}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" /></svg>
        </button>
        <p>{{ t("amis.tonCodeAide") }}</p>
      </div>
     </aside>
    </div>
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
.head-title h1 { font-size: 24px; font-weight: 800; letter-spacing: -0.02em; margin: 0; }

/* ── Disposition : l'activité au centre, les gens à droite ── */
.disposition { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 34px; align-items: start; }
.principal { min-width: 0; }
.cote { min-width: 0; display: flex; flex-direction: column; }
.cote .row { padding: 8px 10px; gap: 10px; }
.calme { margin: 0; font-size: 13.5px; color: var(--text-faint); }
.vide-cote { padding: 4px 2px 20px; font-size: 13px; color: var(--text-faint); line-height: 1.5; }
.vide-cote p { margin: 0 0 4px; }
.vide-cote .empty-title { font-size: 14.5px; }
.carte-code {
  display: flex; flex-direction: column; align-items: flex-start; gap: 4px;
  padding: 14px 16px; border-radius: 14px; background: var(--surface); border: 1px solid var(--border);
}
.carte-code .add-label { margin-bottom: 4px; }
.carte-code p { margin: 8px 0 0; font-size: 12.5px; line-height: 1.45; color: var(--text-faint); }
/* Fenêtre étroite (940 px au minimum) : colonne des amis plus mince, mais toujours à côté —
   en une seule colonne, la liste passait sous tous les jeux en commun, loin de la vue. */
@media (max-width: 1180px) {
  .disposition { grid-template-columns: minmax(0, 1fr) 270px; gap: 24px; }
}
/* Téléphone (version web) : une seule colonne. */
@media (max-width: 820px) {
  .disposition { grid-template-columns: minmax(0, 1fr); }
}
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

/* Étiquette de la carte « Ton code d'ami ». */
.add-label {
  display: block; font-size: 11.5px; font-weight: 700; letter-spacing: 0.06em;
  text-transform: uppercase; color: var(--text-faint); margin-bottom: 7px;
}

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
/* Même place et même forme que « Jouer », en retrait : jouer avec quelqu'un reste l'action
   phare de l'écran, la Boutique une piste. */
.btn-play.boutique { background: var(--surface-2); color: var(--text); border-color: var(--border); }
.btn-play.boutique:hover { background: var(--surface-3); border-color: var(--border-strong); }

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
