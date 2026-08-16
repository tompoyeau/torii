import { watch } from "vue";
import { notify, wishlistAll } from "../lib/tauri";
import { formatEur } from "../lib/format";
import { usePreferences } from "./usePreferences";

const { prefs } = usePreferences();

/**
 * Notifie les baisses de prix de la wishlist Steam. Déclencheur = un jeu passe **en
 * promotion** OU atteint son **plus bas prix historique** (ITAD). Anti-spam : on ne
 * re-notifie que si le prix descend sous le dernier prix déjà notifié ; quand la promo
 * se termine, l'entrée est réinitialisée (une future promo re-notifiera).
 *
 * Le contrôle tourne tant que Torii est ouvert (y compris réduit dans le tray), au
 * démarrage puis toutes les 6 h. Nécessite une session Steam connectée.
 */
const MAP_KEY = "ludo-wishlist-notif";
const SEEDED_KEY = "ludo-wishlist-notif-seeded";
const INTERVAL_MS = 6 * 60 * 60 * 1000; // 6 h
const FIRST_DELAY_MS = 30 * 1000; // laisse l'app/session s'initialiser

function loadMap(): Record<number, number> {
  try {
    const raw = localStorage.getItem(MAP_KEY);
    return raw ? (JSON.parse(raw) as Record<number, number>) : {};
  } catch {
    return {};
  }
}
function saveMap(m: Record<number, number>) {
  try {
    localStorage.setItem(MAP_KEY, JSON.stringify(m));
  } catch {
    /* ignore */
  }
}

let running = false;
async function check() {
  if (!prefs.wishlistNotifications || running) return;
  running = true;
  try {
    const items = await wishlistAll();
    if (!items) return; // hors Tauri ou Steam non connecté
    const map = loadMap();
    const seeded = localStorage.getItem(SEEDED_KEY) === "1";
    const toNotify: { title: string; price: number; savings: number }[] = [];
    let changed = false;

    for (const it of items) {
      if (it.price == null) continue;
      const isDeal =
        it.savings > 0 || (it.historyLow != null && it.price <= it.historyLow * 1.01);
      const last = map[it.appId];
      if (!isDeal) {
        // La promo est terminée : on réinitialise pour re-notifier à la prochaine.
        if (last != null) {
          delete map[it.appId];
          changed = true;
        }
        continue;
      }
      // Nouveau deal, ou prix plus bas que le dernier notifié.
      if (last == null || it.price < last - 0.01) {
        if (seeded) toNotify.push({ title: it.title, price: it.price, savings: it.savings });
        map[it.appId] = it.price;
        changed = true;
      }
    }

    if (changed) saveMap(map);
    // Premier passage = amorçage silencieux (on mémorise l'état sans spammer).
    if (!seeded) {
      localStorage.setItem(SEEDED_KEY, "1");
      return;
    }

    if (toNotify.length === 0) return;
    if (toNotify.length <= 3) {
      for (const d of toNotify) {
        const cut = d.savings > 0 ? ` (-${d.savings}%)` : "";
        void notify("💸 Baisse de prix", `${d.title} — ${formatEur(d.price)}${cut}`);
      }
    } else {
      void notify(
        "💸 Baisses de prix",
        `${toNotify.length} jeux de ta wishlist ont baissé (promo ou plus bas historique).`,
      );
    }
  } finally {
    running = false;
  }
}

let started = false;
/** Démarre la surveillance (au montage de l'app). Idempotent. */
export function startWishlistNotifier() {
  if (started) return;
  started = true;
  window.setTimeout(check, FIRST_DELAY_MS);
  window.setInterval(check, INTERVAL_MS);
  // Réagit immédiatement quand l'utilisateur active l'option (amorçage silencieux).
  watch(
    () => prefs.wishlistNotifications,
    (on) => {
      if (on) void check();
    },
  );
}
