/** Le script de Cloudflare Turnstile, chargé une fois, et seulement sur la page de connexion web. */

export interface OptionsTurnstile {
  sitekey: string;
  action: string;
  theme: string;
  language: string;
  size: string;
  callback: (jeton: string) => void;
  "expired-callback": () => void;
  "timeout-callback": () => void;
  "error-callback": () => void;
}

export interface Turnstile {
  render(conteneur: HTMLElement, options: OptionsTurnstile): string;
  reset(widget: string): void;
  remove(widget: string): void;
}

declare global {
  interface Window {
    turnstile?: Turnstile;
  }
}

const SCRIPT = "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";

let chargement: Promise<Turnstile> | null = null;

export function chargerTurnstile(): Promise<Turnstile> {
  if (window.turnstile) return Promise.resolve(window.turnstile);
  chargement ??= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = SCRIPT;
    script.async = true;
    script.onload = () => (window.turnstile ? resolve(window.turnstile) : reject(new Error("turnstile absent")));
    script.onerror = () => {
      // Le prochain essai doit recréer la balise : celle-ci ne se relancera pas.
      chargement = null;
      script.remove();
      reject(new Error("turnstile injoignable"));
    };
    document.head.appendChild(script);
  });
  return chargement;
}
