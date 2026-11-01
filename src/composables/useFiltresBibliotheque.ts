import { computed } from "vue";
import { useLibrary } from "./useLibrary";
import { estHorsLauncher, platformName } from "../data/platforms";
import { MODE_WEB } from "../lib/mode";
import type { LibraryFilter, PlatformId } from "../types";

/**
 * Les filtres de la bibliothèque et leurs compteurs : la barre latérale les liste, et sur
 * téléphone, où elle disparaît, une rangée de puces prend le relais (`FiltresMobile`).
 */
export function useFiltresBibliotheque() {
  const { games } = useLibrary();

  // Les compteurs des vues normales excluent les jeux masqués.
  const visible = computed(() => games.value.filter((g) => !g.hidden));
  const total = computed(() => visible.value.length);
  const count = (f: LibraryFilter) =>
    computed(() => {
      switch (f) {
        case "all": return visible.value.length;
        case "mine": return visible.value.filter((g) => !g.familyShared).length;
        case "family": return visible.value.filter((g) => g.familyShared).length;
        case "recent": return visible.value.filter((g) => g.recent).length;
        case "favorite": return visible.value.filter((g) => g.favorite).length;
        case "installed": return visible.value.filter((g) => g.installed).length;
        case "hidden": return games.value.filter((g) => g.hidden).length;
        case "horsLauncher":
          return visible.value.filter((g) => estHorsLauncher(g.platform)).length;
        default: return visible.value.filter((g) =>
          g.sources ? g.sources.some((s) => s.platform === f) : g.platform === f,
        ).length;
      }
    });

  /**
   * Les entrees de la liste « Plateformes ». `id` est le filtre applique, `icon` la
   * plateforme dont on emprunte l icone : les deux coincident partout SAUF pour
   * « Hors launcher », qui est une categorie regroupant deux plateformes et n en est
   * donc pas une (cf. `HORS_LAUNCHER`).
   */
  const platforms = computed<{ id: LibraryFilter; icon: PlatformId; label: string }[]>(() => [
    { id: "steam", icon: "steam", label: platformName("steam") },
    { id: "epic", icon: "epic", label: platformName("epic") },
    { id: "gog", icon: "gog", label: platformName("gog") },
    { id: "riot", icon: "riot", label: platformName("riot") },
    { id: "ubisoft", icon: "ubisoft", label: platformName("ubisoft") },
    { id: "ea", icon: "ea", label: platformName("ea") },
    { id: "battlenet", icon: "battlenet", label: platformName("battlenet") },
    { id: "horsLauncher", icon: "manual", label: platformName("manual") },
  ].filter((p) => !MODE_WEB || count(p.id as LibraryFilter).value > 0) as { id: LibraryFilter; icon: PlatformId; label: string }[]);

  return { total, count, platforms };
}
