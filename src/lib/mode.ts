/**
 * La version web (`npm run dev:web`, `npm run build:web`) : le même front que l'application,
 * connecté à un compte Torii au lieu d'un PC. Voir `lib/web.ts`.
 *
 * 🔑 UN TROISIÈME CAS, PAS UN DEUXIÈME. Le navigateur nu servait déjà à la démo du site, avec
 * une bibliothèque inventée. La présence du pont natif ne suffit donc plus à décider : sans
 * lui, on peut être dans la démo (`modeDemo`, données fictives) comme dans la version web
 * (vraies données, mais ni jeu à lancer ni launcher à connecter). Seule la compilation les
 * distingue, c'est pourquoi c'est une variable d'environnement.
 */
export const MODE_WEB = import.meta.env.VITE_TORII_WEB === "1";
