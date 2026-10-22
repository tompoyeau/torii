import { onBeforeUnmount, onMounted, type Ref } from "vue";

/**
 * Appelle `rappel` une fois, quand l'élément approche de l'écran (300 px d'avance).
 *
 * Sert aux rayons du bas de la Boutique : on ne les demande à Steam et au comparateur
 * que si l'utilisateur descend jusqu'à eux.
 */
export function useQuandVisible(el: Ref<HTMLElement | null>, rappel: () => void) {
  let obs: IntersectionObserver | null = null;
  onMounted(() => {
    if (!el.value) return;
    if (typeof IntersectionObserver === "undefined") {
      rappel();
      return;
    }
    obs = new IntersectionObserver(
      (entrees) => {
        if (entrees.some((e) => e.isIntersecting)) {
          obs?.disconnect();
          obs = null;
          rappel();
        }
      },
      { rootMargin: "300px 0px" },
    );
    obs.observe(el.value);
  });
  onBeforeUnmount(() => obs?.disconnect());
}
