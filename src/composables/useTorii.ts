import { computed, ref } from "vue";
import {
  onToriiCircle, toriiCircle, toriiInvite, toriiLogout, toriiMe, toriiMutedGames,
  toriiMuteGame, toriiPrefs, toriiRemoveFriend, toriiRequestCode, toriiRespond,
  toriiRotateCode, toriiSetPrefs, toriiSetProfile, toriiVerify,
} from "../lib/tauri";
import type { SocialPrefs, ToriiAccount, ToriiCircle } from "../types";

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
const prefs = ref<SocialPrefs>({ sharePresence: false, awayAfterMinutes: 10 });
const mutedGames = ref<string[]>([]);
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
  if (account.value) void refresh();
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

async function verify(email: string, code: string) {
  account.value = await toriiVerify(email.trim(), code.trim());
  await refresh();
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

/** Lie (ou délie) son compte Steam, et autorise ou non les amis Steam à nous trouver. */
async function setSteamLink(steamId: string | null, discoverable: boolean) {
  account.value = await toriiSetProfile({ steamId, steamDiscoverable: discoverable });
}

async function setPrefs(next: Partial<SocialPrefs>) {
  prefs.value = await toriiSetPrefs({ ...prefs.value, ...next });
}

/** Ajoute ou retire un jeu de la liste « ne jamais diffuser ». */
async function setMuted(gameId: string, muted: boolean) {
  mutedGames.value = await toriiMuteGame(gameId, muted);
}

function isMuted(gameId: string): boolean {
  return mutedGames.value.includes(gameId);
}

/* ── Amis ──────────────────────────────────────────────────────────────────── */

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
    mutedGames,
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
    setPrefs,
    setMuted,
    isMuted,
    invite,
    respond,
    removeFriend,
    rotateCode,
    stop: () => {
      unlisten?.();
      unlisten = null;
    },
  };
}
