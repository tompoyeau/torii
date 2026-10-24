import { t, type Cle } from "../i18n";
import type { StoreBrowse, StoreBrowseSort, StoreHome, StoreItem, StoreShelfItem, StoreShelfKind, StoreTag } from "../types";

/**
 * Données de démonstration de l'accueil de la Boutique (démo web, hors Tauri).
 *
 * 🔑 UN RELEVÉ RÉEL, PAS UNE INVENTION : titres, visuels, avis, dates de sortie et prix
 * sont ceux de la Boutique française du 22 octobre 2026. Les images viennent de Steam et
 * du comparateur (adresses à empreinte, voir `metadata/steam_art.rs`). Seules les dates
 * de fin des jeux offerts sont recalculées à chaque chargement, pour que la démo ne
 * montre jamais une offre expirée.
 */

const STEAM = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/";
const ITAD = "https://assets.isthereanydeal.com/";

type CleGenre = "action" | "aventure" | "rpg" | "strategie" | "simulation" | "tir" | "course" | "sport" | "reflexion" | "independant";

/** [appid, titre, capsule, header, hero, sortie, score, %, prix, prix normal, remise, boutique, à venir, genres] */
type Ligne = [number, string, string, string, string, string, number, number, number | null, number, number, string, boolean, CleGenre[]];

