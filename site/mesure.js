/**
 * Mesure d'audience maison : une balise par page vue, une par clic sur « Télécharger ».
 *
 * 🔑 SANS COOKIE ET SANS STOCKAGE. Rien n'est écrit dans le navigateur ; le serveur ne
 * garde ni l'IP ni rien qui permette de reconnaître un visiteur d'un jour à l'autre (voir
 * `server/src/audience.js`). D'où l'absence de bandeau de consentement.
 *
 * `sendBeacon` en `text/plain` : pas de requête de pré-vol, et l'envoi survit au départ
 * de la page (indispensable pour le clic de téléchargement, qui quitte souvent la page).
 * Tout échec est silencieux : la mesure ne doit jamais gêner la visite.
 */
(() => {
  const API = "https://torii-api.topo-host.com/v1/hit";
  // Pas de mesure en local ni sur les copies de travail.
  if (location.hostname !== "torii-app.fr" && location.hostname !== "www.torii-app.fr") return;

  const langue = document.documentElement.lang === "en" ? "en" : "fr";

  function envoyer(type) {
    try {
      const corps = JSON.stringify({ t: type, p: location.pathname, l: langue, r: document.referrer });
      navigator.sendBeacon(API, new Blob([corps], { type: "text/plain" }));
    } catch {
      // Navigateur sans sendBeacon, ou bloqué : tant pis.
    }
  }

  envoyer("vue");

  // Clics sur un lien de téléchargement : les boutons, et le lien vers le .msi.
  document.addEventListener("click", (e) => {
    const lien = e.target.closest?.("#telecharger, #telecharger-bas, #lien-msi");
    if (lien) envoyer("telechargement");
  });
})();
