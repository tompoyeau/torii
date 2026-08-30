<script setup lang="ts">
/**
 * La page d'un ami — **de tous les amis**, qu'il soit dans ton cercle Torii ou non.
 *
 * 🔑 Elle existe parce que cliquer sur quelqu'un ouvrait sa page Steam dans le navigateur :
 * on quittait Torii pour une page qui ne connaît que Steam, ne dit rien de ses jeux GOG ou
 * Epic, et n'a évidemment aucun bouton « voir sa bibliothèque ». Tout ce qu'on sait d'une
 * personne — sa présence par canal, ce qu'elle joue, ce qu'elle possède, le lien d'amitié
 * lui-même — vit ici.
 *
 * 🔑 La page d'un ami **qui n'est pas dans le cercle Torii** est presque vide, et c'est le
 * propos : elle dit ce que Torii ne peut pas savoir de lui **et pourquoi**. Une page pauvre
 * qui s'explique vaut mieux qu'un renvoi vers Steam, qui se lisait comme une panne.
 *
 * ⚠️ Elle ne dit PAS qu'il n'a pas de compte Torii — on n'en sait rien. `toriiId` absent
 * signifie seulement qu'il n'est pas dans TES amis Torii ; il peut très bien utiliser
 * Torii sans que vous y soyez liés. Le texte s'en tient donc à ce qui est vérifiable.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useFriendLibrary } from "../composables/useFriendLibrary";
import { useFriendList } from "../composables/useFriendList";
import { useLibrary } from "../composables/useLibrary";
import { useScrollLock } from "../composables/useScrollLock";
import { useTorii } from "../composables/useTorii";
import { useUi } from "../composables/useUi";
import { showToast } from "../composables/useToast";
import { sourcesOf } from "../lib/friendPresence";
import { gradientFor } from "../lib/covers";
import { openExternal, openWebWindow } from "../lib/tauri";
import type { LibGame } from "../types";

const { friendProfileKey, showFriends, showFriendLibrary, openGame } = useUi();
const { inGame, online, offline } = useFriendList();
const { games, loading, loadedId, devicesOf, hasLibrary, load, mine, commonCount, refreshIndex } =
  useFriendLibrary();
const { account: monCompte, removeFriend } = useTorii();
const { launchOrInstall } = useLibrary();

/** La personne, retrouvée par sa clé unifiée (`torii:<id>` ou `steam:<id>`). */
const friend = computed(
  () =>
    [...inGame.value, ...online.value, ...offline.value].find(
      (f) => f.key === friendProfileKey.value,
    ) ?? null,
);

/**
 * Son identifiant Torii. C'est lui qui conditionne tout le reste de la page.
 *
 * 🔑 Il se lit d'abord dans la **clé**, pas seulement dans le cercle : le cercle arrive par
 * battement de cœur et peut être vide à l'ouverture (démarrage à froid, réseau lent, retour
 * souris). S'en remettre à lui seul faisait annoncer « pas dans tes amis Torii » à quelqu'un
 * dont on s'apprêtait justement à afficher la bibliothèque.
 */
const toriiId = computed(() => {
  const cle = friendProfileKey.value ?? "";
  if (cle.startsWith("torii:")) return cle.slice("torii:".length);
  return friend.value?.toriiId ?? null;
});

/** Même repli que dans la vue bibliothèque : l'index porte le pseudo, le cercle peut ne
    pas (encore) l'avoir. */
const name = computed(
  () => friend.value?.name ?? devicesOf(toriiId.value ?? "")[0]?.displayName ?? "Cet ami",
);

const partage = computed(() => hasLibrary(toriiId.value));
/** Nombre de jeux annoncé par l'index — connu sans télécharger la bibliothèque. */
const gameCount = computed(() =>
  devicesOf(toriiId.value ?? "").reduce((n, d) => n + d.gameCount, 0),
);

const canaux = computed(() => (friend.value ? sourcesOf(friend.value) : []));

/** Ce que fait la personne, en une ligne. */
const stateLine = computed(() => {
  const f = friend.value;
  if (!f) return "Hors de ton cercle";
  switch (f.state) {
    case "in-game":
      return f.gameName ? `Joue à ${f.gameName}` : "En jeu";
    case "online":
      return "En ligne";
    case "away":
      return "Absent";
    default:
      return "Hors ligne";
  }
});

