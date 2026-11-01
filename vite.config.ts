import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

/**
 * Version web : de quoi s'installer sur l'écran d'accueil d'un téléphone. Ajouté ici plutôt que
 * dans `index.html`, partagé avec l'application et la démo, qui n'en ont pas l'usage.
 * Les icônes et le manifeste sont dans `public-web/`.
 */
function pageInstallable(): Plugin {
  return {
    name: "torii-page-installable",
    transformIndexHtml(html) {
      return {
        // Le contenu va jusqu'aux bords de l'écran : les encoches se règlent avec `safe-area-inset`.
        html: html.replace('initial-scale=1.0"', 'initial-scale=1.0, viewport-fit=cover"'),
        tags: [
          { tag: "link", attrs: { rel: "manifest", href: "/manifest.webmanifest" }, injectTo: "head" },
          { tag: "link", attrs: { rel: "apple-touch-icon", href: "/icones/apple-touch-icon.png" }, injectTo: "head" },
          { tag: "meta", attrs: { name: "theme-color", content: "#131019" }, injectTo: "head" },
          { tag: "meta", attrs: { name: "apple-mobile-web-app-capable", content: "yes" }, injectTo: "head" },
          { tag: "meta", attrs: { name: "apple-mobile-web-app-title", content: "Torii" }, injectTo: "head" },
          { tag: "meta", attrs: { name: "apple-mobile-web-app-status-bar-style", content: "black" }, injectTo: "head" },
        ],
      };
    },
  };
}

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: [vue(), ...(mode === "web" ? [pageInstallable()] : [])],

  // La version web a ses propres fichiers publics : rien d'elle n'entre dans l'application.
  publicDir: mode === "web" ? "public-web" : "public",

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
