/**
 * Relais des chiffres du service vers Umami (stats.topo-host.com), fiche « Torii · comptes
 * et téléchargements » — pour que toutes les statistiques des projets soient au même endroit.
 *
 * Appelé une fois par nuit, APRÈS les relevés (`releverStats`, `releverGithub`) : il compare
 * les deux derniers relevés de chaque table et envoie des événements datés de la veille.
 *
 * 🔑 UMAMI COMPTE DES ÉVÉNEMENTS, IL NE TRACE PAS DE VALEURS. D'où deux sortes d'envois :
 *  - les FLUX (nouveaux comptes, installeurs téléchargés, vérifications de mise à jour) :
 *    un événement par unité gagnée depuis le relevé précédent. Leur total sur une période
 *    a un sens.
 *  - les JAUGES (« … (relevé du jour) ») : autant d'événements que la valeur du jour. Le
 *    graphique par jour donne la courbe ; leur total sur une période, lui, ne veut rien dire.
 *
 * Aucune donnée personnelle ne part : uniquement des compteurs.
 * Tout échec est journalisé et n'empêche rien d'autre : ce n'est que de la statistique.
 */
const UMAMI = "https://stats.topo-host.com/api/send";
const SITE = "fdc3851d-9461-4933-a0e7-95daa179b9dd";
// ⚠️ Umami ignore les robots, reconnus à leur user-agent : il en faut un de navigateur.
const UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ToriiReleve/1.0";

async function envoyer(nom, fois, horodatage) {
  for (let i = 0; i < fois; i++) {
    const r = await fetch(UMAMI, {
      method: "POST",
      headers: { "content-type": "application/json", "user-agent": UA },
      body: JSON.stringify({
        type: "event",
        payload: {
          website: SITE,
          hostname: "torii-api.topo-host.com",
          url: "/releve",
          language: "fr",
          screen: "0x0",
          name: nom,
          timestamp: horodatage,
        },
      }),
    });
    if (!r.ok) throw new Error(`Umami ${r.status} sur « ${nom} »`);
  }
}

/** Les deux derniers relevés d'une table (le plus récent d'abord). */
async function deuxDerniers(env, table) {
  const r = await env.DB.prepare(`SELECT * FROM ${table} ORDER BY jour DESC LIMIT 2`).all();
  return r.results || [];
}

export async function relayerVersUmami(env) {
  const [stats, github] = await Promise.all([deuxDerniers(env, "stats"), deuxDerniers(env, "github_releve")]);
  // Relevé de la nuit = bilan de la veille : on le date de la veille à midi (UTC).
  const veille = Math.floor(Date.now() / 86_400_000) * 86_400 - 86_400 + 12 * 3600;
  const hausse = (lignes, champ) => (lignes.length === 2 ? Math.max(0, lignes[0][champ] - lignes[1][champ]) : 0);

  await envoyer("Nouveau compte", hausse(stats, "comptes"), veille);
  await envoyer("Installeur téléchargé", hausse(github, "installeurs"), veille);
  await envoyer("Vérification de mise à jour", hausse(github, "controles"), veille);

  const s = stats[0];
  if (s) {
    await envoyer("Comptes au total (relevé du jour)", s.comptes, veille);
    await envoyer("Comptes actifs sur 7 j (relevé du jour)", s.actifs_7j, veille);
    await envoyer("Bibliothèques synchronisées (relevé du jour)", s.biblios, veille);
    await envoyer("Pic de connexions simultanées (relevé du jour)", s.pic_en_ligne, veille);
  }
}
