import { computed, ref, type ComputedRef } from "vue";

/**
 * Première image d'une liste qui n'a pas encore échoué à charger.
 *
 * Brancher `onError` sur l'`<img>` : l'adresse fautive est écartée et la suivante prend
 * sa place. Quand toutes ont échoué, `src` vaut `null` et le décor derrière (dégradé)
 * reste visible.
 */
export function useImageCascade(urls: () => (string | null | undefined)[]): {
  src: ComputedRef<string | null>;
  onError: () => void;
} {
  const failed = ref(new Set<string>());
  const src = computed(() => {
    for (const url of urls()) {
      if (url && !failed.value.has(url)) return url;
    }
    return null;
  });
  function onError() {
    if (src.value) failed.value = new Set(failed.value).add(src.value);
  }
  return { src, onError };
}
