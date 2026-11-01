import assert from "node:assert/strict";
import { test } from "node:test";
import {
  adressesCapture, blocs, classification, configuration, decoder, ficheVide, lireDetails, lireItem, texteBrut,
} from "../src/fiche.js";
import { demos } from "../src/steam.js";

// Extrait réel de `pc_requirements` (Cyberpunk 2077, en français, 22/10/2026).
const CONFIG_MIN = '<strong>Minimale :</strong><br><ul class="bb_ul"><li>Système d\'exploitation et processeur 64 bits nécessaires<br></li><li><strong>Système d\'exploitation :</strong> 64-bit Windows 10<br></li><li><strong>Processeur :</strong> Core i7-6700 or Ryzen 5 1600<br></li><li><strong>Mémoire vive :</strong> 12 GB de mémoire<br></li><li><strong>Espace disque :</strong> 70 GB d\'espace disque disponible<br></li><li><strong>Notes supplémentaires :</strong> SSD required.</li></ul>';
const CONFIG_REC = '<strong>Recommandée :</strong><br><ul class="bb_ul"><li>Système d\'exploitation et processeur 64 bits nécessaires<br></li><li><strong>Processeur :</strong> Core i7-12700 or Ryzen 7 7800X3D<br></li><li><strong>Notes supplémentaires :</strong> SSD required.</li></ul>';

test("lit la configuration requise", () => {
  const c = configuration({ minimum: CONFIG_MIN, recommended: CONFIG_REC });
  assert.equal(c.minimale.length, 4);
  assert.deepEqual(c.minimale[0], { libelle: "Système d'exploitation", valeur: "64-bit Windows 10" });
  assert.equal(c.minimale[3].valeur, "70 GB d'espace disque disponible");
  assert.deepEqual(c.recommandee, [{ libelle: "Processeur", valeur: "Core i7-12700 or Ryzen 7 7800X3D" }]);
  // Les deux colonnes répètent les mêmes notes : on ne les garde qu'une fois.
  assert.deepEqual(c.notes, ["Système d'exploitation et processeur 64 bits nécessaires", "SSD required."]);
});

test("pas de configuration sans lignes", () => {
  // Steam renvoie un tableau vide pour les jeux qui n'en déclarent pas.
  assert.equal(configuration([]), null);
});

test("la description devient des blocs sûrs", () => {
  const html =
    '<h2 class="bb_tag">L&#39;AVENTURE</h2><p class="bb_paragraph">Night&nbsp;City &amp; ses <a href="https://exemple.org">rues</a>.<br>Deuxième ligne.</p>' +
    '<ul class="bb_ul"><li>Un <strong>monde</strong> ouvert</li><li>Des choix</li></ul>' +
    '<img class="bb_img" src="https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1/extras/a.gif?t=1" />' +
    '<img src="https://exemple.org/pisteur.png"><script>alert(1)</script><p onclick="x()">Fin</p>';
  const b = blocs(html);
  assert.deepEqual(
    b.map((x) => [x.genre, x.texte]),
    [
      ["h", "L'AVENTURE"],
      ["p", "Night City & ses rues.\nDeuxième ligne."],
      ["li", "Un monde ouvert"],
      ["li", "Des choix"],
      ["img", ""],
      // Le texte d'un <script> reste du texte : il ne sera jamais exécuté.
      ["p", "alert(1)"],
      ["p", "Fin"],
    ],
  );
  assert.ok(b[4].url.startsWith("https://shared.akamai.steamstatic.com/"));
  assert.equal(b.filter((x) => x.genre === "img").length, 1);
});

// Extrait réel (Hades, 22/10/2026) : une vidéo en boucle, puis du texte.
test("garde les vidéos de Steam, pas celles d'ailleurs", () => {
  const html =
    '<span class="bb_img_ctn"><video class="bb_img" autoplay muted loop playsinline poster="https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1145360/extras/c2.poster.avif?t=1" width=600 height=338 >' +
    '<source src="https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1145360/extras/c2.webm?t=1" type="video/webm; codecs=vp9">' +
    '<source src="https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1145360/extras/c2.mp4?t=1" type="video/mp4"></video></span><br><strong>Hades</strong> est un rogue-like.' +
    '<video poster="https://ailleurs.example/p.jpg"><source src="https://ailleurs.example/v.mp4"></video>';
  const b = blocs(html);
  assert.equal(b.length, 2);
  assert.equal(b[0].genre, "video");
  assert.ok(b[0].url.endsWith("/c2.mp4?t=1"));
  assert.ok(b[0].affiche.endsWith("/c2.poster.avif?t=1"));
  assert.deepEqual([b[1].genre, b[1].texte], ["p", "Hades est un rogue-like."]);
});

test("décode les entités", () => {
  assert.equal(decoder("L&#39;&#x27;a &amp; b &lt;c&gt; &bogus; &"), "L''a & b <c> &bogus; &");
});

test("le texte seul d'un fragment", () => {
  assert.equal(texteBrut("<b>Un</b>  jeu&nbsp;&amp; <i>deux</i> > trois"), "Un jeu & deux > trois");
});

