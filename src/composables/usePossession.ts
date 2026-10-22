import { computed } from "vue";
import { useLibrary } from "./useLibrary";

/** Le jeu est dans la bibliothèque : acheté (`possede`) ou prêté par la famille Steam. */
export type Possession = "possede" | "famille" | null;

/**
 * Titre réduit à ses lettres et chiffres, sans accents ni symboles de marque :
 * « Call of Duty®: Modern Warfare® 4 » → « callofdutymodernwarfare4 ».
 */
export function normaliserTitre(titre: string): string {
  return titre
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

/**
 * Les jeux de la Boutique déjà présents dans la bibliothèque — calculé sur le PC, à
 * partir de la bibliothèque déjà chargée : aucun appel, rien qui sorte de la machine.
 *
 * 🔑 DEUX CLÉS. Les rayons choisis par Steam portent un appid : la correspondance est
 * exacte. Les offres du comparateur (promos, jeux offerts, toutes les offres) n'en ont
 * pas : on compare les titres normalisés, ce qui reconnaît aussi un jeu possédé sur GOG
 * ou Epic.
 *
 * ⚠️ « Partage familial » n'est pas une possession (cf. ARCHITECTURE.md, « accès n'est
 * pas possession ») : le jeu est jouable, mais il disparaît si le proche quitte la
 * famille. La carte le dit, plutôt que d'affirmer « dans ta bibliothèque ».
 */
export function usePossession() {
  const { games } = useLibrary();

  const index = computed(() => {
    const parAppid = new Map<number, Possession>();
    const parTitre = new Map<string, Possession>();
    for (const g of games.value) {
      const etat: Possession = g.familyShared ? "famille" : "possede";
      const ids = [g.id, ...(g.sources ?? []).map((s) => s.id ?? "")];
      for (const id of ids) {
        if (!id.startsWith("steam:")) continue;
        const appid = Number(id.slice("steam:".length));
        if (Number.isFinite(appid) && parAppid.get(appid) !== "possede") parAppid.set(appid, etat);
      }
      const cle = normaliserTitre(g.title);
      if (cle && parTitre.get(cle) !== "possede") parTitre.set(cle, etat);
    }
    return { parAppid, parTitre };
  });

  /** Possession d'un jeu de la Boutique, par appid si on l'a, sinon par titre. */
  function possession(titre: string, appId?: number | null): Possession {
    const { parAppid, parTitre } = index.value;
    if (appId && parAppid.has(appId)) return parAppid.get(appId) ?? null;
    return parTitre.get(normaliserTitre(titre)) ?? null;
  }

  return { possession };
}