const JEUX: Ligne[] = [
  [1091500, "Cyberpunk 2077", "1091500/6399de67cce3cecadce7900e03f6e09ff910a235/library_capsule_2x.jpg?t=1792663801", "1091500/48a93d6445a67520ea49833c93dae8f2e68fc540/header_alt_assets_9.jpg?t=1792663801", "1091500/cf8cec802dc47d0f24b75f9eee135e96812e2652/library_hero.jpg?t=1792663801", "2020-12-10", 8, 87, 17.99, 59.99, 70, "GOG", false, ["rpg", "action"]],
  [1174180, "Red Dead Redemption 2", "1174180/library_600x900_2x.jpg?t=1759502961", "1174180/header.jpg?t=1759502961", "1174180/library_hero.jpg?t=1759502961", "2019-12-05", 8, 92, 13.68, 53.59, 74, "GamesPlanet US", false, ["aventure", "action"]],
  [381210, "Dead by Daylight", "381210/b15601f9dcf601568761aabc8a7a194267b03068/library_capsule_2x.jpg?t=1791302579", "381210/23fed9f741423413769ad1617e41519d1a466cdc/header_alt_assets_10.jpg?t=1791302579", "381210/70198f089662e803447976de71a963ddddfebc61/library_hero.jpg?t=1791302579", "2016-06-14", 6, 78, 7.14, 17.86, 60, "WinGameStore", false, ["action"]],
  [2807960, "Battlefield™ 6", "2807960/5efba891507f611d94ef6685cc2af7cd0640f55d/library_capsule_2x.jpg?t=1792168547", "2807960/c12d12ce3c7d217398d3fcad77427bfc9d57c570/header.jpg?t=1792168547", "2807960/7165682cb6055d92bd2aafd940edbe70cb33ff6b/library_hero.jpg?t=1792168547", "2025-10-10", 5, 62, 27.99, 69.99, 60, "Epic Game Store", false, ["tir", "action"]],
  [4435490, "Call of Duty®: Modern Warfare® 4", "4435490/c71bc2cb60e440d1a0d2435d0c06c4d4a4050edf/library_capsule_2x.jpg?t=1792523954", "4435490/0c492be3204835d71e8b454b6eb1817f8ff68c0d/header.jpg?t=1792523954", "4435490/4138eadd1bb476b0499f6ee7690aba461fdf179c/library_hero.jpg?t=1792523954", "2026-10-23", 8, 91, 69.99, 69.99, 0, "Steam", false, ["tir", "action"]],
  [3219030, "Planet Zoo 2", "3219030/cc1d879275c41850ecd2e7296f314240e8cb768a/library_capsule_2x.jpg?t=1792660647", "3219030/a5131aa4b84d04a55e3b9c561d7c886cc22bbe1c/header.jpg?t=1792660647", "3219030/28d1cd9fdd7aa6640a13fa27afa4248bc415e483/library_hero.jpg?t=1792660647", "2026-10-13", 5, 61, 39.1, 49.99, 22, "Zapagames", false, ["simulation", "strategie"]],
  [4080220, "EA SPORTS FC™ 27", "4080220/81210d536b70c14e4cf9725f3f03d82bf64e5e25/library_capsule_2x.jpg?t=1790973254", "4080220/ff6a3779f1ca83aae8ad3878454f6c11008ea24d/header.jpg?t=1790973254", "4080220/98909665528dc40a7d43fd3dd6edc8088ed98c44/library_hero.jpg?t=1790973254", "2026-09-24", 5, 49, 69.99, 69.99, 0, "Steam", false, ["sport"]],
  [1203620, "Enshrouded", "1203620/0f641453e3418029205a49c1960e5ae5cb1ce382/library_capsule_2x.jpg?t=1792078032", "1203620/98ca939f51b39bdd41e0e9198a9ccf8421148770/header_alt_assets_2.jpg?t=1792078032", "1203620/65bf9ce18aee402c94c5e370f0a4c7c87119f910/library_hero.jpg?t=1792078032", "2026-10-15", 8, 85, 35.99, 39.99, 10, "Steam", false, ["rpg", "aventure"]],
  [2929250, "over the hill", "2929250/aa93232142ad5e206238a49b003a35d6e9fefb60/library_600x900_2x.jpg?t=1792666389", "2929250/header.jpg?t=1792666389", "2929250/8511fad37dfe8ccedee5790c95287fb7d6de9fb9/library_hero.jpg?t=1792666389", "2026-10-14", 8, 92, 17.63, 22.32, 21, "WinGameStore", false, ["course", "aventure"]],
  [1867240, "WARDOGS", "1867240/31dd6ce5538a3064749d769527656f9002a382b0/library_capsule_2x.jpg?t=1791928607", "1867240/8d0cf7b0176267e49f7f87ca7951eaf64eefb203/header_alt_assets_0_french.jpg?t=1791928607", "1867240/9e3d0dba457f3d33734990866160be80c399f67c/library_hero.jpg?t=1791928607", "2026-09-10", 6, 70, 32.15, 35.73, 10, "GamesPlanet US", false, ["tir"]],
  [4231820, "Castlevania: Belmont's Curse", "4231820/a296479a772d37cfb6c36c59059b72d663abca38/library_capsule_2x.jpg?t=1792036946", "4231820/6b65aec2c398006aea8b76e1463cc34d0e4ba68b/header.jpg?t=1792036946", "4231820/a86259a3a341818b144f97400c4cc5b3db0e708e/library_hero.jpg?t=1792036946", "2026-10-15", 6, 74, 21.43, 26.79, 20, "WinGameStore", false, ["action", "rpg"]],
  [4078430, "STAR WARS: Galactic Racer™", "4078430/16514821db6559e75c9067285fee5dd198492aee/library_capsule_2x.jpg?t=1791388584", "4078430/97215eb609882139ce6a28419116be23fcf5ac86/header.jpg?t=1791388584", "4078430/541b6180ab006e79bb64ad768ad523ef0207c489/library_hero.jpg?t=1791388584", "2026-10-06", 8, 85, 46.92, 59.99, 22, "Zapagames", false, ["course"]],
  [4019220, "Dressmaker", "4019220/6182ce74d95456b557ef361a78866173dc157b58/library_capsule_2x.jpg?t=1792401675", "4019220/f1cd08127872e60e4c47a4bc3e40d682d94741bf/header.jpg?t=1792401675", "4019220/122eb9b7e28c9db5679448517302037834d70309/library_hero.jpg?t=1792401675", "2026-09-21", 9, 98, 15.47, 15.47, 0, "Steam", false, ["simulation", "independant"]],
  [2315050, "We Were Here Tomorrow", "2315050/5170fa89f16330acbda072a73c210ca9c2c7acfd/library_capsule_2x.jpg?t=1792588782", "2315050/1e74865089e86e73fe7abefec0f5b59c1ce48305/header_alt_assets_3.jpg?t=1792588782", "2315050/8ddcc85a1171169689bb6d7079994155ce0d7883/library_hero.jpg?t=1792588782", "2026-10-20", 5, 63, 17.99, 19.99, 10, "Steam", false, ["reflexion", "aventure"]],
  [4115450, "Phantom Blade Zero", "4115450/471e77854910665c03eb9a53ad06f2ac12903acd/library_capsule_2x.jpg?t=1792637475", "4115450/7f882f889630270d3d3fb93825ac6f55b55e7a6c/header.jpg?t=1792637475", "4115450/1cd52ca84a3d974c4206e9cb90231956324c5999/library_hero.jpg?t=1792637475", "2026-10-29", 0, 0, 55.19, 59.99, 8, "GreenManGaming", true, ["action", "rpg"]],
  [4354570, "FINAL FANTASY VII REVELATION", "4354570/4ef64a06f35ba298f12ea3f64421756cd5dff9b6/library_capsule_2x.jpg?t=1791260852", "4354570/e1750093047ef96e99879977810c9b4c774bab68/header.jpg?t=1791260852", "4354570/43c430b34b62edf6ca86855441c6475859657d3a/library_hero.jpg?t=1791260852", "2027-04-08", 0, 0, 69.99, 69.99, 0, "Steam", true, ["rpg"]],
  [2590240, "METRO 2039", "2590240/f5e198f382b98e13b4962f98ab7dcd8c5d2d62fa/library_capsule_2x.jpg?t=1792162386", "2590240/6998fb70f896b0f155061de0b472a8262bd06ef9/header.jpg?t=1792162386", "2590240/2c74f11f5bc793ae3568adcdbe566ac128432367/library_hero.jpg?t=1792162386", "2027-02-04", 0, 0, 53.59, 62.53, 14, "GamesPlanet US", true, ["tir", "aventure"]],
  [2272360, "Warhammer 40,000: Dawn of War IV", "2272360/c7dfb77d135e6da1b8a392af39b4272c59b1162a/library_capsule_2x.jpg?t=1792608448", "2272360/f50a309da98ef75cbdae400a7d492e36314ce415/header.jpg?t=1792608448", "2272360/63bb72eb8e7513bc4756bb46d38f046114c33087/library_hero.jpg?t=1792608448", "2026-12-03", 0, 0, 44.66, 53.59, 17, "GamesPlanet US", true, ["strategie"]],
  [2963950, "Persona 4 Revival", "2963950/056cc71104978b202c5968ffb5003045c1931c95/library_capsule_2x.jpg?t=1788989521", "2963950/e83b7da1c2e9ddbed1a27038bd86e9714b2aa893/header.jpg?t=1788989521", "2963950/75a5e87adefb79e083deace37f14769d8ae7cc5f/library_hero.jpg?t=1788989521", "2027-02-18", 0, 0, 56.27, 62.53, 10, "GamesPlanet US", true, ["rpg"]],
  [2769570, "Fable", "2769570/2ec61bc4af6cbd99d7a25ad392e66f17bb6cb0fb/library_capsule_2x.jpg?t=1788984944", "2769570/d2ffd7e15a91f3e5485601d3d876988ba3a859d7/header.jpg?t=1788984944", "2769570/931fc30f2a8f28552526b27483d3fed877452ea4/library_hero.jpg?t=1788984944", "2027-02-23", 0, 0, 64.49, 69.99, 8, "PlayerLand", true, ["rpg", "aventure"]],
  [730, "Counter-Strike 2", "730/library_600x900_2x.jpg?t=1791824795", "730/162664aa5da85f418105350c5d67ca565f6c3713/header.jpg?t=1791824795", "730/library_hero.jpg?t=1791824795", "2012-08-21", 8, 85, null, 0, 0, "", false, ["tir"]],
  [3513350, "Wuthering Waves", "3513350/07948a96389614937d331335b9dca86bd5166d7c/library_capsule_2x.jpg?t=1791864800", "3513350/fb969539fb0373bd5ee07c767bf9dafa4afae5c3/header.jpg?t=1791864800", "3513350/71dc6e3d1b733e31187f06c547576c9916f10bdb/library_hero.jpg?t=1791864800", "2025-04-28", 8, 87, null, 0, 0, "", false, ["rpg", "action"]],
  [2139460, "Once Human", "2139460/5114e6359e5cb6009cba26274734c09642c7d155/library_capsule_2x.jpg?t=1792640697", "2139460/c18b08a2f93dec32fcc5fa71bc2da44ee42de1f0/header.jpg?t=1792640697", "2139460/4b43feb4aadab1fbb9d7ffc583e4a8af12bb1d22/library_hero.jpg?t=1792640697", "2024-07-09", 6, 79, null, 0, 0, "", false, ["action"]],
  [1172470, "Apex Legends™", "1172470/f86ccfa6119d81a3072ab57886dd1843c0306f7a/library_capsule_2x.jpg?t=1790164440", "1172470/6cf04767652d703b6050da84b82cfb1194258d7d/header.jpg?t=1790164440", "1172470/8172a927a16a54bf8e8a21bce045f8304672c788/library_hero.jpg?t=1790164440", "2020-11-05", 5, 67, null, 0, 0, "", false, ["tir"]],
  [230410, "Warframe", "230410/0572533cfd26a64e831f83ef3f940e1b8012b1e8/library_capsule_2x.jpg?t=1792613117", "230410/1a3e7c2a05ce4e5cad5ac4c7e858cf5b0934704e/header.jpg?t=1792613117", "230410/eff9f1a6e0b38ff6287f1fcc57e9eacfd3be27b1/library_hero.jpg?t=1792613117", "2013-03-25", 8, 87, null, 0, 0, "", false, ["action", "tir"]],
];

