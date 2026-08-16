import { ref } from "vue";
import { storeDeals, storeGame, storeSearch, storeSuggest } from "../lib/tauri";
import type { StoreGame, StoreItem, StoreSuggestion } from "../types";

/** Critère de tri de la vitrine (mappé côté Rust vers CheapShark). */
export type StoreSort = "featured" | "savings" | "price" | "recent" | "rating";

// État partagé (singleton) de la boutique.
const items = ref<StoreItem[]>([]);
const loading = ref(false);
const sort = ref<StoreSort>("featured");
const query = ref("");
/** Requête effectivement affichée ("" = vitrine, sinon résultats de recherche). */
const activeQuery = ref("");

const selectedGameId = ref<string | null>(null);
const product = ref<StoreGame | null>(null);
const productLoading = ref(false);

/** Suggestions d'autocomplétion de la barre de recherche. */
const suggestions = ref<StoreSuggestion[]>([]);

/** Pioche en cours (bouton « Au hasard »). */
const randomLoading = ref(false);
/** La grille affiche une sélection aléatoire (et non la vitrine triée). */
const randomMode = ref(false);

let loadedOnce = false;
/** Jeton anti-course : seule la dernière requête lancée applique son résultat. */
let reqToken = 0;
let suggestToken = 0;

/**
 * Liste d'exclusion de boutiques, gérée par l'utilisateur (persistée en localStorage).
 * Une boutique exclue est entièrement masquée du comparatif et n'est jamais retenue
 * comme « meilleur prix ». Clé = nom de la boutique (fonctionne aussi pour Instant Gaming,
 * scrapé hors ITAD). Distinct de la catégorisation officiel/revendeur (celle-ci = factuelle,
 * fournie par le backend ; l'exclusion = choix perso de l'utilisateur).
 */
const STORE_EXCL_KEY = "ludo-excluded-stores";
function loadExcluded(): string[] {
  try {
    const raw = localStorage.getItem(STORE_EXCL_KEY);
    return raw ? (JSON.parse(raw) as string[]) : [];
  } catch {
    return [];
  }
}
const excludedStores = ref<string[]>(loadExcluded());
function persistExcluded() {
  try {
    localStorage.setItem(STORE_EXCL_KEY, JSON.stringify(excludedStores.value));
  } catch {
    /* stockage indisponible : on garde l'exclusion en mémoire pour la session. */
  }
}
/** true si la boutique est dans la liste d'exclusion de l'utilisateur. */
function isStoreExcluded(name: string): boolean {
  return excludedStores.value.includes(name);
}
/** Ajoute/retire une boutique de la liste d'exclusion (et persiste). */
function toggleStoreExcluded(name: string) {
  excludedStores.value = isStoreExcluded(name)
    ? excludedStores.value.filter((n) => n !== name)
    : [...excludedStores.value, name];
  persistExcluded();
}
/** Vide la liste d'exclusion (réaffiche toutes les boutiques). */
function clearExcludedStores() {
  excludedStores.value = [];
  persistExcluded();
}

/** Charge la vitrine (mises en avant / promos) selon le tri courant. */
async function loadHome() {
  loading.value = true;
  activeQuery.value = "";
  randomMode.value = false;
  const token = ++reqToken;
  const res = (await storeDeals(0, sort.value)) ?? mockDeals(sort.value);
  if (token !== reqToken) return; // une requête plus récente a pris le relais
  items.value = res;
  loading.value = false;
}

/** Lance une recherche (ou revient à la vitrine si la requête est vide). */
async function runSearch() {
  clearSuggestions();
  const q = query.value.trim();
  if (!q) {
    void loadHome();
    return;
  }
  loading.value = true;
  activeQuery.value = q;
  randomMode.value = false;
  const token = ++reqToken;
  const res = (await storeSearch(q)) ?? mockSearch(q);
  if (token !== reqToken) return;
  items.value = res;
  loading.value = false;
}

/** Met à jour les suggestions d'autocomplétion pour la saisie courante. */
async function fetchSuggestions(q: string) {
  const query = q.trim();
  if (query.length < 2) {
    suggestions.value = [];
    return;
  }
  const token = ++suggestToken;
  const res = (await storeSuggest(query)) ?? mockSuggest(query);
  if (token !== suggestToken) return; // une frappe plus récente a pris le relais
  suggestions.value = res;
}

