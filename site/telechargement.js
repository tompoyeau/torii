/**
 * Remplit le bouton de téléchargement avec la dernière version publiée.
 *
 * 🔑 POURQUOI CE SCRIPT EXISTE. Le nom de l'installeur contient le numéro de version
 * (`Torii_0.19.0_x64-setup.exe`) : il n'existe donc aucune URL fixe vers « le dernier
 * installeur », et une page statique ne peut pas la deviner. On demande à l'API GitHub.
 *
 * ⚠️ On ne peut PAS lire `latest.json` (le manifeste de l'auto-updater), qui contiendrait
 * pourtant exactement ce qu'il faut : les fichiers de release GitHub sont servis **sans
 * en-tête CORS**, donc le navigateur refuse. L'API, elle, répond
 * `Access-Control-Allow-Origin: *`. Vérifié, ne pas refaire le détour.
 *
 * 🔑 Le HTML fonctionne SANS ce script : le bouton pointe déjà vers la page des versions.
 * Ce fichier ne fait qu'améliorer — il ne casse rien s'il échoue, si GitHub est
 * injoignable, ou si le visiteur a épuisé son quota d'API (60 requêtes par heure et par
 * adresse IP, ce qu'un humain n'atteint jamais).
 */
const DEPOT = "tompoyeau/torii";

/** Cherche un fichier de la release par son extension. */
function fichier(assets, extension) {
  return assets.find((a) => a.name.toLowerCase().endsWith(extension));
}

/** « 4,3 Mo » — l'ordre de grandeur rassure avant de cliquer. */
function poids(octets) {
  return `${(octets / 1048576).toFixed(1).replace(".", ",")} Mo`;
}

/** « 6 septembre 2026 » : une date lisible vaut mieux qu'un horodatage. */
function date(iso) {
  return new Date(iso).toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

async function remplirLeBouton() {
  // Deux boutons identiques, en haut et en bas de page : le visiteur qui a lu jusqu'au
  // bout ne doit pas avoir a remonter.
  const boutons = ["telecharger", "telecharger-bas"].map((id) => document.getElementById(id));
  const textes = ["cta-texte", "cta-texte-bas"].map((id) => document.getElementById(id));
  const bouton = boutons[0];
  const texte = textes[0];
  const meta = document.getElementById("meta");
  const lienMsi = document.getElementById("lien-msi");
  const piedVersion = document.getElementById("pied-version");
  if (!bouton) return;

  let release;
  try {
    const reponse = await fetch(`https://api.github.com/repos/${DEPOT}/releases/latest`, {
      headers: { accept: "application/vnd.github+json" },
    });
    if (!reponse.ok) return; // quota atteint, réseau coupé : les liens de repli restent
    release = await reponse.json();
  } catch {
    return;
  }

  const assets = release.assets || [];
  const exe = fichier(assets, ".exe");
  const msi = fichier(assets, ".msi");
  // Sans installeur exploitable, on ne touche à rien : le repli vaut mieux qu'un bouton
  // qui promet un téléchargement et ouvre une page.
  if (!exe) return;

  const version = (release.tag_name || "").replace(/^v/, "");

  for (const b of boutons) {
    if (!b) continue;
    b.href = exe.browser_download_url;
    // `download` demande au navigateur d'enregistrer plutôt que de naviguer.
    b.setAttribute("download", "");
  }
  for (const t of textes) {
    if (t) t.textContent = `Télécharger Torii ${version}`;
  }

  if (meta) {
    meta.textContent = `Windows 10 et 11 · 64 bits · ${poids(exe.size)} · publié le ${date(
      release.published_at,
    )}`;
  }

  if (msi && lienMsi) {
    lienMsi.href = msi.browser_download_url;
    lienMsi.setAttribute("download", "");
    lienMsi.textContent = `ou le .msi (${poids(msi.size)})`;
  }

  if (piedVersion) piedVersion.textContent = `version ${version}`;
}

remplirLeBouton();
