/**
 * Mesure d'audience : Umami auto-hébergé (stats.topo-host.com), fiche « Torii · site + démo ».
 *
 * 🔑 SANS COOKIE ET SANS STOCKAGE. Umami ne dépose rien dans le navigateur et ne garde pas
 * l'IP : il en tire une empreinte qui change chaque mois. D'où l'absence de bandeau de
 * consentement. Toutes les statistiques des projets sont au même endroit, dans Umami.
 *
 * Les pages vues partent toutes seules ; ce fichier ajoute les clics qui comptent
 * (téléchargement, démo, GitHub). Tout échec est silencieux : la mesure ne doit jamais
 * gêner la visite. Le nom du fichier est gardé pour ne pas retoucher les pages.
 */
(() => {
  // Pas de mesure en local ni sur les copies de travail.
  if (location.hostname !== "torii-app.fr" && location.hostname !== "www.torii-app.fr") return;

  const s = document.createElement("script");
  s.defer = true;
  s.src = "https://stats.topo-host.com/t.js";
  s.dataset.websiteId = "f6c0d016-a53f-48a3-9e1c-0bdc24280cdc";
  document.head.appendChild(s);

  const langue = document.documentElement.lang === "en" ? "en" : "fr";

  function evenement(lien) {
    if (lien.matches("#lien-msi")) return ["Téléchargement", { fichier: "msi" }];
    if (lien.matches("#telecharger, #telecharger-bas")) return ["Téléchargement", { fichier: "exe", bouton: lien.id }];
    const url = new URL(lien.href, location.href);
    if (url.host === location.host && url.pathname.startsWith("/demo/")) return ["Démo ouverte"];
    if (url.hostname === "github.com") return [url.pathname.endsWith("/issues") ? "GitHub · signaler" : "GitHub · code"];
    return null;
  }

  // Phase de capture : le clic est compté avant que le lien ne quitte la page.
  document.addEventListener("click", (e) => {
    const lien = e.target.closest?.("a[href]");
    const ev = lien && evenement(lien);
    if (ev && window.umami) window.umami.track(ev[0], { langue, ...ev[1] });
  }, true);
})();
