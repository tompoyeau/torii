import { t } from "../i18n";
import { fusionnerAppareils } from "./libraries";
import { CLE_A_ENVOYER, CLE_COMPTE } from "./revendeurs";
import type {
  GameDto, LibGame, LibraryIndex, LibrarySnapshot, PlatformId, ToriiAccount, ToriiDevice, ToriiSignIn,
} from "../types";

/**
 * Le « backend » de la version web : ce que le Rust fait dans l'application, fait ici en
 * parlant aux Workers. `lib/tauri.ts` y envoie les commandes qu'il enverrait à Rust, et le
 * reste de l'interface ne voit pas la différence.
 *
 * Deux services : `torii-api` (compte, bibliothèques) et `torii-store` (la Boutique).
 *
 * ⚠️ Une commande absente d'ici n'existe pas sur le web : `call` rend alors sa valeur de repli,
 * et les ponts qui laissent remonter leurs erreurs lèvent. C'est voulu pour tout ce qui suppose
 * un PC : lancer un jeu, connecter un launcher, publier sa présence.
 */

type Args = Record<string, unknown>;

const API = import.meta.env.VITE_TORII_API ?? "https://torii-api.topo-host.com";
const BOUTIQUE = import.meta.env.VITE_TORII_STORE ?? "https://torii-store.topo-host.com";
const DELAI_MS = 20_000;

// --- Session ------------------------------------------------------------------------

const CLE_SESSION = "torii.session";

/**
 * Le jeton de session, dans ce navigateur.
 *
 * ⚠️ Pas de cookie : l'API et la page ne sont pas sur le même site, et Safari les bloque dans ce
 * cas. Le jeton n'existe donc que dans le `localStorage`, et la page ne doit jamais afficher de
 * HTML tiers (aucun `v-html` sur du contenu venu d'ailleurs). Chaque session se ferme depuis
 * Paramètres → Réseau Torii, sur n'importe quel appareil.
 */
function lireJeton(): string | null {
  try {
    return localStorage.getItem(CLE_SESSION);
  } catch {
    return null; // Navigation privée stricte : pas de stockage, donc pas de session durable.
  }
}

function ecrireJeton(jeton: string | null) {
  try {
    if (jeton) localStorage.setItem(CLE_SESSION, jeton);
    else localStorage.removeItem(CLE_SESSION);
  } catch {
    // Rien à faire : la session ne survivra pas à la page, c'est tout.
  }
}

/** Une session existe-t-elle dans ce navigateur ? Sans appel réseau. */
export const sessionPresente = () => lireJeton() !== null;

/**
 * Fin de session : le jeton, et ce qui appartient au compte. Sur un navigateur partagé, les
 * choix du compte précédent ne doivent pas se mêler à ceux du suivant.
 */
function oublierLaSession() {
  ecrireJeton(null);
  try {
    for (const cle of [CLE_EXCLUS, CLE_COMPTE, CLE_A_ENVOYER]) localStorage.removeItem(cle);
  } catch {
    // Sans stockage, il n'y a rien à oublier.
  }
}

/** Le nom de cet appareil dans « Mes appareils » : à défaut de mieux, son système. */
function nomAppareil(): string {
  const ua = navigator.userAgent;
  const systeme = /iPhone|iPad/.test(ua)
    ? "iOS"
    : /Android/.test(ua)
      ? "Android"
      : /Windows/.test(ua)
        ? "Windows"
        : /Mac OS/.test(ua)
          ? "macOS"
          : /Linux/.test(ua)
            ? "Linux"
            : "";
  return systeme ? `Navigateur ${systeme}` : "Navigateur";
}

// --- Transport ----------------------------------------------------------------------

/**
 * Un appel à l'un des services. Lève une `Error` au message lisible : les messages du serveur
 * sont écrits pour être montrés tels quels (« Code incorrect. »).
 */
