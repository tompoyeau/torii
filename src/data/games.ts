import type { Game, GameDto, GameSource, PlatformId } from "../types";
import { gradientFor, relativeTime } from "../lib/covers";
import { cachedLibrary, displayableCover, scanLibrary } from "../lib/tauri";

/** Clé de rapprochement d'un titre : minuscules, alphanumérique seul (retire
 * ™®, ponctuation, espaces). Strict → évite les fusions abusives. */
function titleKey(title: string): string {
  return title.toLowerCase().replace(/[^a-z0-9]/g, "");
}

const PLATFORM_ORDER: Record<PlatformId, number> = { steam: 0, gog: 1, epic: 2, riot: 3, ubisoft: 4, ea: 5, battlenet: 6, manual: 7, detected: 8 };

/**
 * Fusionne les doublons cross-plateforme : un même jeu possédé sur plusieurs
 * launchers devient une carte unique portant plusieurs `sources` (choix du
 * launcher au lancement). On ne fusionne que des titres identiques (clé stricte)
 * répartis sur ≥2 plateformes distinctes.
 */
export function mergeDuplicates(games: Game[]): Game[] {
  const groups = new Map<string, Game[]>();
  for (const g of games) {
    const key = titleKey(g.title);
    // Titres trop courts/vides : jamais regroupés (risque de collision).
    const bucket = key.length >= 3 ? key : `solo:${g.id}`;
    const arr = groups.get(bucket);
    if (arr) arr.push(g);
    else groups.set(bucket, [g]);
  }

  const out: Game[] = [];
  for (const group of groups.values()) {
    const platforms = new Set(group.map((g) => g.platform));
    if (group.length < 2 || platforms.size < 2) {
      out.push(...group);
      continue;
    }
    // Primaire (affichage) : installé d'abord, puis avec jaquette, puis ordre plateforme.
    const primary = [...group].sort(
      (a, b) =>
        Number(b.installed) - Number(a.installed) ||
        Number(!!b.coverUrl) - Number(!!a.coverUrl) ||
        PLATFORM_ORDER[a.platform] - PLATFORM_ORDER[b.platform],
    )[0];
    const sources: GameSource[] = group.map((g) => ({
      platform: g.platform,
      launchTarget: g.launchTarget,
      installed: g.installed,
    }));
    const installedSrc = group.find((g) => g.installed);
    const maxHours = Math.max(0, ...group.map((g) => g.hoursPlayed ?? 0));
    const lastAt = Math.max(0, ...group.map((g) => g.lastPlayedAt ?? 0));
    out.push({
      ...primary,
      installed: group.some((g) => g.installed),
      hoursPlayed: maxHours > 0 ? maxHours : undefined,
      lastPlayedAt: lastAt > 0 ? lastAt : undefined,
      lastPlayed: lastAt > 0 ? relativeTime(lastAt) : primary.lastPlayed,
      recent: group.some((g) => g.recent),
      favorite: group.some((g) => g.favorite),
      sizeGb: installedSrc?.sizeGb ?? primary.sizeGb,
      // Copies famille : portées par la source Steam, quel que soit le primaire affiché.
      familyOwners: group.find((g) => g.familyOwners?.length)?.familyOwners,
      sources,
    });
  }

  out.sort((a, b) => a.title.toLowerCase().localeCompare(b.title.toLowerCase()));
  return out;
}

const DESCRIPTION =
  "Une expérience marquante saluée par la critique. Explore un monde façonné à la main, affine ton style de jeu au fil des heures et laisse la bande-son t'emporter. Chaque session te rapproche de la fin — ou d'un nouveau départ.";

/**
 * Bibliothèque de démonstration : de VRAIS jeux, une fausse bibliothèque.
 *
 * 🔑 Elle sert deux publics à la fois. En développement (`npm run dev`), elle permet de
 * travailler l'interface sans lancer l'application complète. Et en ligne, c'est la
 * **démo jouable** publiée sur le site : un visiteur essaie Torii dans son navigateur
 * avant de télécharger quoi que ce soit.
 *
 * D'où de vrais titres et de vraies jaquettes (CDN Steam, servi publiquement) plutôt que
 * des noms inventés sur des dégradés : une grille de jeux qu'on reconnaît donne une idée
 * de ce que sera SA bibliothèque, ce qu'une liste de titres fictifs ne fait pas.
 *
 * ⚠️ Les heures de jeu, les succès et les dates sont, eux, entièrement inventés.
 */