// Charge sa bibliothèque en arrivant : elle alimente l'aperçu ET le « en commun ».
// Le composable garde un cache par compte, donc revenir sur la page ne retélécharge rien.
//
// 🔑 On relit AUSSI l'index. Tout ce qui suit en dépend — partage-t-il, combien de jeux —
// et il n'est chargé qu'au démarrage et à l'ouverture de la vue Amis. Sans ça, une page
// ouverte autrement (retour souris, ami qui vient d'activer le partage) annoncerait
// « ne partage pas sa bibliothèque » à tort. C'est exactement le piège qui avait rendu le
// bouton « voir sa bibliothèque » invisible en pratique.
watch(
  toriiId,
  (v) => {
    if (!v) return;
    void refreshIndex();
    if (v !== loadedId.value) void load(v);
  },
  { immediate: true },
);

/**
 * Les premières jaquettes seulement : la page profil donne un aperçu, pas un inventaire.
 * Le plafond est là pour ne pas construire des centaines de tuiles dont une seule rangée
 * sera visible — la coupe à une ligne, elle, se fait en CSS (cf. `.strip`).
 */
const apercu = computed<LibGame[]>(() => games.value.slice(0, 16));

function coverOf(g: LibGame): string | undefined {
  return g.cover ?? undefined;
}

/* ── Actions ───────────────────────────────────────────────────────────────── */

/** Le même jeu, chez moi : je le lance s'il est installé, sinon j'ouvre sa fiche. */
function onSameGame() {
  const g = friend.value?.ownedGame;
  if (!g) return;
  if (g.installed) launchOrInstall(g);
  else openGame(g.id);
}

/**
 * Le profil Steam reste accessible quand la personne est amie de ce côté.
 *
 * ⚠️ Il s'ouvre dans une fenêtre Torii et **pas** intégré à la page : Steam sert
 * `X-Frame-Options: SAMEORIGIN` et `frame-ancestors 'self' https://steamloopback.host`
 * sur toutes ses pages. Aucune page Steam ne peut être encadrée par Torii — c'est un refus
 * de Steam, pas une limite de Tauri. Repli sur le navigateur si le natif refuse.
 */
async function openSteam() {
  const url = friend.value?.profileUrl;
  if (!url || url === "#") return;
  if (!(await openWebWindow(url, `${name.value} — profil Steam`))) openExternal(url);
}

/**
 * Le code d'ami, proposé **là où la phrase le conseille**. Sans lui, « donne-lui ton code
 * d'ami » est un conseil qu'il faut aller appliquer ailleurs : la page dit quoi faire, elle
 * doit permettre de le faire. Rien à proposer si on n'a pas soi-même de compte Torii.
 */
const monCode = computed(() => monCompte.value?.friendCode ?? null);

async function copierCode() {
  const code = monCode.value;
  if (!code) return;
  try {
    await navigator.clipboard.writeText(code);
    showToast("Ton code d'ami est copié.");
  } catch {
    showToast("Copie impossible ; note le code à la main.");
  }
}

/* ── Réglages de la relation (l'écrou) ─────────────────────────────────────── */

/**
 * Retirer quelqu'un n'est pas une action de tous les jours : elle vit derrière un écrou,
 * pas à côté de « Voir sa bibliothèque ». Deux conséquences que personne ne devine — le
 * lien saute **des deux côtés**, et pour un ami des deux bords seul le lien Torii saute.
 */
const menuOpen = ref(false);
const confirming = ref(false);
const removing = ref(false);

/** Un écrou sans rien dedans serait une promesse vide : seul un lien Torii se retire. */
const hasSettings = computed(() => !!toriiId.value);

const removeHint = computed(() =>
  friend.value?.source === "both"
    ? `Vous disparaîtrez de la liste Torii l'un de l'autre. Vous resterez amis sur Steam, et tu continueras de le voir par là.`
    : `Vous disparaîtrez de la liste l'un de l'autre.`,
);

// La confirmation est une vraie fenêtre modale : le geste est indéfaisable (il faudra une
// nouvelle demande, acceptée des deux côtés) et il ne doit pas tenir à un clic mal visé.
useScrollLock(confirming);