async function appel<T>(base: string, chemin: string, init: RequestInit = {}): Promise<T> {
  const jeton = lireJeton();
  const headers = new Headers(init.headers);
  if (jeton) headers.set("authorization", `Bearer ${jeton}`);
  if (init.body) headers.set("content-type", "application/json");

  let reponse: Response;
  try {
    reponse = await fetch(`${base}${chemin}`, { ...init, headers, signal: AbortSignal.timeout(DELAI_MS) });
  } catch {
    throw new Error(t("web.injoignable"));
  }

  const corps = await reponse.json().catch(() => null);
  if (reponse.ok) return corps as T;

  // 🔑 Session expirée ou fermée depuis un autre appareil : on repart de l'écran de connexion.
  // Recharger est le seul moyen sûr de ne rien laisser en mémoire du compte précédent.
  if (reponse.status === 401 && jeton) {
    oublierLaSession();
    window.location.reload();
  }
  // Le serveur n'écrit qu'en français : ces deux-là s'affichent sur le web, dans la langue de l'interface.
  if (corps?.error === "compte_inexistant") throw new Error(t("web.pasDeCompte"));
  if (corps?.error === "captcha_refuse") throw new Error(t("web.captcha.refuse"));
  throw new Error(corps?.message ?? t("web.injoignable"));
}

const api = <T>(chemin: string, init?: RequestInit) => appel<T>(API, chemin, init);

const envoyer = <T>(chemin: string, corps: unknown, methode = "POST") =>
  api<T>(chemin, { method: methode, body: JSON.stringify(corps) });

// --- La Boutique --------------------------------------------------------------------

/** Région et langue, transmises par `set_locale` : le Worker en a besoin à chaque appel. */
let locale = { langue: "en", region: "US" };

const CLE_EXCLUS = "torii.revendeursMasques";

/**
 * Revendeurs masqués : ceux du compte, gardés ici pour partir à chaque appel (`useRevendeursCompte`
 * les tient alignés). La fiche produit reçoit toutes les offres et filtre elle-même ; le reste
 * passe par le Worker, seul à pouvoir choisir la meilleure offre.
 */
function lireExclus(): string[] {
  try {
    const liste = JSON.parse(localStorage.getItem(CLE_EXCLUS) ?? "[]");
    return Array.isArray(liste) ? liste.filter((n) => typeof n === "string") : [];
  } catch {
    return [];
  }
}

function ecrireExclus(liste: string[]): string[] {
  try {
    localStorage.setItem(CLE_EXCLUS, JSON.stringify(liste));
  } catch {
    // Sans stockage, le choix ne vaudra que pour cette page.
  }
  return liste;
}

/** Une route du Worker, avec ce dont dépend toute réponse : région, langue, revendeurs masqués. */
function boutique<T>(chemin: string, parametres: Record<string, string | number | null | undefined> = {}): Promise<T> {
  const q = new URLSearchParams({ region: locale.region, langue: locale.langue });
  for (const [cle, valeur] of Object.entries(parametres)) {
    if (valeur !== null && valeur !== undefined && valeur !== "") q.set(cle, String(valeur));
  }
  for (const nom of lireExclus()) q.append("exclus", nom);
  return appel<T>(BOUTIQUE, `${chemin}?${q}`);
}

// --- La bibliothèque ----------------------------------------------------------------

const PLATEFORMES: readonly string[] = ["steam", "epic", "gog", "riot", "ubisoft", "ea", "battlenet", "manual", "detected"];

/**
 * Un jeu synchronisé, une entrée par launcher : `mergeDuplicates` en refait une seule carte
 * multi-sources, exactement comme avec le scan d'un PC.
 *
 * ⚠️ `installed: false` ne veut rien dire ici : le serveur ne sait pas ce qui est installé où,
 * et l'interface web n'en parle pas. Il ne connaît pas non plus le temps de jeu : la liste
 * envoyée par le PC est volontairement pauvre, parce que les amis peuvent la lire.
 */
function versDtos(jeu: LibGame): GameDto[] {
  const plateformes = jeu.platforms.filter((p) => PLATEFORMES.includes(p));
  return (plateformes.length ? plateformes : ["manual"]).map((platform) => ({
    id: `${platform}:${jeu.key}`,
    title: jeu.title,
    platform: platform as PlatformId,
    installed: false,
    owned: true,
    familyShared: !!jeu.familyShared,
    sizeGb: 0,
    coverUrl: jeu.cover ?? null,
    launchTarget: "",
  }));
}

/** Ma bibliothèque : l'union de ce qu'ont déposé mes appareils. Vide sans session. */
async function bibliotheque(): Promise<GameDto[]> {
  if (!sessionPresente()) return [];
  const index = await api<LibraryIndex>("/v1/library");
  const instantanes = await Promise.all(
    index.mine.map((e) =>
      api<LibrarySnapshot>(`/v1/library/${e.accountId}/${e.deviceId}`)
        .then((s) => s.games)
        // Un appareil illisible (retiré entre-temps) ne doit pas vider toute la vue.
        .catch(() => [] as LibGame[]),
    ),
  );
  return fusionnerAppareils(instantanes).flatMap(versDtos);
}

