import { computed, ref } from "vue";
import {
  getSettings,
  onToriiCircle, toriiCircle, toriiInvite, toriiInviteAccount, toriiLogout, toriiMe,
  toriiMutedGames, toriiMuteGame, toriiPrefs, toriiRemoveFriend, toriiRequestCode,
  toriiRespond, toriiRotateCode, toriiSetPrefs, toriiSetProfile, toriiSuggestions,
  toriiVerify,
} from "../lib/tauri";
import type { ToriiPerson } from "../types";
import type { PresenceMode, SocialPrefs, ToriiAccount, ToriiCircle } from "../types";

/**
 * État du réseau Torii (compte, amis, présence, réglages de partage).
 *
 * Le cercle arrive de deux façons : à la demande (`refresh`) et surtout **poussé** par
 * le battement de cœur côté Rust, qui publie la présence et reçoit celle des amis dans
 * la même requête. L'interface se met donc à jour toute seule toutes les 30 secondes,
 * sans rien réclamer.
 */

const account = ref<ToriiAccount | null>(null);
const circle = ref<ToriiCircle>({ friends: [], incoming: [], outgoing: [] });
const prefs = ref<SocialPrefs>({
  presenceMode: "offline",
  sharePresence: false,
  awayAfterMinutes: 10,
  notifyFriendLaunch: true,
  steamAutoLinked: false,
});
const mutedGames = ref<string[]>([]);
/** Amis Steam qui se trouvent avoir un compte Torii découvrable. */
const suggestions = ref<ToriiPerson[]>([]);
const searching = ref(false);
/** Vrai une fois la recherche faite, pour distinguer « rien trouvé » de « pas cherché ». */
const searched = ref(false);
const loading = ref(false);
const booted = ref(false);

let started = false;
let unlisten: (() => void) | null = null;

/** Charge l'état initial et s'abonne au flux de présence. Une seule fois. */
async function start() {
  if (started) return;
  started = true;
  account.value = await toriiMe();
  prefs.value = await toriiPrefs();
  mutedGames.value = await toriiMutedGames();
  booted.value = true;
  if (account.value) {
    void reconcilierSteam();
    void refresh();
  }
  unlisten = await onToriiCircle((next) => {
    circle.value = next;
  });
}

/** Recharge le cercle à la demande (après une invitation, une acceptation…). */
async function refresh() {
  if (!account.value) return;
  loading.value = true;
  try {
    circle.value = await toriiCircle();
  } catch {
    // Panne réseau : on garde ce qu'on affiche déjà, le battement de cœur reprendra.
  } finally {
    loading.value = false;
  }
}

/* ── Connexion ─────────────────────────────────────────────────────────────── */

/**
 * Demande un code par e-mail. Renvoie le code lui-même si le serveur tourne en mode
 * développement — l'interface l'affiche alors, puisqu'aucun e-mail ne partira.
 */
async function requestCode(email: string): Promise<string | null> {
  return await toriiRequestCode(email.trim());
}

/** Renvoie `true` si le compte vient d'être créé — le front propose alors un pseudo. */
async function verify(email: string, code: string): Promise<boolean> {
  const signIn = await toriiVerify(email.trim(), code.trim());
  account.value = signIn.account;
  await reconcilierSteam();
  await refresh();
  return signIn.created;
}

/**
 * Fait remonter le SteamID dans le compte Torii dès que les deux connexions existent.
 *
 * Les deux se font dans n'importe quel ordre — Steam puis Torii, ou l'inverse, parfois à
 * des jours d'intervalle. On appelle donc ce rapprochement aux trois moments où l'état
 * peut changer : au démarrage, à la connexion Torii, et juste après une connexion Steam.
 * Tant qu'il manque une des deux moitiés, la fonction ne fait rien et retentera plus tard.
 *
 * 🔑 Une seule fois dans la vie du compte, mémorisé dans les réglages. Sans ce drapeau,
 * « jamais lié » et « visibilité éteinte à la main » sont indiscernables — le SteamID est
 * effacé dans les deux cas — et on rallumerait à chaque démarrage ce que la personne
 * vient d'éteindre.
 *
 * 🔑 Le SteamID et la visibilité partent ENSEMBLE. Se déclarer visible sans identifiant
 * lié afficherait un interrupteur allumé qui ne rapproche rien.
 */
