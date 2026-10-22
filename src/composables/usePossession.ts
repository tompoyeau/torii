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

  /** Ce qu'on retient d'un jeu possédé : son état et sa carte dans la bibliothèque. */
  interface Trouve {
    etat: Exclude<Possession, null>;
    id: string;
  }

  const index = computed(() => {
    const parAppid = new Map<number, Trouve>();
    const parTitre = new Map<string, Trouve>();
    // Un jeu possédé l'emporte sur le même jeu prêté par la famille.
    const garder = <K>(carte: Map<K, Trouve>, cle: K, t: Trouve) => {
      if (carte.get(cle)?.etat !== "possede") carte.set(cle, t);
    };
    for (const g of games.value) {
      const t: Trouve = { etat: g.familyShared ? "famille" : "possede", id: g.id };
      const ids = [g.id, ...(g.sources ?? []).map((s) => s.id ?? "")];
      for (const id of ids) {
        if (!id.startsWith("steam:")) continue;
        const appid = Number(id.slice("steam:".length));
        if (Number.isFinite(appid)) garder(parAppid, appid, t);
      }
      const cle = normaliserTitre(g.title);
      if (cle) garder(parTitre, cle, t);
    }
    return { parAppid, parTitre };
  });

  function trouver(titre: string, appId?: number | null): Trouve | null {
    const { parAppid, parTitre } = index.value;
    if (appId && parAppid.has(appId)) return parAppid.get(appId) ?? null;
    return parTitre.get(normaliserTitre(titre)) ?? null;
  }

  /** Possession d'un jeu de la Boutique, par appid si on l'a, sinon par titre. */
  function possession(titre: string, appId?: number | null): Possession {
    return trouver(titre, appId)?.etat ?? null;
  }

  /** L'identifiant de la carte du jeu dans la bibliothèque, s'il y est. */
  function idBibliotheque(titre: string, appId?: number | null): string | null {
    return trouver(titre, appId)?.id ?? null;
  }

  return { possession, idBibliotheque };
}
