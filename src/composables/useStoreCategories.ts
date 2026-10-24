import { computed, reactive, ref, watch } from "vue";
import { parcoursDeDemo, tagsDeDemo } from "../data/storeMock";
import { langueCourante } from "../i18n";
import { regionCourante } from "../i18n/regions";
import { hasTauriRuntime, storeBrowse, storeTags } from "../lib/tauri";
import type { StoreBrowseSort, StoreShelfItem, StoreTag } from "../types";
import { useStore } from "./useStore";
import { useUi } from "./useUi";

/**
 * Boutique — parcourir le catalogue Steam par catégories (tags), plusieurs à la fois
 * (singleton).
 *
 * 🔑 MÊME VOCABULAIRE QUE LA BIBLIOTHÈQUE : les tags Steam, dans la langue de
 * l'interface. Les jeux viennent de Steam (`store_browse`), le prix du comparateur.
 *
 * 🔑 UNE CATÉGORIE OU UN TITRE, PAS LES DEUX. Steam filtre par tags mais ne cherche pas
 * de texte, le comparateur cherche un titre mais ignore les tags : cocher une catégorie
 * efface la recherche par titre, et lancer une recherche par titre décoche tout.
 *
 * ⚠️ Gardé en mémoire 30 minutes, comme les rayons (`useStoreHome`) : chaque page coûte
 * un appel au comparateur, dont le quota est partagé par tous les utilisateurs de Torii.
 * Revenir en arrière sur une recherche déjà faite ne redemande donc rien.
 */

type Etat = "vide" | "chargement" | "pret" | "echec";

const DUREE = 30 * 60_000;

const tags = ref<StoreTag[]>([]);
let tagsCle = "";
/** Noms connus des tags, y compris ceux ouverts depuis une fiche avant le chargement du menu. */
const noms = reactive(new Map<number, string>());

const choisis = ref<number[]>([]);
const tri = ref<StoreBrowseSort>("topSellers");
const jeux = ref<StoreShelfItem[]>([]);
const total = ref(0);
const etat = ref<Etat>("vide");
const chargementSuite = ref(false);

interface Resultat {
  total: number;
  jeux: StoreShelfItem[];
  pages: number;
  at: number;
}
const memoire = new Map<string, Resultat>();
/** Anti-course : seule la dernière demande a le droit d'écrire. */
let jeton = 0;

const contexte = () => `${regionCourante.value}|${langueCourante.value}`;
const cle = () => `${contexte()}|${tri.value}|${[...choisis.value].sort((a, b) => a - b).join(",")}`;

async function chargerTags() {
  if (tagsCle === langueCourante.value && tags.value.length) return;
  const langue = langueCourante.value;
  const liste = hasTauriRuntime() ? await storeTags() : tagsDeDemo();
  if (!liste?.length || langue !== langueCourante.value) return;
  tags.value = liste;
  tagsCle = langue;
  for (const t of liste) noms.set(t.id, t.nom);
}

async function charger() {
  const ma = ++jeton;
  if (!choisis.value.length) {
    jeux.value = [];
    total.value = 0;
    etat.value = "vide";
    return;
  }
  const k = cle();
  const connu = memoire.get(k);
  if (connu && Date.now() - connu.at < DUREE) {
    jeux.value = connu.jeux;
    total.value = connu.total;
    etat.value = "pret";
    return;
  }
  etat.value = "chargement";
  const res = hasTauriRuntime()
    ? await storeBrowse(choisis.value, tri.value, 0)
    : parcoursDeDemo(choisis.value, tri.value, 0);
  if (ma !== jeton) return;
  if (!res) {
    etat.value = "echec";
    return;
  }
  jeux.value = res.jeux;
  total.value = res.total;
  etat.value = "pret";
  memoire.set(k, { total: res.total, jeux: res.jeux, pages: 1, at: Date.now() });
}

async function chargerPlus() {
  const k = cle();
  const connu = memoire.get(k);
  if (!connu || chargementSuite.value || jeux.value.length >= total.value) return;
  const ma = jeton;
  chargementSuite.value = true;
  const res = hasTauriRuntime()
    ? await storeBrowse(choisis.value, tri.value, connu.pages)
    : parcoursDeDemo(choisis.value, tri.value, connu.pages);
  chargementSuite.value = false;
  if (ma !== jeton || !res) return;
  // Steam peut décaler son classement entre deux pages : pas de doublon à l'écran.
  const vus = new Set(jeux.value.map((j) => j.appId));
  const suite = [...jeux.value, ...res.jeux.filter((j) => !vus.has(j.appId))];
  jeux.value = suite;
  memoire.set(k, { ...connu, jeux: suite, pages: connu.pages + 1 });
}

// Toute la recherche se relance d'elle-même : cases, tri, région ou langue.
watch([choisis, tri, regionCourante, langueCourante], () => void charger());

/** Efface la recherche par titre en cours (les deux ne se combinent pas). */
function quitterRechercheTitre() {
  const store = useStore();
  store.query.value = "";
  store.clearSuggestions();
  // Une recherche lancée remet aussi la grille du bas sur la vitrine ; une saisie pas
  // encore lancée n'a rien chargé, il suffit de la vider.
  if (store.activeQuery.value) store.runSearch();
}

function basculer(id: number) {
  quitterRechercheTitre();
  choisis.value = choisis.value.includes(id) ? choisis.value.filter((c) => c !== id) : [...choisis.value, id];
}

function effacer() {
  choisis.value = [];
}

/**
 * Ouvre la Boutique sur UNE catégorie : tag cliqué dans une fiche, « Tout voir » d'un
 * rayon. Referme les fiches ouvertes par-dessus, pour que les résultats se voient.
 */
function ouvrir(tag: StoreTag) {
  noms.set(tag.id, tag.nom);
  quitterRechercheTitre();
  const store = useStore();
  const ui = useUi();
  if (store.selectedGameId.value) store.closeProduct();
  ui.selectedGameId.value = null;
  choisis.value = [tag.id];
  ui.showStore();
}

/** Les catégories cochées, nommées (« Coop + Survie »). */
const libelle = computed(() => choisis.value.map((id) => noms.get(id) ?? `#${id}`).join(" + "));

export function useStoreCategories() {
  return {
    tags, noms, choisis, tri, jeux, total, etat, chargementSuite, libelle,
    chargerTags, charger, chargerPlus, basculer, effacer, ouvrir,
  };
}
