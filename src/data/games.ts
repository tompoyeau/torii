import type { Game, GameDto, GameSource, PlatformId } from "../types";
import { gradientFor, relativeTime } from "../lib/covers";

/** Clé de rapprochement d'un titre : minuscules, alphanumérique seul (retire
 * ™®, ponctuation, espaces). Strict → évite les fusions abusives. */
function titleKey(title: string): string {
  return title.toLowerCase().replace(/[^a-z0-9]/g, "");
}

const PLATFORM_ORDER: Record<PlatformId, number> = { steam: 0, gog: 1, epic: 2, riot: 3, ubisoft: 4, ea: 5, battlenet: 6, manual: 7 };

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
 * Jeu de données fictif servant de base à la maquette.
 * À terme, `fetchGames()` appellera une commande Tauri (`scan_library`)
 * qui agrège les vraies bibliothèques Steam / Epic / GOG / manuelles.
 */
const MOCK_GAMES: Game[] = [
  { id: "neon-requiem", title: "Neon Requiem", platform: "steam", genre: "Action-RPG", cover: "radial-gradient(120% 140% at 82% 18%,#ff7a4d,#b0288a 40%,#3a1c6e 72%,#14102a)", hoursPlayed: 47, lastPlayed: "il y a 2 h", developer: "Vireo Studio", year: 2024, sizeGb: 62, achievements: { unlocked: 31, total: 48 }, installed: true, installDir: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Neon Requiem", favorite: true, recent: true, description: DESCRIPTION },
  { id: "hollow-verge", title: "Hollow Verge", platform: "gog", genre: "Metroidvania", cover: "linear-gradient(150deg,#2b1055,#7597de)", hoursPlayed: 23, lastPlayed: "hier", developer: "Pale Lantern", year: 2023, sizeGb: 8, achievements: { unlocked: 22, total: 34 }, installed: true, installDir: "D:\\GOG Games\\Hollow Verge", favorite: true, recent: true, description: DESCRIPTION },
  { id: "starbound-drifter", title: "Starbound Drifter", platform: "epic", genre: "Aventure", cover: "linear-gradient(150deg,#0b3d5c,#12c2b0)", hoursPlayed: 8, lastPlayed: "il y a 3 j", developer: "Cloudreach", year: 2025, sizeGb: 14, achievements: { unlocked: 6, total: 40 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "ashen-kingdoms", title: "Ashen Kingdoms", platform: "steam", genre: "RPG", cover: "linear-gradient(150deg,#3a1010,#c25b2a)", hoursPlayed: 112, lastPlayed: "il y a 5 j", developer: "Ironquill", year: 2022, sizeGb: 88, launchTarget: "1086940", achievements: { unlocked: 41, total: 52 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "velocity-protocol", title: "Velocity Protocol", platform: "epic", genre: "Course", cover: "linear-gradient(150deg,#141e30,#ff5f6d)", hoursPlayed: 14, lastPlayed: "il y a 6 j", developer: "Apex Loop", year: 2024, sizeGb: 36, achievements: { unlocked: 12, total: 30 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "garden-of-echoes", title: "Garden of Echoes", platform: "manual", genre: "Puzzle", cover: "linear-gradient(150deg,#134e3a,#a8e063)", hoursPlayed: 5, lastPlayed: "il y a 1 sem", developer: "Miru Games", year: 2023, sizeGb: 3, achievements: { unlocked: 9, total: 24 }, installed: true, favorite: true, recent: false, description: DESCRIPTION },
  { id: "ironclad-tactics", title: "Ironclad Tactics", platform: "gog", genre: "Stratégie", cover: "linear-gradient(150deg,#232526,#8e9eab)", hoursPlayed: 41, lastPlayed: "il y a 2 j", developer: "Ninefold", year: 2021, sizeGb: 11, achievements: { unlocked: 18, total: 45 }, installed: false, favorite: false, recent: true, description: DESCRIPTION },
  { id: "lumen", title: "Lumen", platform: "steam", genre: "Plateforme", cover: "linear-gradient(150deg,#40128b,#e94bd0)", hoursPlayed: 19, lastPlayed: "aujourd'hui", developer: "Softglow", year: 2025, sizeGb: 6, achievements: { unlocked: 15, total: 28 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "nightfall-express", title: "Nightfall Express", platform: "epic", genre: "Horreur", cover: "linear-gradient(150deg,#1a1a2e,#e94057)", hoursPlayed: 6, lastPlayed: "il y a 4 j", developer: "Dim Corridor", year: 2024, sizeGb: 22, achievements: { unlocked: 4, total: 20 }, installed: false, favorite: false, recent: true, description: DESCRIPTION },
  { id: "cobalt-deep", title: "Cobalt Deep", platform: "steam", genre: "Survie", cover: "linear-gradient(150deg,#0f2027,#2c9fff)", hoursPlayed: 37, lastPlayed: "il y a 3 j", developer: "Abyssal", year: 2023, sizeGb: 44, achievements: { unlocked: 20, total: 50 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "paper-districts", title: "Paper Districts", platform: "gog", genre: "Narratif", cover: "linear-gradient(150deg,#614385,#ff9a8b)", hoursPlayed: 11, lastPlayed: "il y a 1 sem", developer: "Foldwork", year: 2022, sizeGb: 5, achievements: { unlocked: 14, total: 18 }, installed: false, favorite: true, recent: false, description: DESCRIPTION },
  { id: "solstice", title: "Solstice", platform: "manual", genre: "Exploration", cover: "linear-gradient(150deg,#42275a,#ffb75e)", hoursPlayed: 2, lastPlayed: "il y a 2 sem", developer: "Northlight", year: 2025, sizeGb: 9, achievements: { unlocked: 2, total: 16 }, installed: true, favorite: false, recent: false, description: DESCRIPTION },
  { id: "chrono-static", title: "Chrono Static", platform: "steam", genre: "Puzzle", cover: "linear-gradient(150deg,#093028,#41c7af)", hoursPlayed: 28, lastPlayed: "hier", developer: "Loophole", year: 2023, sizeGb: 7, achievements: { unlocked: 24, total: 36 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "bastion-fall", title: "Bastion Fall", platform: "epic", genre: "Roguelite", cover: "linear-gradient(150deg,#3e1e2f,#ff8c42)", hoursPlayed: 52, lastPlayed: "il y a 2 j", developer: "Emberforge", year: 2024, sizeGb: 18, achievements: { unlocked: 33, total: 60 }, installed: false, favorite: true, recent: true, description: DESCRIPTION },
  { id: "tidebreak", title: "Tidebreak", platform: "steam", genre: "Aventure", cover: "linear-gradient(150deg,#0b486b,#5fd6ff)", hoursPlayed: 16, lastPlayed: "il y a 3 j", developer: "Saltwind", year: 2025, sizeGb: 25, achievements: { unlocked: 11, total: 32 }, installed: true, favorite: false, recent: true, description: DESCRIPTION },
  { id: "emberwild", title: "Emberwild", platform: "gog", genre: "RPG", cover: "linear-gradient(150deg,#4b1010,#ffb347)", hoursPlayed: 64, lastPlayed: "il y a 1 sem", developer: "Ironquill", year: 2020, sizeGb: 40, achievements: { unlocked: 29, total: 55 }, installed: false, favorite: false, recent: false, description: DESCRIPTION },
  { id: "signal-lost", title: "Signal Lost", platform: "manual", genre: "Thriller", cover: "linear-gradient(150deg,#1f1c2c,#928dab)", hoursPlayed: 9, lastPlayed: "il y a 5 j", developer: "Static Room", year: 2023, sizeGb: 12, achievements: { unlocked: 7, total: 22 }, installed: true, favorite: false, recent: false, description: DESCRIPTION },
  { id: "aurora-drift", title: "Aurora Drift", platform: "steam", genre: "Ambient", cover: "linear-gradient(150deg,#12005e,#9d50ff)", hoursPlayed: 33, lastPlayed: "aujourd'hui", developer: "Polar Bloom", year: 2024, sizeGb: 15, achievements: { unlocked: 19, total: 26 }, installed: true, favorite: true, recent: true, description: DESCRIPTION },
  { id: "rustbound", title: "Rustbound", platform: "epic", genre: "Craft", cover: "linear-gradient(150deg,#3d1c00,#d38312)", hoursPlayed: 47, lastPlayed: "il y a 4 j", developer: "Cogwork", year: 2022, sizeGb: 34, achievements: { unlocked: 26, total: 48 }, installed: false, favorite: false, recent: false, description: DESCRIPTION },
];

/** Jeux mis en avant dans le carrousel du mode Salon. */
export const SPOTLIGHT_IDS = ["neon-requiem", "ashen-kingdoms", "bastion-fall", "aurora-drift"];

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
    coverUrl: dto.coverUrl ?? undefined,
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

async function invokeGames(command: string): Promise<Game[]> {
  const { invoke } = await import("@tauri-apps/api/core");
  const dtos = await invoke<GameDto[]>(command);
  return dtos.map(fromDto);
}

/**
 * Source unique des jeux.
 * - Sous Tauri : commande Rust `scan_library` (Steam / Epic / GOG / manuel).
 * - Hors Tauri (navigateur, `npm run dev`) : données fictives.
 */
export async function fetchGames(): Promise<Game[]> {
  try {
    return await invokeGames("scan_library");
  } catch {
    return MOCK_GAMES;
  }
}

/**
 * Version enrichie (genre, jaquettes, captures…) via l'API Steam Store.
 * Plus lente : à appeler en arrière-plan après `fetchGames`.
 * Renvoie `null` hors Tauri (aucun enrichissement à faire).
 */
export async function enrichGames(): Promise<Game[] | null> {
  try {
    return await invokeGames("enrich_metadata");
  } catch {
    return null;
  }
}