// --- Les commandes ------------------------------------------------------------------

const COMMANDES: Record<string, (a: Args) => Promise<unknown>> = {
  set_locale: async ({ language, region }) => {
    locale = { langue: String(language), region: String(region) };
  },

  // Compte
  torii_request_code: async ({ email, captcha }) => {
    const r = await envoyer<{ devCode?: string }>("/v1/auth/request-code", { email, captcha });
    return r.devCode ?? null;
  },
  // 🔑 Pas de création de compte sur le web : un compte se crée dans l'application, et le serveur
  // refuse d'en créer un depuis une page (`server/src/captcha.js`).
  torii_verify: async ({ email, code }): Promise<ToriiSignIn> => {
    const r = await envoyer<{ token: string; account: ToriiAccount }>("/v1/auth/verify", {
      email,
      code,
      device: nomAppareil(),
    });
    ecrireJeton(r.token);
    return { account: r.account, created: false, signupToken: null };
  },
  torii_me: async () => {
    if (!sessionPresente()) return null;
    return (await api<{ account: ToriiAccount }>("/v1/me")).account;
  },
  torii_logout: async () => {
    // Le jeton part même si l'appel échoue : une panne réseau ne doit pas empêcher de se déconnecter.
    try {
      await envoyer("/v1/auth/logout", {});
    } finally {
      oublierLaSession();
    }
  },
  torii_set_profile: async ({ displayName }) =>
    (await envoyer<{ account: ToriiAccount }>("/v1/me", { displayName }, "PATCH")).account,
  torii_devices: async () => (await api<{ sessions: ToriiDevice[] }>("/v1/sessions")).sessions,
  torii_revoke_device: ({ id }) => api(`/v1/sessions/${id}`, { method: "DELETE" }),
  torii_revoke_other_devices: () => api("/v1/sessions", { method: "DELETE" }),

  // Bibliothèque
  scan_library: () => bibliotheque(),
  library_index: () => api<LibraryIndex>("/v1/library"),

  // Boutique
  store_home: () => boutique("/v1/home"),
  store_deals: ({ page, sort }) => boutique("/v1/deals", { page: page as number, sort: sort as string }),
  store_shelf: ({ kind, tag }) => boutique("/v1/shelf", { kind: kind as string, tag: tag as number | null }),
  store_tags: () => boutique("/v1/tags"),
  store_browse: ({ tags, sort, page }) =>
    boutique("/v1/browse", { tags: (tags as number[]).join(","), sort: sort as string, page: page as number }),
  store_game_extras: ({ appId, dlc, tags }) =>
    boutique("/v1/extras", { appId: appId as number, dlc: (dlc as number[]).join(","), tags: (tags as number[]).join(",") }),
  store_search: ({ query }) => boutique("/v1/search", { q: query as string }),
  store_suggest: ({ query }) => boutique("/v1/suggest", { q: query as string }),
  store_game: ({ gameId }) => boutique(`/v1/game/${encodeURIComponent(gameId as string)}`),
  store_id_for_steam: async ({ appId }) => (await boutique<{ id: string | null }>(`/v1/id-for-steam/${appId}`)).id,
  lien_partage: async ({ titre }) => (await boutique<{ lien: string | null }>("/v1/share", { titre: titre as string })).lien,

  // Revendeurs masqués
  get_excluded_stores: async () => lireExclus(),
  set_store_excluded: async ({ name, excluded }) => {
    const reste = lireExclus().filter((n) => n !== name);
    return ecrireExclus(excluded ? [...reste, String(name)] : reste);
  },
  clear_excluded_stores: async () => ecrireExclus([]),
  replace_excluded_stores: async ({ stores }) => ecrireExclus(stores as string[]),
  torii_set_hidden_stores: ({ stores }) => envoyer("/v1/me/hidden-stores", { stores }, "PUT"),
};

/** Cette commande existe-t-elle sur le web ? */
export const commandeConnue = (cmd: string) => Object.prototype.hasOwnProperty.call(COMMANDES, cmd);

/** Exécute une commande. Lève si elle échoue, ou si elle n'existe pas sur le web. */
export async function commandeWeb<T>(cmd: string, args: Args = {}): Promise<T> {
  if (!commandeConnue(cmd)) throw new Error(t("web.pasSurLeWeb"));
  return (await COMMANDES[cmd](args)) as T;
}
