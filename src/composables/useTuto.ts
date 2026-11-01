import { ref } from "vue";
import { tutoFait } from "../lib/tauri";

/**
 * Le tuto du premier lancement : un assistant où l'on règle vraiment les choses, puis une visite
 * en bulles sur la vraie interface. Il se montre seul à une installation neuve (`tuto_a_faire`
 * côté Rust), et se revoit depuis Paramètres → Général.
 */
type Phase = "ferme" | "assistant" | "visite";

const phase = ref<Phase>("ferme");

export function useTuto() {
  return {
    phase,
    demarrer: () => {
      phase.value = "assistant";
    },
    versVisite: () => {
      phase.value = "visite";
    },
    /** Terminé ou passé : dans les deux cas, on ne s'impose plus. */
    terminer: () => {
      phase.value = "ferme";
      void tutoFait();
    },
  };
}
