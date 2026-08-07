import type { Platform, PlatformId } from "../types";

export const PLATFORMS: Record<PlatformId, Platform> = {
  steam: { id: "steam", name: "Steam", color: "var(--steam)" },
  epic: { id: "epic", name: "Epic Games", color: "var(--epic)" },
  gog: { id: "gog", name: "GOG", color: "var(--gog)" },
  riot: { id: "riot", name: "Riot Games", color: "var(--riot)" },
  ubisoft: { id: "ubisoft", name: "Ubisoft Connect", color: "var(--ubisoft)" },
  ea: { id: "ea", name: "EA", color: "var(--ea)" },
  battlenet: { id: "battlenet", name: "Battle.net", color: "var(--battlenet)" },
  manual: { id: "manual", name: "Manuel", color: "var(--manual)" },
};

export function platformName(id: PlatformId): string {
  return PLATFORMS[id].name;
}