const MOCK_GAMES: Game[] = [
  { id: "bg3", title: "Baldur's Gate 3", platform: "steam", genre: "Jeu de rôle", cover: "linear-gradient(150deg,#3a1010,#c25b2a)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1086940/library_600x900.jpg", launchTarget: "1086940", hoursPlayed: 254, lastPlayed: "hier", developer: "Larian Studios", year: 2023, sizeGb: 150, achievements: { unlocked: 39, total: 54 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "elden-ring", title: "ELDEN RING", platform: "steam", genre: "Action-RPG", cover: "linear-gradient(150deg,#12005e,#9d50ff)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1245620/library_600x900.jpg", launchTarget: "1245620", hoursPlayed: 187, lastPlayed: "il y a 2 j", developer: "FromSoftware", year: 2022, sizeGb: 60, achievements: { unlocked: 28, total: 42 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "hades2", title: "Hades II", platform: "steam", genre: "Rogue-lite", cover: "linear-gradient(150deg,#0b3d5c,#12c2b0)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1145350/library_600x900.jpg", launchTarget: "1145350", hoursPlayed: 46, lastPlayed: "aujourd'hui", developer: "Supergiant Games", year: 2024, sizeGb: 20, achievements: { unlocked: 17, total: 49 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "cyberpunk", title: "Cyberpunk 2077", platform: "gog", genre: "Action-RPG", cover: "linear-gradient(150deg,#2b1055,#7597de)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1091500/library_600x900.jpg", hoursPlayed: 92, lastPlayed: "il y a 5 j", developer: "CD PROJEKT RED", year: 2020, sizeGb: 70, achievements: { unlocked: 31, total: 57 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "witcher3", title: "The Witcher 3: Wild Hunt", platform: "gog", genre: "Jeu de rôle", cover: "linear-gradient(150deg,#134e3a,#a8e063)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/292030/library_600x900.jpg", hoursPlayed: 213, lastPlayed: "il y a 3 sem", developer: "CD PROJEKT RED", year: 2015, sizeGb: 50, achievements: { unlocked: 52, total: 78 }, installed: true, favorite: false, recent: false, description: DESCRIPTION },
  { id: "hollow-knight", title: "Hollow Knight", platform: "steam", genre: "Metroidvania", cover: "linear-gradient(150deg,#1a1a2e,#e94057)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/367520/library_600x900.jpg", launchTarget: "367520", hoursPlayed: 63, lastPlayed: "il y a 4 j", developer: "Team Cherry", year: 2017, sizeGb: 9, achievements: { unlocked: 38, total: 63 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "stardew", title: "Stardew Valley", platform: "steam", genre: "Simulation", cover: "linear-gradient(150deg,#232526,#8e9eab)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/413150/library_600x900.jpg", launchTarget: "413150", hoursPlayed: 118, lastPlayed: "il y a 6 j", developer: "ConcernedApe", year: 2016, sizeGb: 1, achievements: { unlocked: 29, total: 40 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "rdr2", title: "Red Dead Redemption 2", platform: "epic", genre: "Aventure", cover: "linear-gradient(150deg,#3d1c00,#d38312)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1174180/library_600x900.jpg", hoursPlayed: 74, lastPlayed: "il y a 1 sem", developer: "Rockstar Games", year: 2019, sizeGb: 120, achievements: { unlocked: 22, total: 51 }, installed: false, favorite: false, recent: false, description: DESCRIPTION },
  { id: "helldivers", title: "HELLDIVERS 2", platform: "steam", genre: "Tir", cover: "linear-gradient(150deg,#3a1010,#c25b2a)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/553850/library_600x900.jpg", launchTarget: "553850", hoursPlayed: 58, lastPlayed: "aujourd'hui", developer: "Arrowhead", year: 2024, sizeGb: 140, achievements: { unlocked: 24, total: 38 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "deeprock", title: "Deep Rock Galactic", platform: "steam", genre: "Coopératif", cover: "linear-gradient(150deg,#12005e,#9d50ff)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/548430/library_600x900.jpg", launchTarget: "548430", hoursPlayed: 96, lastPlayed: "il y a 2 j", developer: "Ghost Ship Games", year: 2020, sizeGb: 17, achievements: { unlocked: 44, total: 89 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "factorio", title: "Factorio", platform: "steam", genre: "Gestion", cover: "linear-gradient(150deg,#0b3d5c,#12c2b0)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/427520/library_600x900.jpg", launchTarget: "427520", hoursPlayed: 141, lastPlayed: "il y a 9 j", developer: "Wube Software", year: 2020, sizeGb: 3, achievements: { unlocked: 18, total: 38 }, installed: true, favorite: true, recent: false, description: DESCRIPTION },
  { id: "rimworld", title: "RimWorld", platform: "steam", genre: "Simulation", cover: "linear-gradient(150deg,#2b1055,#7597de)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/294100/library_600x900.jpg", launchTarget: "294100", hoursPlayed: 88, lastPlayed: "il y a 12 j", developer: "Ludeon Studios", year: 2018, sizeGb: 1, installed: true, favorite: false, recent: false, description: DESCRIPTION },
  { id: "disco", title: "Disco Elysium", platform: "gog", genre: "Jeu de rôle", cover: "linear-gradient(150deg,#134e3a,#a8e063)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/632470/library_600x900.jpg", hoursPlayed: 41, lastPlayed: "il y a 3 sem", developer: "ZA/UM", year: 2019, sizeGb: 20, achievements: { unlocked: 9, total: 54 }, installed: false, favorite: true, recent: false, description: DESCRIPTION },
  { id: "celeste", title: "Celeste", platform: "epic", genre: "Plateforme", cover: "linear-gradient(150deg,#1a1a2e,#e94057)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/504230/library_600x900.jpg", hoursPlayed: 27, lastPlayed: "il y a 5 j", developer: "Maddy Makes Games", year: 2018, sizeGb: 2, achievements: { unlocked: 14, total: 35 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "slay-spire", title: "Slay the Spire", platform: "steam", genre: "Deck-building", cover: "linear-gradient(150deg,#232526,#8e9eab)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/646570/library_600x900.jpg", launchTarget: "646570", hoursPlayed: 72, lastPlayed: "il y a 4 j", developer: "Mega Crit", year: 2019, sizeGb: 1, achievements: { unlocked: 26, total: 45 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "terraria", title: "Terraria", platform: "steam", genre: "Bac à sable", cover: "linear-gradient(150deg,#3d1c00,#d38312)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/105600/library_600x900.jpg", launchTarget: "105600", hoursPlayed: 205, lastPlayed: "il y a 1 mois", developer: "Re-Logic", year: 2011, sizeGb: 1, achievements: { unlocked: 61, total: 115 }, installed: true, favorite: false, recent: false, description: DESCRIPTION },
  { id: "dave-diver", title: "DAVE THE DIVER", platform: "steam", genre: "Aventure", cover: "linear-gradient(150deg,#3a1010,#c25b2a)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1868140/library_600x900.jpg", launchTarget: "1868140", hoursPlayed: 33, lastPlayed: "il y a 8 j", developer: "MINTROCKET", year: 2023, sizeGb: 4, achievements: { unlocked: 21, total: 44 }, installed: true, favorite: true, recent: false, description: DESCRIPTION },
  { id: "vampire-surv", title: "Vampire Survivors", platform: "epic", genre: "Rogue-lite", cover: "linear-gradient(150deg,#12005e,#9d50ff)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1794680/library_600x900.jpg", hoursPlayed: 54, lastPlayed: "hier", developer: "poncle", year: 2022, sizeGb: 1, achievements: { unlocked: 86, total: 201 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "sekiro", title: "Sekiro: Shadows Die Twice", platform: "steam", genre: "Action", cover: "linear-gradient(150deg,#0b3d5c,#12c2b0)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/814380/library_600x900.jpg", launchTarget: "814380", hoursPlayed: 49, lastPlayed: "il y a 2 sem", developer: "FromSoftware", year: 2019, sizeGb: 25, achievements: { unlocked: 17, total: 34 }, installed: false, favorite: false, recent: false, description: DESCRIPTION },
  { id: "valorant", title: "VALORANT", platform: "riot", genre: "Tir", cover: "linear-gradient(150deg,#2b1055,#7597de)", hoursPlayed: 312, lastPlayed: "il y a 2 h", developer: "Riot Games", year: 2020, sizeGb: 30, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "portal2", title: "Portal 2", platform: "steam", genre: "Réflexion", cover: "linear-gradient(150deg,#134e3a,#a8e063)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/620/library_600x900.jpg", launchTarget: "620", hoursPlayed: 19, lastPlayed: "il y a 1 mois", developer: "Valve", year: 2011, sizeGb: 13, achievements: { unlocked: 38, total: 51 }, installed: true, favorite: true, recent: false, description: DESCRIPTION },
  { id: "lethal", title: "Lethal Company", platform: "steam", genre: "Horreur", cover: "linear-gradient(150deg,#1a1a2e,#e94057)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1966720/library_600x900.jpg", launchTarget: "1966720", hoursPlayed: 26, lastPlayed: "il y a 6 j", developer: "Zeekerss", year: 2023, sizeGb: 2, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "detected:minecraft", title: "Minecraft", platform: "detected", genre: "Bac à sable", cover: "linear-gradient(150deg,#232526,#8e9eab)", hoursPlayed: 167, lastPlayed: "aujourd'hui", developer: "Mojang Studios", year: 2011, sizeGb: 2, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "ac-valhalla", title: "Assassin's Creed Valhalla", platform: "ubisoft", genre: "Action-Aventure", cover: "linear-gradient(150deg,#12005e,#9d50ff)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/2208920/library_600x900.jpg", hoursPlayed: 67, lastPlayed: "il y a 2 sem", developer: "Ubisoft Montreal", year: 2020, sizeGb: 110, installed: true, favorite: false, recent: false, description: DESCRIPTION },
  { id: "diablo4", title: "Diablo IV", platform: "battlenet", genre: "Action-RPG", cover: "linear-gradient(150deg,#3a1010,#c25b2a)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/2344520/library_600x900.jpg", hoursPlayed: 83, lastPlayed: "il y a 3 j", developer: "Blizzard Entertainment", year: 2023, sizeGb: 90, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "sims4", title: "Les Sims 4", platform: "ea", genre: "Simulation", cover: "linear-gradient(150deg,#3d1c00,#d38312)", coverUrl: "https://cdn.cloudflare.steamstatic.com/steam/apps/1222670/library_600x900.jpg", hoursPlayed: 61, lastPlayed: "il y a 10 j", developer: "Maxis", year: 2014, sizeGb: 35, installed: true, favorite: false, recent: false, description: DESCRIPTION },
];

const RECENT_WINDOW_DAYS = 21;

/** Convertit un jeu brut (scan Rust) en modèle d'affichage. */
export function fromDto(dto: GameDto): Game {
  const lastPlayed = dto.lastPlayed ?? null;
  const minutes = dto.playtimeMinutes ?? 0;
  return {
    id: dto.id,
    title: dto.title,
    platform: dto.platform,
    cover: gradientFor(dto.id),
    // Une jaquette de jeu manuel peut être un fichier local choisi par l'utilisateur :
    // la webview a besoin d'une URL `asset://` pour l'afficher.
    coverUrl: dto.coverUrl ? displayableCover(dto.coverUrl) : undefined,
    heroUrl: dto.heroUrl ?? undefined,
    installed: dto.installed,
    owned: dto.owned ?? dto.installed,
    familyShared: dto.familyShared ?? false,
    familyOwners: dto.familyOwners?.length ? dto.familyOwners : undefined,
    favorite: dto.favorite ?? false,
    recent: lastPlayed
      ? Date.now() / 1000 - lastPlayed < RECENT_WINDOW_DAYS * 86400
      : false,
    launchTarget: dto.launchTarget,
    installDir: dto.installDir ?? undefined,
    sizeGb: dto.sizeGb,
    // Temps de jeu (Steam) : masqué en dessous d'une heure pour éviter les « 0 h ».
    hoursPlayed: minutes >= 60 ? Math.round(minutes / 60) : undefined,
    lastPlayed: lastPlayed ? relativeTime(lastPlayed) : undefined,
    lastPlayedAt: lastPlayed ?? undefined,
    genre: dto.genre ?? undefined,
    description: dto.description ?? undefined,
    developer: dto.developer ?? undefined,
    year: dto.year ?? undefined,
    screenshots: dto.screenshots?.length ? dto.screenshots : undefined,
    hidden: dto.hidden ?? false,
  };
}

/**
 * Source unique des jeux.
 * - Sous Tauri : commande Rust `scan_library` (Steam / Epic / GOG / manuel).
 * - Hors Tauri (navigateur, `npm run dev`) : données fictives.
 */
export async function fetchGames(): Promise<Game[]> {
  const dtos = await scanLibrary();
  return dtos ? dtos.map(fromDto) : MOCK_GAMES;
}

/**
 * Bibliothèque du dernier scan (cache disque) : rendue immédiatement au lancement,
 * le temps que `fetchGames()` termine. Vide au premier lancement et hors Tauri.
 */
export async function fetchCachedGames(): Promise<Game[]> {
  return (await cachedLibrary()).map(fromDto);
}
