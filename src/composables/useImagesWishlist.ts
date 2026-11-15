import { computed } from "vue";
import { useWishlist } from "./useWishlist";

/**
 * Les images que la wishlist connaît pour un jeu de la Boutique (par identifiant ITAD) :
 * capsule Steam, puis jaquette ITAD de repli. Les cartes de la Boutique les ajoutent en
 * secours de leurs propres images.
 *
 * Pourquoi : le comparateur n'a pas de jaquette pour tous les jeux, et l'ancienne adresse
 * des images Steam n'existe plus pour beaucoup de jeux récents. Un jeu suivi s'affichait
 * ainsi sans visuel dans la Boutique, alors que la page Wishlist en montrait un.
 * Rien de plus n'est chargé : ce sont les données de la wishlist, déjà en mémoire.
 */
export function useImagesWishlist() {
  const { items } = useWishlist();
  const parJeu = computed(
    () => new Map(items.value.filter((i) => i.gameId).map((i) => [i.gameId, [i.coverUrl, i.coverFallbackUrl]])),
  );
  return (gameId: string): (string | null | undefined)[] => parJeu.value.get(gameId) ?? [];
}
