/**
 * Torii — briques communes du service social (réponses, hachage, aléa, temps).
 */

export const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, PATCH, PUT, DELETE, OPTIONS",
  "Access-Control-Allow-Headers": "content-type, authorization",
  "Access-Control-Max-Age": "86400",
};

/** Instant Unix en secondes (toutes les dates du service sont dans cette unité). */
export const now = () => Math.floor(Date.now() / 1000);

export function json(data, status = 200, headers = {}) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", ...CORS, ...headers },
  });
}

/** Erreur applicative : un code stable que le client peut traiter, plus un message lisible. */
export function fail(status, code, message) {
  return json({ error: code, message }, status);
}

/** Corps JSON d'une requête, ou `null` s'il est absent/illisible (jamais d'exception). */
export async function body(request) {
  try {
    return await request.json();
  } catch {
    return null;
  }
}

const ENC = new TextEncoder();

/**
 * Empreinte SHA-256 en hexadécimal, salée par un secret serveur (« poivre »).
 *
 * 🔑 Codes de connexion et jetons de session ne sont JAMAIS stockés en clair : une
 * copie de la base ne permet donc de se connecter à aucun compte. Le poivre ajoute
 * qu'un attaquant ayant la base mais pas le secret ne peut pas non plus pré-calculer
 * les 10⁶ codes possibles à 6 chiffres.
 */
export async function hash(value, pepper) {
  const bytes = await crypto.subtle.digest("SHA-256", ENC.encode(`${pepper}:${value}`));
  return [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** Comparaison à temps constant : ne fuit pas la position du premier caractère faux. */
export function sameHash(a, b) {
  if (typeof a !== "string" || typeof b !== "string" || a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

/** Jeton opaque de 32 octets, en base64url (sûr en en-tête HTTP comme en URL). */
export function randomToken() {
  const raw = crypto.getRandomValues(new Uint8Array(32));
  return btoa(String.fromCharCode(...raw)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** Code de connexion à 6 chiffres, tiré d'une source cryptographique. */
export function randomCode() {
  const [n] = crypto.getRandomValues(new Uint32Array(1));
  return String(n % 1_000_000).padStart(6, "0");
}

/**
 * Identifiant de compte : horodatage + aléa. Trié chronologiquement, non devinable,
 * et sans dépendance externe (pas de bibliothèque ULID à embarquer).
 */
export function newId() {
  const stamp = Date.now().toString(36).padStart(9, "0");
  const rand = [...crypto.getRandomValues(new Uint8Array(8))]
    .map((b) => b.toString(36).padStart(2, "0"))
    .join("");
  return `${stamp}${rand}`;
}

/**
 * Code d'ami : 8 caractères d'un alphabet sans I, O, 0, 1 ni U — on le dicte à
 * l'oral ou on le recopie, donc les confusions visuelles sont exclues, et l'absence
 * de voyelles évite les mots malheureux.
 */
const FRIEND_ALPHABET = "23456789ABCDEFGHJKLMNPQRSTVWXYZ";
export function newFriendCode() {
  const raw = crypto.getRandomValues(new Uint8Array(8));
  return [...raw].map((b) => FRIEND_ALPHABET[b % FRIEND_ALPHABET.length]).join("");
}

/** Normalise une adresse pour qu'elle soit une identité stable (casse, espaces). */
export function normalizeEmail(value) {
  return typeof value === "string" ? value.trim().toLowerCase() : "";
}

/** Validation volontairement permissive : c'est la réception du code qui fait foi. */
export function looksLikeEmail(value) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(value) && value.length <= 254;
}

/** Tronque une chaîne fournie par le client (aucune donnée non bornée n'entre en base). */
export function clamp(value, max) {
  return typeof value === "string" ? value.trim().slice(0, max) : "";
}
