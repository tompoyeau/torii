import { createApp } from "vue";
import "./style.css";
import App from "./App.vue";
import { logFrontError } from "./lib/tauri";

/**
 * Tout ce qui casse dans l'interface part dans le journal de l'application.
 *
 * 🔑 Sans ça, une erreur de script donne un écran blanc et **aucune trace** : côté Rust
 * tout va bien, et personne ne sait que la vue a cessé de fonctionner. C'est le pendant
 * du gestionnaire de paniques côté natif.
 */
function surveiller() {
  window.addEventListener("error", (e) => {
    const ou = e.filename ? ` (${e.filename}:${e.lineno}:${e.colno})` : "";
    void logFrontError(`${e.message}${ou}\n${e.error?.stack ?? ""}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const raison = e.reason instanceof Error ? `${e.reason.message}\n${e.reason.stack}` : String(e.reason);
    void logFrontError(`promesse rejetée : ${raison}`);
  });
}

surveiller();

const app = createApp(App);
// Erreurs levées dans un composant : Vue les intercepte avant `window.onerror`.
app.config.errorHandler = (err, _instance, info) => {
  const e = err instanceof Error ? `${err.message}\n${err.stack}` : String(err);
  void logFrontError(`composant (${info}) : ${e}`);
  console.error(err);
};
app.mount("#app");
