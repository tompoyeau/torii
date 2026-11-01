import { ref } from "vue";

type Theme = "dark" | "light";

const stored = (localStorage.getItem("ludo-theme") as Theme | null) ?? null;
const theme = ref<Theme | null>(stored);

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/**
 * La barre du navigateur (ou du système, sur un site installé) prend le fond de la page. Seule la
 * version web porte cette balise : ailleurs, ces lignes ne trouvent rien à faire.
 */
function teinterLaBarre() {
  const sombre = theme.value ? theme.value === "dark" : systemPrefersDark();
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", sombre ? "#131019" : "#f2f0f7");
}

function apply(value: Theme | null) {
  const root = document.documentElement;
  if (value) root.setAttribute("data-theme", value);
  else root.removeAttribute("data-theme");
  teinterLaBarre();
}
apply(theme.value);
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", teinterLaBarre);

export function useTheme() {
  function toggle() {
    const current = theme.value ?? (systemPrefersDark() ? "dark" : "light");
    const next: Theme = current === "dark" ? "light" : "dark";
    setTheme(next);
  }
  /** Fixe le thème : "dark"/"light", ou null pour suivre le système. */
  function setTheme(value: Theme | null) {
    theme.value = value;
    if (value) localStorage.setItem("ludo-theme", value);
    else localStorage.removeItem("ludo-theme");
    apply(value);
  }
  return { theme, toggle, setTheme };
}