/** Vide les suggestions (fermeture du menu). */
function clearSuggestions() {
  suggestToken++; // invalide toute requête en vol
  suggestions.value = [];
}

function setSort(s: StoreSort) {
  // En mode aléatoire, cliquer un tri fait toujours revenir à la vitrine triée.
  if (sort.value === s && !randomMode.value) return;
  sort.value = s;
  // Le tri ne concerne que la vitrine (pas les résultats de recherche).
  if (!activeQuery.value) void loadHome();
}

/** Ouvre la fiche produit d'un jeu (comparatif de prix + méta). */
async function openProduct(gameId: string) {
  clearSuggestions();
  selectedGameId.value = gameId;
  product.value = null;
  productLoading.value = true;
  const res = (await storeGame(gameId)) ?? mockGame(gameId);
  // Ne pas écraser si l'utilisateur a déjà changé de fiche / fermé.
  if (selectedGameId.value === gameId) {
    product.value = res;
    productLoading.value = false;
  }
}

/** Mélange une copie du tableau (Fisher-Yates). */
function shuffle<T>(arr: T[]): T[] {
  const out = [...arr];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

/**
 * Remplit la grille d'une **sélection de jeux au hasard**. Pour varier au-delà des
 * jeux déjà affichés, on tire une page + un tri aléatoires de la vitrine, puis on
 * mélange le résultat (repli sur la vitrine courante si la page tirée est vide).
 * Rappuyer repioche une nouvelle sélection.
 */
async function pickRandom() {
  if (randomLoading.value) return;
  randomLoading.value = true;
  loading.value = true;
  clearSuggestions();
  activeQuery.value = "";
  randomMode.value = true;
  const token = ++reqToken;
  try {
    const sorts: StoreSort[] = ["featured", "savings", "price", "recent", "rating"];
    const s = sorts[Math.floor(Math.random() * sorts.length)];
    const page = Math.floor(Math.random() * 15);
    const res = (await storeDeals(page, s)) ?? mockDeals(s);
    if (token !== reqToken) return; // une requête plus récente a pris le relais
    const pool = res.length ? res : items.value;
    items.value = shuffle(pool);
  } finally {
    if (token === reqToken) loading.value = false;
    randomLoading.value = false;
  }
}

function closeProduct() {
  selectedGameId.value = null;
  product.value = null;
  productLoading.value = false;
}

/**
 * Ouvre la fiche boutique correspondant au **titre** d'un jeu (depuis la bibliothèque) :
 * on résout le meilleur résultat via l'autocomplétion (rapide), puis on ouvre sa fiche.
 * Sans correspondance exacte, on bascule sur une recherche classique (grille de résultats).
 */
async function openForTitle(title: string) {
  clearSuggestions();
  // Affiche tout de suite l'overlay en chargement (évite un flash de la vitrine).
  selectedGameId.value = "__loading__";
  product.value = null;
  productLoading.value = true;
  const res = (await storeSuggest(title)) ?? mockSuggest(title);
  const first = res[0];
  if (!first) {
    selectedGameId.value = null;
    productLoading.value = false;
    query.value = title;
    void runSearch();
    return;
  }
  await openProduct(first.gameId);
}

export function useStore() {
  // Premier affichage de la boutique : charge la vitrine une seule fois.
  if (!loadedOnce) {
    loadedOnce = true;
    void loadHome();
  }
  return {
    items,
    loading,
    sort,
    query,
    activeQuery,
    selectedGameId,
    product,
    productLoading,
    suggestions,
    randomLoading,
    randomMode,
    loadHome,
    runSearch,
    setSort,
    openProduct,
    openForTitle,
    closeProduct,
    pickRandom,
    fetchSuggestions,
    clearSuggestions,
    excludedStores,
    isStoreExcluded,
    toggleStoreExcluded,
    clearExcludedStores,
  };
}

// --- Données fictives (hors Tauri : preview navigateur) -----------------------

function steamCover(appid: string): string {
  return `https://cdn.cloudflare.steamstatic.com/steam/apps/${appid}/library_600x900.jpg`;
}
const MOCK_ITEMS: StoreItem[] = [
  { gameId: "m1", title: "Baldur's Gate 3", coverUrl: steamCover("1086940"), price: 44.99, normalPrice: 59.99, savings: 25, storeName: "Steam", buyUrl: "https://store.steampowered.com/app/1086940" },
  { gameId: "m2", title: "Cyberpunk 2077", coverUrl: steamCover("1091500"), price: 17.99, normalPrice: 59.99, savings: 70, storeName: "GOG", buyUrl: "https://www.gog.com/" },
  { gameId: "m3", title: "Elden Ring", coverUrl: steamCover("1245620"), price: 34.99, normalPrice: 59.99, savings: 42, storeName: "Green Man Gaming", buyUrl: "https://www.greenmangaming.com/" },
  { gameId: "m4", title: "Hades", coverUrl: steamCover("1145360"), price: 12.49, normalPrice: 24.99, savings: 50, storeName: "Steam", buyUrl: "https://store.steampowered.com/app/1145360" },
  { gameId: "m5", title: "The Witcher 3: Wild Hunt", coverUrl: steamCover("292030"), price: 7.99, normalPrice: 39.99, savings: 80, storeName: "GOG", buyUrl: "https://www.gog.com/" },
  { gameId: "m6", title: "Hollow Knight", coverUrl: steamCover("367520"), price: 7.49, normalPrice: 14.99, savings: 50, storeName: "Humble Store", buyUrl: "https://www.humblebundle.com/" },
  { gameId: "m7", title: "Disco Elysium", coverUrl: steamCover("632470"), price: 11.99, normalPrice: 39.99, savings: 70, storeName: "Fanatical", buyUrl: "https://www.fanatical.com/" },
  { gameId: "m8", title: "Stardew Valley", coverUrl: steamCover("413150"), price: 10.49, normalPrice: 13.99, savings: 25, storeName: "Steam", buyUrl: "https://store.steampowered.com/app/413150" },
];

function mockDeals(sort: StoreSort): StoreItem[] {
  const by = [...MOCK_ITEMS];
  if (sort === "savings") by.sort((a, b) => b.savings - a.savings);
  else if (sort === "price") by.sort((a, b) => a.price - b.price);
  return by;
}

function mockSearch(q: string): StoreItem[] {
  const s = q.toLowerCase();
  return MOCK_ITEMS.filter((i) => i.title.toLowerCase().includes(s));
}

function mockSuggest(q: string): StoreSuggestion[] {
  const s = q.toLowerCase();
  return MOCK_ITEMS.filter((i) => i.title.toLowerCase().includes(s))
    .slice(0, 8)
    .map((i) => ({ gameId: i.gameId, title: i.title, coverUrl: i.coverUrl }));
}

function mockGame(gameId: string): StoreGame {
  const it = MOCK_ITEMS.find((i) => i.gameId === gameId) ?? MOCK_ITEMS[0];
  const r2 = (n: number) => Math.round(n * 100) / 100;
  return {
    gameId,
    title: it.title,
    coverUrl: it.coverUrl,
    heroUrl: it.coverUrl, // (mock : réutilise la jaquette)
    cheapestEver: r2(it.price * 0.8),
    prices: [
      { storeName: "Instant Gaming", price: r2(it.price - 2), retailPrice: it.normalPrice, savings: it.savings + 5, buyUrl: "https://www.instant-gaming.com/" },
      { storeName: it.storeName, price: it.price, retailPrice: it.normalPrice, savings: it.savings, buyUrl: it.buyUrl },
      { storeName: "Steam", price: r2(it.price + 3), retailPrice: it.normalPrice, savings: Math.max(0, it.savings - 8), buyUrl: "https://store.steampowered.com/" },
      { storeName: "Epic Games Store", price: r2(it.price + 5), retailPrice: it.normalPrice, savings: Math.max(0, it.savings - 12), buyUrl: "https://store.epicgames.com/" },
    ],
    description: "Un titre encensé par la critique. Explore un monde façonné à la main, affine ton style au fil des heures et laisse-toi porter par sa direction artistique.",
    genre: "Action-RPG",
    developer: "Studio Fictif",
    year: 2023,
    screenshots: [],
  };
}
