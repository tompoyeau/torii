import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useLibrary } from "./useLibrary";
import { useUi } from "./useUi";
import type { Game } from "../types";

/** Une rangée du Salon (titre + jeux), telle que calculée par `SalonView`. */
export interface SalonRowDef {
  title: string;
  games: Game[];
}

/**
 * Navigation spatiale du Salon au **clavier et à la manette** (mode canapé / 10-foot UI).
 *
 * Grille : la rangée 0 = le hero (carrousel), les rangées 1..N = les rangées de contenu.
 * - Flèches / D-pad / stick gauche : se déplacer (← → dans une rangée, ↑ ↓ entre rangées ;
 *   ← → sur le hero fait défiler la sélection).
 * - Entrée / A : lancer (hero) ou ouvrir la fiche (tuile).
 * - Échap / B : remonter au hero.
 *
 * L'anneau de focus n'apparaît qu'une fois une touche/manette utilisée (`active`) et
 * disparaît dès qu'on repasse à la souris. La nav se suspend quand une surcouche est
 * ouverte (fiche jeu, paramètres, ajout manuel). Le composable possède aussi l'index du
 * hero et son défilement automatique (mis en pause pendant qu'on choisit dans le hero).
 */
export function useSalonNav(getRows: () => SalonRowDef[]) {
  const { spotlight, launchOrInstall } = useLibrary();
  const ui = useUi();

  const active = ref(false); // focus clavier/manette engagé (pilote l'anneau)
  const row = ref(0); // 0 = hero, 1..N = rangées de contenu
  const col = ref(0);
  const heroIndex = ref(0);

  const rowLens = computed(() => getRows().map((r) => r.games.length));
  const rowCount = computed(() => rowLens.value.length);

  // --- Hero : défilement automatique (possédé ici pour rester la seule source de vérité) ---
  let heroTimer: number | undefined;
  function restartHero() {
    clearInterval(heroTimer);
    heroTimer = window.setInterval(() => {
      const n = spotlight.value.length;
      if (!n) return;
      if (active.value && row.value === 0) return; // pause pendant qu'on choisit
      heroIndex.value = (heroIndex.value + 1) % n;
    }, 6000);
  }
  watch(
    spotlight,
    () => {
      if (heroIndex.value >= spotlight.value.length) heroIndex.value = 0;
      restartHero();
    },
    { immediate: true },
  );
  function setHero(i: number) {
    heroIndex.value = i;
    restartHero();
  }

  /** Une surcouche est ouverte → on laisse le clavier/manette à ces composants. */
  function suspended(): boolean {
    return ui.selectedGameId.value != null || ui.settingsOpen.value || ui.addGameOpen.value;
  }

  function clamp() {
    if (row.value < 0) row.value = 0;
    if (row.value > rowCount.value) row.value = rowCount.value;
    if (row.value >= 1) {
      const len = rowLens.value[row.value - 1] ?? 0;
      col.value = Math.min(Math.max(col.value, 0), Math.max(0, len - 1));
    }
  }

  function move(dx: number, dy: number) {
    active.value = true;
    if (dy !== 0) {
      const target = row.value + dy;
      if (target >= 0 && target <= rowCount.value) row.value = target;
      clamp();
      return;
    }
    if (dx !== 0) {
      if (row.value === 0) {
        const n = spotlight.value.length;
        if (n) setHero((heroIndex.value + (dx > 0 ? 1 : -1) + n) % n);
        return;
      }
      col.value += dx;
      clamp();
    }
  }

  function activate() {
    active.value = true;
    if (row.value === 0) {
      const g = spotlight.value[heroIndex.value];
      if (g) launchOrInstall(g);
      return;
    }
    const g = getRows()[row.value - 1]?.games[col.value];
    if (g) ui.openGame(g.id);
  }

  function back() {
    if (row.value > 0) {
      row.value = 0;
      active.value = true;
    }
  }

  // Remonter au hero → ramener la page en haut.
  watch(
    () => [active.value, row.value] as const,
    ([act, r]) => {
      if (act && r === 0) window.scrollTo({ top: 0, behavior: "smooth" });
    },
  );

  // --- Clavier ---
  function onKeydown(e: KeyboardEvent) {
    if (suspended()) return;
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
    switch (e.key) {
      case "ArrowLeft": e.preventDefault(); move(-1, 0); break;
      case "ArrowRight": e.preventDefault(); move(1, 0); break;
      case "ArrowUp": e.preventDefault(); move(0, -1); break;
      case "ArrowDown": e.preventDefault(); move(0, 1); break;
      case "Enter": case " ": e.preventDefault(); activate(); break;
      case "Escape": case "Backspace": back(); break;
    }
  }
  // Repasser à la souris masque l'anneau de focus.
  function onMouseMove() {
    if (active.value) active.value = false;
  }

  // --- Manette (polling en rAF, détection de front montant) ---
  let raf = 0;
  const prevButtons: boolean[] = [];
  let axisCooldown = 0;
  function pollGamepad(ts: number) {
    raf = requestAnimationFrame(pollGamepad);
    if (suspended()) return;
    const pads = navigator.getGamepads?.() ?? [];
    const gp = [...pads].find((p): p is Gamepad => !!p);
    if (!gp) return;
    const edge = (i: number) => {
      const now = !!gp.buttons[i]?.pressed;
      const was = prevButtons[i] ?? false;
      prevButtons[i] = now;
      return now && !was;
    };
    // D-pad (12↑ 13↓ 14← 15→), A = 0, B = 1.
    if (edge(12)) move(0, -1);
    if (edge(13)) move(0, 1);
    if (edge(14)) move(-1, 0);
    if (edge(15)) move(1, 0);
    if (edge(0)) activate();
    if (edge(1)) back();
    // Stick gauche (axe 0 = X, 1 = Y), avec anti-rebond.
    const ax = gp.axes[0] ?? 0;
    const ay = gp.axes[1] ?? 0;
    const TH = 0.5;
    if (ts > axisCooldown) {
      let moved = true;
      if (ax <= -TH) move(-1, 0);
      else if (ax >= TH) move(1, 0);
      else if (ay <= -TH) move(0, -1);
      else if (ay >= TH) move(0, 1);
      else moved = false;
      if (moved) axisCooldown = ts + 180;
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("mousemove", onMouseMove);
    raf = requestAnimationFrame(pollGamepad);
  });
  onBeforeUnmount(() => {
    window.removeEventListener("keydown", onKeydown);
    window.removeEventListener("mousemove", onMouseMove);
    cancelAnimationFrame(raf);
    clearInterval(heroTimer);
  });

  const heroActive = computed(() => active.value && row.value === 0);

  return { active, row, col, heroIndex, setHero, heroActive };
}