async function onRemove() {
  const id = toriiId.value;
  if (!id || removing.value) return;
  removing.value = true;
  try {
    await removeFriend(id);
    showToast(
      friend.value?.source === "both"
        ? `${name.value} a été retiré de tes amis Torii. Vous restez amis sur Steam.`
        : `${name.value} a été retiré de tes amis, des deux côtés.`,
    );
    // Sa page n'a plus d'objet : on revient à la liste plutôt que de laisser un profil
    // d'inconnu à l'écran.
    showFriends();
  } catch (e) {
    showToast(`Retrait impossible : ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    removing.value = false;
    confirming.value = false;
  }
}

/** Échap et clic ailleurs ferment le menu, puis la modale — dans cet ordre. */
function onDocClick() {
  menuOpen.value = false;
}
function onEsc(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (confirming.value) confirming.value = false;
  else menuOpen.value = false;
}
onMounted(() => {
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onEsc);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("keydown", onEsc);
});

/* ── Avatar ────────────────────────────────────────────────────────────────── */

const avatarFailed = ref(false);
const avatar = computed(() =>
  friend.value?.avatarUrl && !avatarFailed.value ? friend.value.avatarUrl : null,
);
const initials = computed(() => name.value.trim().slice(0, 2).toUpperCase());
</script>

<template>
  <div class="profile">
    <div class="sec-head">
      <button class="chip back" title="Retour aux amis" @click="showFriends()">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M15 6l-6 6 6 6" />
        </svg>
        Amis
      </button>
    </div>

    <header class="hero">
      <span class="avatar">
        <img v-if="avatar" :src="avatar" alt="" @error="avatarFailed = true" />
        <span v-else class="avatar-fallback">{{ initials }}</span>
      </span>
      <div class="ident">
        <h1>{{ name }}</h1>
        <p class="state" :class="friend?.state ?? 'offline'">
          <span class="dot" />
          {{ stateLine }}
          <span
            v-for="s in canaux"
            :key="s.key"
            class="src"
            :class="[s.key, { off: !s.live }]"
            :title="s.title"
          >{{ s.label }}</span>
        </p>
      </div>
      <div class="actions">
        <button v-if="partage" class="btn-primary" @click="showFriendLibrary(toriiId!)">
          Voir sa bibliothèque
        </button>
        <button v-if="friend?.ownedGame" class="btn-soft" @click="onSameGame">
          {{ friend.ownedGame.installed ? "Jouer au même jeu" : "Voir la fiche" }}
        </button>
        <button v-if="friend?.profileUrl" class="btn-soft" @click="openSteam">
          Profil Steam
        </button>

        <!-- L'écrou : les réglages de la relation, pas ceux de l'application. -->
        <div v-if="hasSettings" class="gear-wrap" @click.stop>
          <button
            class="gear"
            title="Réglages de cette relation"
            aria-label="Réglages de cette relation"
            :aria-expanded="menuOpen"
            @click="menuOpen = !menuOpen"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
              <circle cx="12" cy="12" r="3.2" />
              <path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" />
            </svg>
          </button>
          <div v-if="menuOpen" class="gear-menu">
            <button class="gear-item danger" @click="menuOpen = false; confirming = true">
              Retirer cet ami
            </button>
          </div>
        </div>
      </div>
    </header>

    <div v-if="partage" class="stats">
      <div class="stat">
        <b>{{ gameCount }}</b>
        <span>total de jeux</span>
      </div>
      <div class="stat">
        <b>{{ commonCount }}</b>
        <span>en commun avec toi</span>
      </div>
    </div>

    <section class="lib">
      <!-- Pas de compte Torii : c'est LA raison pour laquelle cette page est vide, et elle
           mérite mieux qu'une phrase posée sur du noir. Le bloc porte son propre titre, son
           illustration et — surtout — le geste que la phrase conseille. -->
      <div v-if="!toriiId" class="blank">
        <span class="blank-art" aria-hidden="true">
          <span class="blank-halo" />
          <svg viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round">
            <circle cx="32" cy="32" r="22" />
            <path d="M22.5 26.5h4M37.5 26.5h4" />
            <path d="M23 43.5c2.7-4 5.7-6 9-6s6.3 2 9 6" />
          </svg>
        </span>
        <h2>Torii ne voit presque rien de {{ name }}</h2>
        <p>
          {{ name }} n'est pas dans tes amis Torii. Torii ne sait de lui que ce que Steam veut
          bien dire : son pseudo, son avatar, s'il est connecté et à quoi il joue
          <b>sur Steam</b>. Donne-lui ton code d'ami pour voir tout le reste.
        </p>
        <button v-if="monCode" class="code-chip" title="Copier ton code d'ami" @click="copierCode">
          {{ monCode }}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="9" y="9" width="11" height="11" rx="2" />
            <path d="M5 15V5a2 2 0 0 1 2-2h10" />
          </svg>
        </button>
      </div>

      <!-- Compte Torii, mais partage éteint : la porte existe, elle est simplement fermée
           — et lui seul a la clé. Même habillage, autre glyphe. -->
      <div v-else-if="!partage" class="blank">
        <span class="blank-art" aria-hidden="true">
          <span class="blank-halo" />
          <svg viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round">
            <rect x="12" y="16" width="13" height="34" rx="3" />
            <rect x="29" y="16" width="13" height="34" rx="3" />
            <path d="M47 20l5 32" />
            <path d="M14 12l38 40" stroke-width="2.8" />
          </svg>
        </span>
        <h2>{{ name }} ne partage pas sa bibliothèque</h2>
        <p>
          Le partage est éteint par défaut, et lui seul peut l'allumer, dans ses réglages
          Torii.
        </p>
      </div>

      <p v-else-if="loading && !games.length" class="empty">Chargement de sa bibliothèque…</p>

      <template v-else>
        <h2>Sa bibliothèque</h2>
        <!-- Une seule rangée, quelle que soit la largeur : la coupe est en CSS, pas
             calculée en JS (cf. `.strip`). -->
        <div class="strip">
          <button
            v-for="g in apercu"
            :key="g.key"
            class="tile cover-card"
            :title="g.title"
            @click="showFriendLibrary(toriiId!)"
          >
            <span class="cover" :style="{ background: gradientFor(g.key) }">
              <img v-if="coverOf(g)" class="cover-img" :src="coverOf(g)" alt="" loading="lazy" />
              <span class="cover-scrim" />
              <span class="cover-title">{{ g.title }}</span>
            </span>
            <span v-if="mine(g)" class="tile-mine">Tu l'as aussi</span>
          </button>
        </div>
        <button class="btn-more" @click="showFriendLibrary(toriiId!)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
          Voir plus
        </button>
      </template>
    </section>

    <!-- Confirmation de retrait : une vraie modale, parce que c'est indéfaisable. -->
    <div v-if="confirming" class="modal-back" @click.self="confirming = false">
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="retirer-titre">
        <h3 id="retirer-titre">Retirer {{ name }} ?</h3>
        <p>{{ removeHint }}</p>
        <p class="sub">
          Pour revenir en arrière, il faudra une nouvelle demande, acceptée des deux côtés.
        </p>
        <div class="modal-actions">
          <button class="c-no" @click="confirming = false">Annuler</button>
          <button class="c-yes" :disabled="removing" @click="onRemove">
            {{ removing ? "Retrait…" : "Retirer" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.profile { min-width: 0; }
.sec-head { display: flex; align-items: center; gap: 12px; margin-bottom: 18px; }
.back { display: inline-flex; align-items: center; gap: 6px; }
.back svg { width: 16px; height: 16px; }

/* ── En-tête ─────────────────────────────────────────────────────────────── */
.hero {
  display: flex; align-items: center; gap: 20px; flex-wrap: wrap;
  padding: 20px; border-radius: var(--radius);
  background: var(--surface); border: 1px solid var(--border);
}
.avatar {
  width: 84px; height: 84px; border-radius: 50%; overflow: hidden; flex: none;
  display: grid; place-items: center; background: var(--surface-3);
  border: 1px solid var(--border-strong);
}
.avatar img { width: 100%; height: 100%; object-fit: cover; }
.avatar-fallback { font-size: 26px; font-weight: 700; color: var(--text-dim); }
/* `flex: 1 1 240px` et pas `flex: 1` : à `flex-basis: 0` l'identité se laissait écraser
   par les boutons, le pseudo passait sur deux lignes et la pastille d'état se retrouvait
   seule au-dessus de sa phrase. En dessous de cette largeur, les actions passent à la
   ligne — c'est elles qui doivent céder, pas le nom de la personne. */
.ident { min-width: 0; flex: 1 1 240px; }
.ident h1 { margin: 0 0 6px; font-size: 26px; font-weight: 800; letter-spacing: -0.02em; }
.state {
  margin: 0; display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
  font-size: 13.5px; color: var(--text-dim);
}
.state .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--text-faint); flex: none; }
.state.online .dot { background: #4ade80; }
.state.away .dot { background: #fbbf24; }
.state.in-game .dot { background: var(--accent); }
/* Pastilles de canal : mêmes couleurs et même sens que dans la vue Amis (allumée =
   présent sur ce canal, éteinte = ami de ce côté mais pas connecté). */
.src {
  font-size: 10.5px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase;
  padding: 2px 7px; border-radius: 99px; cursor: default;
}
.src.torii { color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); }
.src.steam { color: var(--steam); background: color-mix(in srgb, var(--steam) 16%, transparent); }
.src.off { color: var(--text-faint); background: var(--surface-2); }
.actions { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.btn-primary {
  padding: 11px 18px; border-radius: 12px; font-weight: 600; font-size: 13.5px;
  background: var(--accent); color: var(--accent-ink); border: none;
}
.btn-primary:hover { background: var(--accent-hover); }
.btn-soft {
  padding: 11px 16px; border-radius: 12px; font-weight: 600; font-size: 13.5px;
  background: var(--surface-2); color: var(--text); border: 1px solid var(--border);
}
.btn-soft:hover { border-color: var(--border-strong); }

/* ── L'écrou et son menu ─────────────────────────────────────────────────── */
.gear-wrap { position: relative; }
.gear {
  width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim);
}
.gear:hover { color: var(--text); border-color: var(--border-strong); }
.gear svg { width: 18px; height: 18px; }
.gear-menu {
  position: absolute; top: calc(100% + 6px); right: 0; z-index: 60; min-width: 190px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 12px;
  box-shadow: var(--shadow-hero); padding: 6px;
}
.gear-item {
  display: block; width: 100%; text-align: left; padding: 9px 11px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13px;
}
.gear-item:hover { background: var(--surface-2); }
.gear-item.danger { color: #ff6b6b; }
.gear-item.danger:hover { background: rgba(255, 107, 107, 0.12); }

/* ── Chiffres ────────────────────────────────────────────────────────────── */
.stats { display: flex; gap: 12px; flex-wrap: wrap; margin-top: 14px; }
.stat {
  flex: 1; min-width: 140px; padding: 14px 16px; border-radius: 14px;
  background: var(--surface); border: 1px solid var(--border);
  display: flex; flex-direction: column; gap: 2px;
}
.stat b {
  font-family: var(--mono); font-size: 24px; font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
}
.stat span { font-size: 12.5px; color: var(--text-dim); }

/* ── Aperçu de la bibliothèque ───────────────────────────────────────────── */
.lib { margin-top: 30px; }
.lib h2 { margin: 0 0 14px; font-size: 17px; font-weight: 700; letter-spacing: -0.02em; }
.empty { color: var(--text-dim); font-size: 14px; margin: 0; max-width: 68ch; line-height: 1.5; }

/* ── Écran vide ──────────────────────────────────────────────────────────────
   Une phrase seule posée sur du noir se lit comme un bug ; un panneau centré qui
   s'annonce, s'illustre et propose le geste suivant se lit comme une réponse. Le même
   habillage sert aux deux vides (hors du cercle Torii / partage éteint), seuls le glyphe
   et le texte changent — sinon les deux cas divergeraient au premier retouchage. */
.blank {
  display: flex; flex-direction: column; align-items: center; text-align: center;
  gap: 12px; padding: 46px 24px 40px; border-radius: var(--radius);
  background: var(--surface); border: 1px dashed var(--border-strong);
  animation: blank-entre 0.45s cubic-bezier(0.2, 0.7, 0.3, 1) both;
}
.blank h2 { margin: 0; font-size: 18px; font-weight: 700; letter-spacing: -0.02em; }
.blank p {
  margin: 0; max-width: 52ch; font-size: 13.5px; line-height: 1.6; color: var(--text-dim);
}
.blank p b { color: var(--text); font-weight: 600; }

/* L'illustration : un glyphe qui respire au-dessus d'un halo qui bat plus lentement que
   lui. Deux rythmes volontairement décalés (5 s / 6,5 s) — synchrones, ça clignote. */
.blank-art {
  position: relative; display: grid; place-items: center;
  width: 108px; height: 108px; margin-bottom: 4px;
}
.blank-halo {
  position: absolute; inset: 0; border-radius: 50%;
  background: radial-gradient(circle, var(--accent-soft) 0%, transparent 68%);
  animation: blank-halo 6.5s ease-in-out infinite;
}
.blank-art svg {
  position: relative; width: 62px; height: 62px; color: var(--text-faint);
  animation: blank-flotte 5s ease-in-out infinite;
}
@keyframes blank-entre {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: none; }
}
@keyframes blank-flotte {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-6px); }
}
@keyframes blank-halo {
  0%, 100% { opacity: 0.55; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.08); }
}
/* Le code d'ami : le geste que la phrase conseille, à portée de clic. Mêmes tirets et même
   monospace que dans la vue Amis — c'est le même objet, il doit se reconnaître. */
.code-chip {
  margin-top: 6px; display: inline-flex; align-items: center; gap: 9px;
  padding: 9px 16px; border-radius: 10px; font-family: var(--mono); font-size: 16px;
  letter-spacing: 0.16em; font-weight: 600;
  background: var(--surface-2); border: 1px dashed var(--border-strong); color: var(--text);
}
.code-chip:hover { border-color: var(--accent); color: var(--accent); }
.code-chip svg { width: 14px; height: 14px; opacity: 0.7; }
/* 🔑 UNE SEULE RANGÉE, sans JavaScript ni mesure : la première rangée est explicite et
   prend sa hauteur, toutes les suivantes sont des rangées implicites de hauteur nulle,
   coupées par `overflow: hidden`. `row-gap: 0` est indispensable — sinon les rangées
   invisibles laisseraient quand même leurs gouttières, soit une bande vide sous la ligne.
   Le nombre de tuiles visibles suit donc la largeur de la fenêtre, tout seul. */
/* 🔑 `--card-min` et pas une largeur en dur : c'est la variable que pilote la densité
   (Paramètres → Général, `usePreferences`). Une jaquette d'aperçu qui garderait sa taille
   pendant que celles de la bibliothèque grossissent ou rétrécissent ferait mentir le
   réglage. Même gouttière que la grille de la bibliothèque, pour la même raison. */
.strip {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(var(--card-min, 178px), 1fr));
  grid-template-rows: auto; grid-auto-rows: 0;
  column-gap: 20px; row-gap: 0; overflow: hidden;
}
/* 🔑 La tuile EST une `.cover-card` (rayon, ombre, survol, voile, titre incrusté : tout
   vient de `style.css`). Je l'avais d'abord redessinée à la main — rayon 12 au lieu de 16,
   titre en 12,5 px au lieu de 19 — exactement la divergence que la mutualisation était
   censée empêcher, et elle a sauté aux yeux dès que les jaquettes ont pris leur vraie
   taille. Ne reste ici que ce qui est propre à l'aperçu. */
.tile { gap: 6px; }
.tile-mine {
  font-size: 11px; font-weight: 600; color: var(--accent);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
/* Centré et en corail : il prolonge la rangée de jaquettes, il ne s'aligne pas sur elle.
   Le corail en fond teinté plutôt qu'en aplat — « Voir sa bibliothèque » en haut de page
   mène au même endroit et porte déjà l'aplat ; deux boutons pleins pour une seule
   destination donneraient deux actions principales là où il n'y en a qu'une. */
.btn-more {
  margin: 18px auto 0; display: flex; align-items: center; gap: 7px;
  padding: 10px 20px; border-radius: 11px; font-size: 13px; font-weight: 600;
  color: var(--accent); background: var(--accent-soft);
  border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
  transition: background 0.15s, border-color 0.15s;
}
.btn-more:hover {
  background: color-mix(in srgb, var(--accent) 22%, transparent);
  border-color: var(--accent);
}
.btn-more svg { width: 16px; height: 16px; }

/* ── Modale de confirmation ──────────────────────────────────────────────── */
.modal-back {
  position: fixed; inset: 0; z-index: 320; display: grid; place-items: center;
  background: rgba(8, 6, 14, 0.62); backdrop-filter: blur(3px); padding: 24px;
}
.modal {
  width: min(430px, 100%); padding: 22px; border-radius: 16px;
  background: var(--surface); border: 1px solid var(--border-strong);
  box-shadow: var(--shadow-hero);
}
.modal h3 { margin: 0 0 10px; font-size: 17px; font-weight: 700; }
.modal p { margin: 0 0 8px; font-size: 13.5px; color: var(--text-dim); line-height: 1.5; }
.modal .sub { font-size: 12.5px; color: var(--text-faint); }
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px; }
.c-yes, .c-no {
  padding: 10px 16px; border-radius: 10px; font-size: 13px; font-weight: 600;
  border: 1px solid var(--border);
}
.c-yes { color: #ff6b6b; background: rgba(255, 107, 107, 0.12); border-color: rgba(255, 107, 107, 0.32); }
.c-yes:hover:not(:disabled) { background: rgba(255, 107, 107, 0.2); }
.c-yes:disabled { opacity: 0.6; }
.c-no { color: var(--text-dim); background: var(--surface-2); }
.c-no:hover { color: var(--text); border-color: var(--border-strong); }
</style>
