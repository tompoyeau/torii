import { ref } from "vue";

/** Petites notifications éphémères in-app (coin bas de l'écran). */
export interface Toast {
  id: number;
  message: string;
}

const toasts = ref<Toast[]>([]);
let seq = 0;

/** Affiche un toast qui disparaît tout seul après `ms`. */
export function showToast(message: string, ms = 2600) {
  const id = ++seq;
  toasts.value = [...toasts.value, { id, message }];
  window.setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, ms);
}

export function useToast() {
  return { toasts };
}
