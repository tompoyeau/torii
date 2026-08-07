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
    theme.value = next;
    localStorage.setItem("ludo-theme", next);
    apply(next);
  }
  return { theme, toggle };
}