test("adresses des captures", () => {
  const c = adressesCapture("steam/apps/1091500/ss_2f64.jpg?t=17");
  assert.equal(c.mini, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1091500/ss_2f64.600x338.jpg?t=17");
  assert.equal(c.grande, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1091500/ss_2f64.1920x1080.jpg?t=17");
});

test("langues et aperçu", () => {
  const item = {
    reviews: { summary_filtered: { review_count: 10, percent_positive: 87, review_score: 8 } },
    tags: [{ tagid: 4115 }, { tagid: 6650 }, { tagid: 1695 }, { tagid: 4182 }, { tagid: 122 }],
    screenshots: { all_ages_screenshots: [{ filename: "steam/apps/1/ss_a.jpg?t=1" }] },
    trailers: {
      highlights: [
        {
          trailer_name: "Rêves brisés",
          screenshot_full: "257/abc/movie_full.jpg",
          microtrailer: [
            { filename: "1/2/micro.webm", type: "video/webm" },
            { filename: "1/2/micro.mp4", type: "video/mp4" },
          ],
          adaptive_trailers: [
            { cdn_path: "1/2/dash_av1.mpd", encoding: "dash_av1" },
            { cdn_path: "1/2/hls_264_master.m3u8", encoding: "hls_h264" },
          ],
        },
        { trailer_name: "Sans vidéo", screenshot_full: "9/x/movie_full.jpg" },
      ],
    },
    supported_languages: [
      { elanguage: 0, supported: true, full_audio: true, subtitles: true },
      { elanguage: 2, supported: true, full_audio: false, subtitles: true },
      { elanguage: 999, supported: true },
    ],
  };
  const noms = new Map([[4115, "Cyberpunk"], [1695, "Monde ouvert"], [122, "RPG"]]);
  const f = ficheVide();
  lireItem(f, item, noms);
  assert.deepEqual(f.tagIds, [4115, 1695, 122]);
  assert.deepEqual(f.tags, ["Cyberpunk", "Monde ouvert", "RPG"]);
  assert.deepEqual(f.etiquettes, [
    { id: 4115, nom: "Cyberpunk" },
    { id: 1695, nom: "Monde ouvert" },
    { id: 122, nom: "RPG" },
  ]);
  assert.equal(f.captures.length, 1);
  assert.ok(f.captures[0].mini.endsWith("ss_a.600x338.jpg?t=1"));
  // La seconde entrée n'a rien à lire : écartée.
  assert.equal(f.bandesAnnonces.length, 1);
  const a = f.bandesAnnonces[0];
  assert.equal(a.hls, "https://video.akamai.steamstatic.com/store_trailers/1/2/hls_264_master.m3u8");
  assert.equal(a.mp4, "https://video.akamai.steamstatic.com/store_trailers/1/2/micro.mp4");
  assert.equal(a.affiche, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/257/abc/movie_full.jpg");
  assert.deepEqual(f.langues, [
    { code: "en", voix: true, sousTitres: true },
    { code: "fr", voix: false, sousTitres: true },
  ]);
  assert.deepEqual([f.reviewScore, f.reviewPercent], [8, 87]);
});

test("classification de la région", () => {
  const notes = {
    pegi: { rating: "18", descriptors: "Violence\r\nSex\r\nBad Language" },
    esrb: { rating: "m", descriptors: "Blood and Gore" },
  };
  const fr = classification(notes, "FR");
  assert.deepEqual([fr.systeme, fr.note], ["PEGI", "18"]);
  assert.deepEqual(fr.descripteurs, ["Violence", "Sex", "Bad Language"]);
  const us = classification(notes, "US");
  assert.deepEqual([us.systeme, us.note], ["ESRB", "M"]);
  // Pas d'USK fourni : l'Allemagne retombe sur PEGI.
  assert.equal(classification(notes, "DE").systeme, "PEGI");
});

// Réponses réelles du 31/10/2026 : Wyldheart (`GetItems`, `appdetails`) et sa démo.
test("démo et jeu complet", () => {
  assert.deepEqual(demos({ appid: 3642910, related_items: { demo_appid: [4475270] } }), [4475270]);
  assert.deepEqual(demos({ appid: 4475270, type: 1, related_items: { parent_appid: 3642910 } }), []);
  assert.deepEqual(demos({ appid: 1091500 }), []);

  const f = ficheVide();
  lireDetails(f, { type: "demo", fullgame: { appid: "3642910", name: "Wyldheart" } }, "FR");
  assert.deepEqual(f.jeuComplet, { appId: 3642910, titre: "Wyldheart" });
  // Sans `GetItems`, les démos viennent d'`appdetails`.
  lireDetails(f, { type: "game", demos: [{ appid: 4475270, description: "" }] }, "FR");
  assert.deepEqual([f.demos, f.jeuComplet], [[4475270], null]);
});

test("les liens et les notes hors de Steam ne passent pas", () => {
  const f = ficheVide();
  lireDetails(f, {
    website: "http://exemple.org",
    metacritic: { score: 92, url: "https://ailleurs.example/" },
    achievements: { total: 0 },
  }, "FR");
  assert.equal(f.site, null);
  assert.equal(f.metacritic, 92);
  assert.equal(f.metacriticUrl, null);
  assert.equal(f.succes, null);
});
