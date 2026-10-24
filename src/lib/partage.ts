import { showToast } from "../composables/useToast";
import { t } from "../i18n";
import { pageSteam } from "./boutique";
import { lienPartage } from "./tauri";

/** Copie un lien dans le presse-papiers, et le dit. */
export async function copierLien(url: string) {
  try {
    await navigator.clipboard.writeText(url);
    showToast(t("commun.partage.copie"));
  } catch {
    showToast(t("commun.partage.impossible"));
  }
}

/**
 * Bouton « Partager » d'une fiche : copie la page du jeu à envoyer à un ami — Steam si le
 * jeu y est, sinon Epic, GOG ou son site officiel (cherchés par Rust dans IGDB).
 *
 * `id` : un jeu de la bibliothèque (sa fiche IGDB est en cache) ; sans lui, le titre sert.
 */
export async function partagerJeu(jeu: { appId?: number | null; id?: string | null; titre: string }) {
  const url = jeu.appId ? pageSteam(jeu.appId) : await lienPartage(jeu.id ?? null, jeu.titre);
  if (url) await copierLien(url);
  else showToast(t("commun.partage.aucun"));
}

/** Paramètres d'adresse qui portent une session ou une preuve de connexion. */
const SENSIBLES = new Set([
  "exchangecode", "code", "token", "access_token", "id_token", "refresh_token",
  "auth", "nonce", "sessionid", "session", "ticket", "state", "password",
]);

/**
 * Une adresse de la page web intégrée, prête à être envoyée à quelqu'un. `null` si elle
 * ne doit pas sortir de Torii.
 *
 * 🔑 LES PAGES DE TORII SONT CONNECTÉES À TES COMPTES. À l'arrivée sur Epic, l'adresse est
 * un instant `epicgames.com/id/exchange?exchangeCode=…` : un code à usage unique qui ouvre
 * ta session à qui l'utilise dans les cinq minutes. On partage alors la page visée
 * (`redirectUrl`), jamais le code. Ailleurs, les paramètres de session connus sont retirés.
 */
export function lienPartageable(adresse: string | null | undefined): string | null {
  if (!adresse) return null;
  let url: URL;
  try {
    url = new URL(adresse);
  } catch {
    return null;
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") return null;
  if (url.hostname.endsWith("epicgames.com") && url.pathname.startsWith("/id/")) {
    const cible = url.searchParams.get("redirectUrl");
    return cible && cible !== adresse ? lienPartageable(cible) : null;
  }
  for (const cle of [...url.searchParams.keys()]) {
    if (SENSIBLES.has(cle.toLowerCase())) url.searchParams.delete(cle);
  }
  return url.toString();
}