function versJeu(l: Ligne): StoreShelfItem {
  const [appId, title, capsule, header, hero, sortie, score, pct, prix, normal, remise, boutique, aVenir, genres] = l;
  const gratuit = prix === null;
  return {
    appId,
    gameId: `steam-${appId}`,
    title,
    capsuleUrl: STEAM + capsule,
    headerUrl: STEAM + header,
    heroUrl: STEAM + hero,
    description: t("demo.descriptionBoutique"),
    tags: genres.map((g) => t(`boutique.accueil.genres.${g}` as Cle)),
    reviewScore: score,
    reviewPercent: score ? pct : null,
    reviewCount: score ? 1000 : 0,
    releaseDate: Date.parse(sortie) / 1000,
    releaseDisplay: "date_full",
    comingSoon: aVenir,
    isFree: gratuit,
    price: prix,
    normalPrice: gratuit ? null : normal,
    savings: remise,
    storeName: boutique,
    buyUrl: "https://store.steampowered.com/app/" + appId,
    record: false,
  };
}

const parId = (ids: number[]) =>
  ids.map((id) => JEUX.find((l) => l[0] === id)).filter((l): l is Ligne => !!l).map(versJeu);

/** [id ITAD, titre, fichier, prix, prix normal, remise, boutique] */
const OFFRES: [string, string, string, number, number, number, string][] = [
  ["018d937f-3a3b-7210-bd2d-0d1dfb1d84c0", "Red Dead Redemption 2", "?t=1787681123", 13.68, 53.59, 74, "GamesPlanet US"],
  ["018d937f-09dd-729b-88aa-5496f800b3bd", "Outer Wilds", "?t=1787450418", 9.08, 22.99, 61, "Fanatical"],
  ["018d937f-15f9-7056-8edf-2e8fa4b8f357", "No Man's Sky", "?t=1789510538", 17.84, 53.59, 67, "GamesPlanet US"],
  ["018d937f-194c-7075-b7c9-a3c1fd30c467", "Stray", "?t=1787426414", 11.05, 27.99, 61, "Fanatical"],
  ["018d937f-2b4b-72f9-ab75-bc33784a295d", "Dead Cells", "?t=1786210238", 10.52, 22.32, 53, "GamesPlanet US"],
  ["018d937e-f0ad-7094-b86c-d8fb32a1e6b4", "Detroit: Become Human", "?t=1786382836", 6.59, 39.99, 84, "IndieGala Store"],
  ["018d937f-6136-71e4-a7d4-088a32c7d518", "The Alters", "?t=1792059012", 14.06, 31.26, 55, "WinGameStore"],
  ["018d937f-700e-7161-9c8d-5423af1b7c99", "Balatro", "?t=1790695528", 10.1, 13.39, 25, "GamesPlanet US"],
];

