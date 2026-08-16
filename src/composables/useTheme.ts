import { ref } from "vue";

type Theme = "dark" | "light";

const stored = (localStorage.getItem("ludo-theme") as Theme | null) ?? null;
const theme = ref<Theme | null>(stored);

function apply(value: Theme | null) {
  const root = document.documentElement;
  if (value) root.setAttribute("data-theme", value);
  else root.removeAttribute("data-theme");
}
apply(theme.value);

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

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
