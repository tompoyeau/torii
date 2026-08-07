import { reactive } from "vue";
import type { Game } from "../types";

/** État du menu contextuel (clic droit sur une carte). Singleton partagé. */
interface CtxState {
  open: boolean;
  /** Position d'ancrage (coordonnées viewport du clic). */
  x: number;
  y: number;
  /** Jeu ciblé par le menu, ou null quand fermé. */
  game: Game | null;
}

const state = reactive<CtxState>({ open: false, x: 0, y: 0, game: null });

export function useContextMenu() {
  return {
    ctx: state,
    /** Ouvre le menu contextuel à l'emplacement du clic droit, pour un jeu donné. */
    openContext(e: MouseEvent, game: Game) {
      e.preventDefault();
      state.x = e.clientX;
      state.y = e.clientY;
      state.game = game;
      state.open = true;
    },
    closeContext() {
      state.open = false;
      state.game = null;
    },
  };
}
