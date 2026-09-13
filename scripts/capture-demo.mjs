/**
 * Prend la capture d'en-tête du site anglais (`site/captures/library-en.jpg`) depuis la
 * démo, en anglais, à la taille exacte de la capture française (1700×1020).
 *
 *     python -m http.server 5602 --directory site      (dans un autre terminal)
 *     node scripts/capture-demo.mjs "http://localhost:5602/demo/?lang=en" site/captures/library-en.jpg
 *
 * 🔑 POURQUOI DEPUIS LA DÉMO ET PAS DEPUIS L'APPLICATION. La capture française est une vraie
 * bibliothèque, anonymisée à la main (voir `site/README.md`). Celle-ci est prise sur la
 * bibliothèque fictive de la démo : rien à anonymiser, et elle se refait en une commande
 * après chaque évolution de l'interface. Contrepartie assumée : 26 jeux au lieu de 848.
 *
 * Edge sans interface, piloté par le protocole DevTools — aucune dépendance à installer :
 * Node fournit `fetch` et `WebSocket`, Edge est présent sur tout Windows.
 * Le thème sombre est forcé (`prefers-color-scheme`), le bandeau « Démo » retiré.
 * ⚠️ Relancer `npm run build:demo` AVANT : la capture montre la démo telle qu'elle est bâtie.
 */
import { spawn } from "node:child_process";
import { writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const EDGE = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe";
const URL_DEMO = process.argv[2];
const SORTIE = process.argv[3];
const PORT = 9333;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

const profil = mkdtempSync(join(tmpdir(), "torii-capture-"));
const edge = spawn(EDGE, [
  "--headless=new", `--remote-debugging-port=${PORT}`, `--user-data-dir=${profil}`,
  "--window-size=1700,1020", "--hide-scrollbars", "--no-first-run", "about:blank",
], { stdio: "ignore" });

try {
  let cible;
  for (let i = 0; i < 40 && !cible; i++) {
    await pause(250);
    try {
      const liste = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
      cible = liste.find((t) => t.type === "page");
    } catch {}
  }
  if (!cible) throw new Error("Edge ne répond pas sur le port de débogage");

  const ws = new WebSocket(cible.webSocketDebuggerUrl);
  await new Promise((ok, ko) => { ws.onopen = ok; ws.onerror = ko; });
  let id = 0;
  const attente = new Map();
  ws.onmessage = (m) => {
    const d = JSON.parse(m.data);
    if (d.id && attente.has(d.id)) { attente.get(d.id)(d); attente.delete(d.id); }
  };
  const cdp = (method, params = {}) => new Promise((ok) => {
    const n = ++id; attente.set(n, ok); ws.send(JSON.stringify({ id: n, method, params }));
  });

  await cdp("Emulation.setDeviceMetricsOverride", { width: 1700, height: 1020, deviceScaleFactor: 1, mobile: false });
  await cdp("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] });
  await cdp("Page.enable");
  await cdp("Page.navigate", { url: URL_DEMO });
  await pause(5000); // écran de démarrage + jaquettes

  const etat = await cdp("Runtime.evaluate", {
    expression: `(() => {
      document.querySelector('.demo-banner')?.remove();
      return JSON.stringify({ lang: document.documentElement.lang, titre: document.title,
        vedette: document.querySelector('.hero-eyebrow')?.textContent.trim(),
        splash: !!document.querySelector('.splash'),
        imagesPretes: [...document.images].filter(i => i.complete).length + '/' + document.images.length });
    })()`,
    returnByValue: true,
  });
  console.log(etat.result?.result?.value);
  await pause(800);

  const shot = await cdp("Page.captureScreenshot", { format: "jpeg", quality: 82, captureBeyondViewport: false });
  writeFileSync(SORTIE, Buffer.from(shot.result.data, "base64"));
  console.log("capture écrite :", SORTIE);
  ws.close();
} finally {
  edge.kill();
}