const CADEAUX: [string, string, string, number, number][] = [
  ["019bfebf-023c-7054-8937-6e460ed3880b", "GetHigh.EXE", "?t=1790946104", 1.79, 3],
  ["0192950a-25b8-70d0-a7a4-de0539dfc10e", "Bad Cheese", "?t=1786211509", 11.69, 5],
  ["018d937f-1cee-72e4-8de7-8b8a111eea47", "Agent A: A puzzle in disguise", "?t=1790882232", 17.99, 9],
];

function offre([id, title, v, prix, normal, remise, boutique]: (typeof OFFRES)[number]): StoreItem {
  return {
    gameId: id,
    title,
    coverUrl: `${ITAD}${id}/boxart.jpg${v}`,
    bannerUrl: `${ITAD}${id}/banner600.jpg${v}`,
    price: prix,
    normalPrice: normal,
    savings: remise,
    storeName: boutique,
    buyUrl: `https://isthereanydeal.com/`,
  };
}

/** Pour la fiche produit de démo : un jeu d'un rayon, retrouvé par son id. */
export function jeuDeDemo(gameId: string): StoreItem | null {
  const steam = JEUX.find((l) => `steam-${l[0]}` === gameId);
  if (steam) {
    const j = versJeu(steam);
    return {
      gameId,
      title: j.title,
      coverUrl: j.capsuleUrl,
      price: j.price ?? 0,
      normalPrice: j.normalPrice ?? 0,
      savings: j.savings,
      storeName: j.storeName,
      buyUrl: j.buyUrl,
    };
  }
  const o = [...OFFRES].find((x) => x[0] === gameId);
  return o ? offre(o) : null;
}

