/**
 * Relève, pays par pays, ce que le comparateur de prix renvoie vraiment : nombre d'offres
 * et devise, pour trois jeux très répandus. Passe par le proxy de Torii, comme l'application.
 *
 *     node scripts/sonde-regions.mjs FR US MX JP        (codes ISO à tester)
 *
 * 🔑 À LANCER AVANT D'AJOUTER UN PAYS à `REGIONS` (`src/i18n/regions.ts`). La devise ne se
 * devine pas : le relevé du 13 septembre 2026 a montré la Suisse et la Suède tarifées en
 * euros, et le Mexique ou Singapour recevant les prix américains en dollars. Un pays qui
 * répond en EUR doit aussi entrer dans `locale::zone_euro` côté Rust.
 */
const P = "https://torii-igdb-proxy.toriiapp.workers.dev/itad";
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

const appids = [1086940, 1091500, 413150]; // Baldur's Gate 3, Cyberpunk 2077, Stardew Valley
const ids = [];
for (const a of appids) {
  const r = await (await fetch(`${P}/games/lookup/v1?appid=${a}`)).json();
  if (r.found) ids.push(r.game.id);
  await pause(300);
}

const CANDIDATS = process.argv.slice(2);
const resultats = [];
for (const pays of CANDIDATS) {
  try {
    const rep = await fetch(`${P}/games/prices/v3?country=${pays}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(ids),
    });
    const texte = await rep.text();
    let donnees;
    try { donnees = JSON.parse(texte); } catch { donnees = null; }
    if (!Array.isArray(donnees)) {
      resultats.push({ pays, statut: rep.status, offres: 0, devises: "", note: texte.slice(0, 80) });
    } else {
      const deals = donnees.flatMap((e) => e.deals ?? []);
      const devises = [...new Set(deals.map((d) => d.price?.currency))].join(",");
      const boutiques = new Set(deals.map((d) => d.shop?.name)).size;
      resultats.push({ pays, statut: rep.status, offres: deals.length, boutiques, devises });
    }
  } catch (e) {
    resultats.push({ pays, erreur: String(e) });
  }
  await pause(700);
}
console.log(JSON.stringify(resultats));
