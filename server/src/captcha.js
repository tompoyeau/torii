/**
 * Torii — ce qui change quand un appel vient d'une page web et non de l'application.
 *
 * Deux règles pour la version web : le contrôle anti-robot (Cloudflare Turnstile) avant d'envoyer
 * un code, et aucune création de compte. Un compte Torii se crée dans l'application.
 *
 * 🔑 C'EST `Origin` QUI LES DISTINGUE. Un navigateur le pose toujours sur un POST, et une page
 * ne peut pas l'enlever ; l'application parle depuis Rust et n'en envoie jamais. Une page tierce
 * qui ferait appeler `request-code` par le navigateur de ses visiteurs, autant d'IP différentes,
 * passe donc par le contrôle elle aussi, et n'a pas de jeton à présenter pour notre site.
 *
 * ⚠️ Un script, lui, peut omettre `Origin` et se faire passer pour l'application : le contrôle ne
 * l'arrête pas. Les limites de `index.js` restent le rempart de ce chemin-là, comme avant, parce
 * qu'il faudrait sinon un contrôle dans l'application, donc une version pour tout le monde.
 */

const SITEVERIFY = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
/** Ce que la page annonce en demandant son jeton (`src/components/CaptchaTurnstile.vue`). */
const ACTION = "connexion";
const MAX_JETON = 2048;
const DELAI_MS = 10_000;

/** L'appel vient-il d'une page web ? */
export const depuisUnePage = (request) => request.headers.has("origin");

/**
 * Le jeton présenté par la page est-il valable ? Il doit l'être pour cette action et pour l'une de
 * nos pages (`TURNSTILE_HOSTNAMES`, séparées par des virgules).
 *
 * 🔑 Le doute se solde par un refus : sans secret, sans réponse de Cloudflare ou sur une réponse
 * illisible. L'application, elle, n'est pas concernée et continue de se connecter.
 */
export async function captchaValide(env, jeton, ip) {
  const pages = new Set((env.TURNSTILE_HOSTNAMES ?? "").split(",").map((p) => p.trim()).filter(Boolean));
  if (!env.TURNSTILE_SECRET || !pages.size) {
    console.error("contrôle anti-robot mal configuré : TURNSTILE_SECRET ou TURNSTILE_HOSTNAMES manquant");
    return false;
  }
  if (typeof jeton !== "string" || !jeton || jeton.length > MAX_JETON) return false;

  const corps = new URLSearchParams({ secret: env.TURNSTILE_SECRET, response: jeton });
  if (ip) corps.set("remoteip", ip);

  let resultat;
  try {
    const reponse = await fetch(SITEVERIFY, { method: "POST", body: corps, signal: AbortSignal.timeout(DELAI_MS) });
    if (!reponse.ok) throw new Error(`siteverify ${reponse.status}`);
    resultat = await reponse.json();
  } catch (e) {
    console.error("contrôle anti-robot injoignable —", e);
    return false;
  }

  // Les clés d'essai de Cloudflare ne rendent aucune action. Elles ne passent pas en production pour
  // autant : leur page est toujours `example.com`, absente de `TURNSTILE_HOSTNAMES`.
  const action = resultat.metadata?.result_with_testing_key === true ? ACTION : resultat.action;
  return resultat.success === true && action === ACTION && pages.has(resultat.hostname);
}