export function accueilDeDemo(): StoreHome {
  const jour = 86_400_000;
  return {
    // Même règle que Rust : promo d'au moins 20 % et au moins 75 % d'avis positifs.
    featured: parId([1091500, 1174180, 381210, 2929250, 4078430]),
    topSellers: parId([4435490, 3219030, 4080220, 1203620, 2807960, 2929250, 1867240, 4231820, 4078430, 4115450]),
    deals: OFFRES.map(offre),
    giveaways: CADEAUX.map(([id, title, v, normal, jours]) => ({
      gameId: id,
      title,
      coverUrl: `${ITAD}${id}/boxart.jpg${v}`,
      bannerUrl: `${ITAD}${id}/banner600.jpg${v}`,
      price: 0,
      normalPrice: normal,
      savings: 100,
      storeName: "Epic Game Store",
      buyUrl: "https://store.epicgames.com/",
      expiry: new Date(Date.now() + jours * jour).toISOString(),
    })),
  };
}

/** Correspondance tag Steam → clé de genre de la démo. */
const TAG_GENRE: Record<number, CleGenre> = {
  19: "action", 21: "aventure", 122: "rpg", 9: "strategie", 599: "simulation",
  1774: "tir", 699: "course", 701: "sport", 1664: "reflexion", 492: "independant",
};

export function rayonDeDemo(kind: StoreShelfKind, tag?: number): StoreShelfItem[] {
  switch (kind) {
    case "topSellers":
      return accueilDeDemo().topSellers;
    case "newReleases":
      return parId([1203620, 4231820, 4019220, 2315050, 3219030, 4078430, 2929250]);
    case "upcoming":
      return parId([4115450, 2272360, 2590240, 2963950, 2769570, 4354570]);
    case "freeToPlay":
      return parId([730, 3513350, 2139460, 1172470, 230410]);
    case "genre": {
      const genre = tag !== undefined ? TAG_GENRE[tag] : undefined;
      return JEUX.filter((l) => l[8] !== null && !l[12] && genre && l[13].includes(genre)).map(versJeu);
    }
  }
}

/** Le menu « Catégories » de la démo : les genres du relevé, sous leur vrai tag Steam. */
export function tagsDeDemo(): StoreTag[] {
  return Object.entries(TAG_GENRE).map(([id, g]) => ({ id: Number(id), nom: t(`boutique.accueil.genres.${g}` as Cle) }));
}

/** Recherche par catégories de la démo, sur les jeux du relevé (déjà sortis), par pages de 24. */
export function parcoursDeDemo(tags: number[], tri: StoreBrowseSort, page: number): StoreBrowse {
  const genres = tags.map((id) => TAG_GENRE[id]);
  const lignes = JEUX.filter((l) => !l[12] && genres.every((g) => g && l[13].includes(g)));
  if (tri === "rating") lignes.sort((a, b) => b[7] - a[7]);
  if (tri === "trending") lignes.sort((a, b) => b[5].localeCompare(a[5]));
  const jeux = lignes.map(versJeu);
  return { total: jeux.length, jeux: jeux.slice(page * 24, page * 24 + 24) };
}