async function reconcilierSteam() {
  if (!account.value || prefs.value.steamAutoLinked) return;
  try {
    // Déjà lié (par la personne elle-même, ou sur une autre machine) : rien à faire,
    // mais on note que c'est réglé.
    if (account.value.steamId) return await marquerSteamRapproche();

    const steamId = (await getSettings())?.steamId;
    if (!steamId) return; // Steam pas encore connecté : on réessaiera.

    account.value = await toriiSetProfile({ steamId, steamDiscoverable: true });
    await marquerSteamRapproche();
  } catch {
    // Panne réseau ou compte Steam illisible : ce n'est qu'un réglage de confort, il ne
    // doit jamais faire échouer une connexion. On retentera au prochain démarrage.
  }
}

async function marquerSteamRapproche() {
  if (!prefs.value.steamAutoLinked) await setPrefs({ steamAutoLinked: true });
}

async function logout() {
  await toriiLogout();
  account.value = null;
  circle.value = { friends: [], incoming: [], outgoing: [] };
}

/* ── Profil et réglages ────────────────────────────────────────────────────── */

async function setDisplayName(name: string) {
  account.value = await toriiSetProfile({ displayName: name });
}

/**
 * Lie (ou délie) son compte Steam, et autorise ou non les amis Steam à nous trouver.
 * `steamId` vide = délier : un `null` ne saurait pas exprimer « efface » (cf. `social.rs`).
 */
async function setSteamLink(steamId: string | null, discoverable: boolean) {
  account.value = await toriiSetProfile({ steamId, steamDiscoverable: discoverable });
}

async function setPrefs(next: Partial<SocialPrefs>) {
  prefs.value = await toriiSetPrefs({ ...prefs.value, ...next });
}

/**
 * Mode effectif. Un compte configuré avant l'arrivée des trois états n'a que l'ancien
 * booléen : on le traduit ici comme le fait le cœur Rust, pour que l'interface montre
 * la même chose que ce qui est réellement publié.
 */
const presenceMode = computed<PresenceMode>(() => {
  const m = prefs.value.presenceMode;
  if (m === "offline" || m === "online" || m === "detailed") return m;
  return prefs.value.sharePresence ? "detailed" : "offline";
});

/** Change ce qu'on laisse voir. On écrit toujours le mode, jamais l'ancien booléen. */
async function setPresenceMode(mode: PresenceMode) {
  await setPrefs({ presenceMode: mode, sharePresence: mode !== "offline" });
}

/** Ajoute ou retire un jeu de la liste « ne jamais diffuser ». */
async function setMuted(gameId: string, muted: boolean) {
  mutedGames.value = await toriiMuteGame(gameId, muted);
}

function isMuted(gameId: string): boolean {
  return mutedGames.value.includes(gameId);
}

/* ── Amis ──────────────────────────────────────────────────────────────────── */

/**
 * Cherche, parmi les SteamID fournis, ceux qui ont un compte Torii.
 *
 * 🔑 Le serveur exige que **les deux** personnes se soient rendues découvrables : sans
 * cette double condition, envoyer une liste de SteamID suffirait à cartographier les
 * utilisateurs de Torii. Une liste vide n'a donc rien d'anormal.
 */
async function findSteamFriends(steamIds: string[]) {
  searching.value = true;
  try {
    const known = new Set(circle.value.friends.map((f) => f.id));
    suggestions.value = (await toriiSuggestions(steamIds)).filter((p) => !known.has(p.id));
    searched.value = true;
  } finally {
    searching.value = false;
  }
}

/** Envoie une demande à quelqu'un trouvé par suggestion. */
async function inviteAccount(accountId: string) {
  await toriiInviteAccount(accountId);
  suggestions.value = suggestions.value.filter((p) => p.id !== accountId);
  await refresh();
}

async function invite(friendCode: string) {
  await toriiInvite(friendCode.trim());
  await refresh();
}

async function respond(accountId: string, accept: boolean) {
  await toriiRespond(accountId, accept);
  await refresh();
}

async function removeFriend(accountId: string) {
  await toriiRemoveFriend(accountId);
  await refresh();
}

/** Régénère son code d'ami : l'ancien cesse aussitôt de fonctionner. */
async function rotateCode() {
  const code = await toriiRotateCode();
  if (account.value) account.value = { ...account.value, friendCode: code };
  return code;
}

export function useTorii() {
  void start();
  return {
    account,
    circle,
    prefs,
    presenceMode,
    mutedGames,
    suggestions,
    searching,
    searched,
    loading,
    booted,
    connected: computed(() => account.value !== null),
    pendingCount: computed(() => circle.value.incoming.length),
    refresh,
    requestCode,
    verify,
    logout,
    setDisplayName,
    setSteamLink,
    reconcilierSteam,
    setPrefs,
    setPresenceMode,
    setMuted,
    isMuted,
    invite,
    inviteAccount,
    findSteamFriends,
    respond,
    removeFriend,
    rotateCode,
    stop: () => {
      unlisten?.();
      unlisten = null;
    },
  };
}
