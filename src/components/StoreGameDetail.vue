<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { usePossession } from "../composables/usePossession";
import { useStore } from "../composables/useStore";
import { useScrollLock } from "../composables/useScrollLock";
import { useFocusTrap } from "../composables/useFocusTrap";
import { useToriiWishlist } from "../composables/useToriiWishlist";
import { useUi } from "../composables/useUi";
import { useModaleDeNavigation, useNavigation } from "../composables/useNavigation";
import { avecRaccourci } from "../composables/useRaccourcis";
import { ouvrirPage } from "../composables/usePageWeb";
import { activation, dateLongue, descripteur, libelleAvis, libelleFin, pageSteam, tonAvis } from "../lib/boutique";
import { openExternal } from "../lib/tauri";
import { formatPrix } from "../lib/format";
import { t } from "../i18n";
import { deviseAttendue, instantGamingPertinent } from "../i18n/regions";
import type { StorePrice, StoreReview } from "../types";
import StorePossede from "./StorePossede.vue";
import StoreSheetBas from "./StoreSheetBas.vue";
import StoreSheetBlocs from "./StoreSheetBlocs.vue";
import StoreSheetConfig from "./StoreSheetConfig.vue";
import StoreSheetFacts from "./StoreSheetFacts.vue";
import StoreSheetMedia from "./StoreSheetMedia.vue";
import StoreSheetVerdict from "./StoreSheetVerdict.vue";

const { product, productLoading, productIndisponible, selectedGameId, closeProduct, retryProduct, isStoreExcluded, toggleStoreExcluded } =
  useStore();
const { isWishlisted, toggle: toggleWishlist } = useToriiWishlist();
const { possession, idBibliotheque } = usePossession();
const { openGame } = useUi();

const open = computed(() => selectedGameId.value != null);
useScrollLock(open);

/** Même raison que la fiche de la bibliothèque : le panneau recouvre la page. */
const panneau = ref<HTMLElement | null>(null);
useFocusTrap(panneau, open);

/** Un montant dans la devise de l'offre qui le porte. */
const price = (montant: number, offre: StorePrice) => formatPrix(montant, offre.currency);

/**
 * La devise de la fiche, pour l'avertissement en pied de colonne. Celle des offres si
 * elles en portent une, sinon celle attendue pour la région.
 */
const deviseFiche = computed(
  () => product.value?.currency || product.value?.prices.find((p) => p.currency)?.currency || deviseAttendue.value,
);

/** Offres retenues (hors boutiques exclues par l'utilisateur), triées par prix croissant. */
const visiblePrices = computed(() => (product.value?.prices ?? []).filter((p) => !isStoreExcluded(p.storeName)));
/** Offres masquées par l'utilisateur (liste d'exclusion perso). */
const hiddenPrices = computed(() => (product.value?.prices ?? []).filter((p) => isStoreExcluded(p.storeName)));
/** Affiche/replie la liste des boutiques masquées (pour les réafficher). */
const showHidden = ref(false);

/** Meilleure offre = 1re offre NON exclue ET disponible (jamais une rupture de stock). */
const best = computed(() => visiblePrices.value.find((p) => p.available !== false) ?? null);

/**
 * Acheter : la page du revendeur s'ouvre DANS Torii, barre de Torii
 * au-dessus pour voir sur quel site on paie, et « Ouvrir dans le navigateur » à portée
 * pour qui préfère son gestionnaire de mots de passe.
 */
function buy(url: string) {
  ouvrirPage(url);
}

function hideBroken(e: Event) {
  (e.target as HTMLElement).style.display = "none";
}

/**
 * La visionneuse capte ses touches ; Échap pour fermer la fiche passe par
 * `useRaccourcis` (même raison que dans `GameDetail.vue`).
 */
function onKey(e: KeyboardEvent) {
  if (zoom.value == null || e.altKey) return;
  if (e.key === "Escape") {
    e.preventDefault();
    zoom.value = null;
  } else if (e.key === "ArrowRight") stepZoom(1);
  else if (e.key === "ArrowLeft") stepZoom(-1);
}
onMounted(() => document.addEventListener("keydown", onKey));
onBeforeUnmount(() => document.removeEventListener("keydown", onKey));
useModaleDeNavigation({ ouverte: () => zoom.value != null, fermer: () => (zoom.value = null) });

/**
 * Le bouton retour recule d'UNE étape : de la fiche d'un jeu similaire à celle du jeu
 * d'où l'on vient, et il le dit. Échap, lui, sort de la fiche (retour à la vitrine).
 */
const { reculer, avancer, peutAvancer, libellePrecedent, libelleSuivant, fermer } = useNavigation();
function retour() {
  if (!reculer()) fermer("produit");
}

/**
 * Captures : celles de Steam (toutes, en deux tailles) quand le jeu y est, sinon celles
 * d'IGDB (une seule taille, cinq au plus).
 */
const captures = computed(() => {
  const steam = product.value?.steam?.captures ?? [];
  if (steam.length) return steam;
  return (product.value?.screenshots ?? []).map((s) => ({ mini: s, grande: s }));
});

/** Visionneuse plein écran des captures (index dans `shots`, sinon fermée). */
const zoom = ref<number | null>(null);
const shots = computed(() => captures.value.map((c) => c.grande));
function stepZoom(delta: number) {
  const n = shots.value.length;
  if (zoom.value == null || n === 0) return;
  zoom.value = (zoom.value + delta + n) % n;
}

// Une autre fiche s'ouvre (un DLC, un jeu similaire) : on repart du haut (la description
// complète se replie d'elle-même, son composant étant indexé sur le jeu).
watch(selectedGameId, () => {
  zoom.value = null;
  if (panneau.value) panneau.value.scrollTop = 0;
});

/** Le jeu est déjà dans la bibliothèque : on le dit, et on y mène. */
const possede = computed(() => (product.value ? possession(product.value.title, product.value.appId) : null));
function voirBibliotheque() {
  const p = product.value;
  const id = p ? idBibliotheque(p.title, p.appId) : null;
  if (!id) return;
  closeProduct();
  openGame(id);
}

function ouvrirSteam() {
  if (product.value?.appId) ouvrirPage(pageSteam(product.value.appId));
}

/** Notes critiques à côté de celle de Steam (qui vient, elle, de la fiche Steam). */
const note = (source: string): StoreReview | null => product.value?.reviews?.find((r) => r.source === source) ?? null;
const tonNote = (score: number) => (score >= 75 ? "pos" : score >= 50 ? "mixte" : "neg");

/** Ligne sous le titre : studio, sortie, genres. */
const faits = computed(() => {
  const p = product.value;
  if (!p) return [];
  const annee = p.releaseDate ? p.releaseDate.slice(0, 4) : p.year ? String(p.year) : "";
  const genres = p.steam?.tags.length ? p.steam.tags.slice(0, 3) : p.genre ? [p.genre] : [];
  return [p.developers?.[0] ?? p.developer ?? "", annee, ...genres].filter(Boolean);
});

/** Ce qu'il faut savoir d'une offre : activation, fin, code promo. */
const detailOffre = (p: StorePrice) =>
  [activation(p.storeName, p.drm), libelleFin(p.expiry), p.voucher ? t("boutique.produit.code", { code: p.voucher }) : ""]
    .filter(Boolean)
    .join(" · ");
</script>

<template>
  <div ref="panneau" class="sd" data-defilement="produit" :class="{ open }" role="dialog" aria-modal="true" tabindex="-1" :aria-label="t('boutique.produit.fiche')">
    <template v-if="open">
      <div class="sd-banner">
        <div class="sd-banner-art" :style="{ background: 'linear-gradient(160deg,#241a3a,#0e0a18)' }" />
        <img v-if="product?.heroUrl" class="sd-banner-img" :src="product.heroUrl" alt="" @error="hideBroken" />
        <div class="sd-scrim" />
        <div class="sd-nav">
          <button class="sd-back" :title="avecRaccourci(t('bibliotheque.barre.precedent'), t('reglages.raccourcis.touches.alt'), '←')" @click="retour">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
            <span class="sd-back-label">{{ libellePrecedent ?? t("boutique.titre") }}</span>
          </button>
          <button
            v-if="peutAvancer"
            class="sd-forward"
            :title="avecRaccourci(t('bibliotheque.barre.suivantVers', { lieu: libelleSuivant ?? '' }), t('reglages.raccourcis.touches.alt'), '→')"
            :aria-label="t('bibliotheque.barre.suivantVers', { lieu: libelleSuivant ?? '' })"
            @click="avancer()"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        </div>
        <div class="sd-header">
          <span class="sd-tag">{{ t("boutique.titre") }}</span>
          <h1 class="sd-title">{{ product?.title ?? (productIndisponible ? t("boutique.produit.fiche") : t("boutique.produit.chargement")) }}</h1>
          <div v-if="faits.length" class="sd-facts">{{ faits.join(" · ") }}</div>
          <div v-if="product" class="sd-badges">
            <button
              v-if="product.steam?.reviewScore"
              class="badge-note"
              :title="t('boutique.produit.voirSur', { source: 'Steam' })"
              @click="ouvrirSteam"
            >
              <span class="avis" :class="tonAvis(product.steam.reviewScore)">Steam · {{ libelleAvis(product.steam.reviewScore) }}<template v-if="product.steam.reviewPercent != null"> · {{ product.steam.reviewPercent }} %</template></span>
            </button>
            <template v-for="src in ['Metascore', 'OpenCritic']" :key="src">
              <button
                v-if="note(src)"
                class="badge-note"
                :title="t('boutique.produit.voirSur', { source: src === 'Metascore' ? 'Metacritic' : src })"
                :disabled="!note(src)!.url"
                @click="note(src)!.url && openExternal(note(src)!.url!)"
              >
                <b class="score" :class="tonNote(note(src)!.score)">{{ note(src)!.score }}</b>{{ src === "Metascore" ? "Metacritic" : src }}
              </button>
            </template>
            <span v-if="product.steam?.classification" class="badge-note fixe">
              <b>{{ product.steam.classification.systeme }} {{ product.steam.classification.note }}</b>
            </span>
            <template v-if="possede">
              <StorePossede :etat="possede" />
              <button class="badge-note lien" @click="voirBibliotheque">{{ t("boutique.produit.voirBibliotheque") }} →</button>
            </template>
          </div>
        </div>
      </div>

      <div class="sd-body">
        <div class="sd-main">
          <div v-if="productLoading && !product" class="sd-loading">
            <span class="spin" /> {{ t("boutique.produit.chargementFiche") }}
          </div>
          <div v-else-if="productIndisponible" class="sd-indisponible">
            <p>{{ t("boutique.indisponible") }}</p>
            <p class="dim">{{ t("boutique.indisponibleAide") }}</p>
            <button class="chip" @click="retryProduct">{{ t("boutique.accueil.reessayer") }}</button>
          </div>
          <template v-else-if="product">
            <StoreSheetMedia
              :key="product.gameId"
              class="sd-sec"
              :bandes-annonces="product.steam?.bandesAnnonces ?? []"
              :captures="captures"
              @agrandir="zoom = $event"
              @steam="ouvrirSteam"
            />

            <section class="sd-sec">
              <h4>{{ t("fiche.apropos") }}</h4>
              <p v-if="product.description" class="desc">{{ product.description }}</p>
              <p v-else class="desc dim">{{ t("fiche.aucuneDescription") }}</p>
              <StoreSheetBlocs v-if="product.steam" :key="product.gameId" :blocs="product.steam.description" />
            </section>

            <StoreSheetFacts
              class="sd-sec"
              :steam="product.steam"
              :joueurs-jour="product.playersDay"
              :joueurs-record="product.playersPeak"
              :suivis="product.waitlisted"
            />

            <StoreSheetBas :key="`dlc-${product.gameId}`" class="sd-sec" :jeu="product" partie="dlc" />

            <StoreSheetConfig v-if="product.steam?.configuration" class="sd-sec" :configuration="product.steam.configuration" />

            <StoreSheetBas :key="`sim-${product.gameId}`" class="sd-sec" :jeu="product" partie="similaires" />
          </template>
        </div>

        <aside class="sd-prices">
          <div v-if="best" class="best">
            <div class="best-top">
              <span class="best-label">{{ t("boutique.produit.meilleurPrix") }}</span>
              <span v-if="best.savings > 0" class="best-save">-{{ best.savings }}%</span>
            </div>
            <div class="best-price">{{ price(best.price, best) }}</div>
            <div v-if="best.savings > 0" class="best-was">{{ t("boutique.produit.auLieuDe", { prix: price(best.retailPrice, best) }) }}</div>
            <button class="buy big" @click="buy(best.buyUrl)">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6h15l-1.5 9h-12z" /><circle cx="9" cy="20" r="1.4" /><circle cx="18" cy="20" r="1.4" /><path d="M6 6 5 3H2" /></svg>
              {{ t("boutique.produit.acheterChez", { boutique: best.storeName }) }}
            </button>
            <button
              v-if="product"
              class="wish"
              :class="{ on: isWishlisted(product.gameId) }"
              @click="toggleWishlist({ gameId: product.gameId, title: product.title, coverUrl: product.coverUrl })"
            >
              <svg viewBox="0 0 24 24" :fill="isWishlisted(product.gameId) ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2"><path d="M12 20s-7-4.3-7-9.3A3.7 3.7 0 0 1 12 8a3.7 3.7 0 0 1 7 2.7c0 5-7 9.3-7 9.3Z" /></svg>
              {{ isWishlisted(product.gameId) ? t("boutique.produit.dansWishlist") : t("boutique.ajouterWishlist") }}
            </button>
          </div>

          <StoreSheetVerdict
            v-if="best && product"
            :prix="best.price"
            :devise="best.currency || deviseFiche"
            :bas3m="product.low3m"
            :bas1a="product.low1y"
            :bas-tout="product.cheapestEver"
          />

          <div v-if="product && product.prices.length > 1" class="compare">
            <div class="compare-label">{{ t("boutique.produit.comparer", { n: visiblePrices.length }) }}</div>
            <div v-for="(p, i) in visiblePrices" :key="p.storeName + p.buyUrl + i" class="row" :class="{ oos: p.available === false }">
              <span class="row-store">
                {{ p.storeName }}
                <span v-if="detailOffre(p)" class="row-detail">{{ detailOffre(p) }}</span>
              </span>
              <span v-if="p.available === false" class="row-oos">{{ t("boutique.produit.rupture") }}</span>
              <span v-else-if="p.savings > 0" class="row-save">-{{ p.savings }}%</span>
              <span class="row-price">{{ price(p.price, p) }}</span>
              <button class="row-icon" :title="t('boutique.produit.masquerVendeur')" @click="toggleStoreExcluded(p.storeName)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" /><path d="M3 3l18 18" /></svg>
              </button>
              <button class="row-buy" :title="p.available === false ? t('boutique.produit.voirBoutique') : t('boutique.produit.acheter')" @click="buy(p.buyUrl)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
              </button>
            </div>

            <template v-if="hiddenPrices.length">
              <button class="hidden-toggle" @click="showHidden = !showHidden">
                {{ showHidden ? "▾" : "▸" }} {{ t("boutique.produit.vendeursMasques", { n: hiddenPrices.length }) }}
              </button>
              <div v-if="showHidden" class="hidden-list">
                <div v-for="(p, i) in hiddenPrices" :key="'h' + p.storeName + i" class="row muted">
                  <span class="row-store">{{ p.storeName }}</span>
                  <span class="row-price">{{ price(p.price, p) }}</span>
                  <button class="row-icon" :title="t('boutique.produit.reafficherVendeur')" @click="toggleStoreExcluded(p.storeName)">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" /></svg>
                  </button>
                </div>
              </div>
            </template>
          </div>

          <div v-if="product && (product.developers?.length || product.publishers?.length || product.releaseDate || product.steam)" class="infos">
            <span class="compare-label">{{ t("boutique.produit.informations") }}</span>
            <div v-if="product.developers?.length" class="info">
              <span class="info-libelle">{{ t("boutique.produit.developpeur", { n: product.developers.length }) }}</span>
              <span>{{ product.developers.join(", ") }}</span>
            </div>
            <div v-if="product.publishers?.length" class="info">
              <span class="info-libelle">{{ t("boutique.produit.editeur", { n: product.publishers.length }) }}</span>
              <span>{{ product.publishers.join(", ") }}</span>
            </div>
            <div v-if="product.releaseDate" class="info">
              <span class="info-libelle">{{ t("boutique.produit.sortie") }}</span>
              <span>{{ dateLongue(product.releaseDate) }}</span>
            </div>
            <div v-if="product.steam?.classification" class="info">
              <span class="info-libelle">{{ product.steam.classification.systeme }} {{ product.steam.classification.note }}</span>
              <span>{{ product.steam.classification.descripteurs.map(descripteur).join(", ") }}</span>
            </div>
            <div class="liens">
              <button v-if="product.steam?.site" class="lien-texte" @click="openExternal(product.steam.site!)">{{ t("boutique.produit.siteOfficiel") }} ↗</button>
              <button v-if="product.appId" class="lien-texte" @click="ouvrirSteam">{{ t("boutique.produit.pageSteam") }} ↗</button>
            </div>
          </div>

          <p class="disclaimer">{{ t("boutique.produit.avertissement", { devise: deviseFiche }) }}</p>
          <!--
            Hors euros, Instant Gaming est écarté (voir `locale::zone_euro`) : on le dit ici,
            à l'endroit où il manquerait, plutôt que de laisser croire à une offre perdue.
          -->
          <p v-if="!instantGamingPertinent" class="disclaimer">{{ t("prix.revendeurIndisponible") }}</p>
        </aside>
      </div>

      <div v-if="zoom != null && shots[zoom]" class="lightbox" @click.self="zoom = null">
        <button class="lb-close" :aria-label="t('commun.fermer')" @click="zoom = null">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
        <button v-if="shots.length > 1" class="lb-nav prev" :aria-label="t('fiche.capturePrecedente')" @click.stop="stepZoom(-1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
        </button>
        <img class="lb-img" :src="shots[zoom]" alt="" @error="hideBroken" />
        <button v-if="shots.length > 1" class="lb-nav next" :aria-label="t('fiche.captureSuivante')" @click.stop="stepZoom(1)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
        </button>
        <div v-if="shots.length > 1" class="lb-count">{{ zoom + 1 }} / {{ shots.length }}</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.sd { position: fixed; inset: 0; z-index: 110; background: var(--bg); overflow-y: auto; opacity: 0; visibility: hidden; transition: opacity 0.28s; }
.sd.open { opacity: 1; visibility: visible; }
.sd-banner { position: relative; height: 40vh; min-height: 300px; overflow: hidden; }
.sd-banner-art { position: absolute; inset: 0; }
.sd-banner-img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center 25%; display: block; }
.sd-scrim { position: absolute; inset: 0; background: linear-gradient(0deg, var(--bg) 1%, rgba(10, 7, 16, 0.25) 45%, rgba(10, 7, 16, 0.55) 100%); }
.sd-nav {
  position: absolute; top: 22px; left: 24px; z-index: 5; display: flex; gap: 8px; max-width: calc(100% - 48px);
}
.sd-back, .sd-forward {
  display: inline-flex; align-items: center; gap: 8px; min-width: 0;
  padding: 9px 16px 9px 12px; border-radius: 11px; border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(12, 8, 18, 0.5); backdrop-filter: blur(8px); color: #fff; font-weight: 600; font-size: 13.5px;
}
.sd-forward { padding: 9px 10px; flex: none; }
.sd-back svg, .sd-forward svg { width: 17px; height: 17px; flex: none; }
/* Le libellé est le nom de l'étape précédente : souvent un titre de jeu, parfois long. */
.sd-back-label { max-width: 340px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sd-header { position: absolute; bottom: 0; left: 0; right: 0; padding: 0 56px 26px; }
.sd-tag {
  display: inline-block; font-family: var(--mono); font-size: 11px; letter-spacing: 0.1em; text-transform: uppercase;
  color: #fff; padding: 4px 11px; border-radius: 99px; background: var(--accent); margin-bottom: 12px;
}
.sd-title { font-size: clamp(30px, 4.5vw, 52px); font-weight: 800; letter-spacing: -0.035em; margin: 0; color: #fff; line-height: 1; text-shadow: 0 3px 24px rgba(0, 0, 0, 0.4); }
.sd-facts { margin-top: 12px; font-family: var(--mono); font-size: 13px; color: rgba(255, 255, 255, 0.85); display: flex; gap: 6px; }

.sd-body { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 40px; padding: 34px 56px 70px; align-items: start; }
.sd-main { min-width: 0; }
.sd-loading { display: flex; align-items: center; gap: 10px; color: var(--text-faint); padding: 30px 0; }
.sd-indisponible { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; padding: 30px 0; color: var(--text-dim); }
.sd-indisponible p { margin: 0; }
.sd-indisponible .dim { color: var(--text-faint); font-size: 13px; }
.sd-sec { margin-bottom: 34px; }
.sd-sec h4 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.12em; color: var(--text-faint); font-weight: 700; margin: 0 0 16px; }
.desc { font-size: 15.5px; line-height: 1.7; color: var(--text-dim); max-width: 90ch; }
.desc.dim { color: var(--text-faint); font-style: italic; }
/* Notes, classification et possession, sous le titre (sur la bannière : toujours clair sur sombre). */
.sd-badges { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 14px; }
.badge-note {
  display: inline-flex; align-items: center; gap: 7px; padding: 6px 11px; border-radius: 9px; font-size: 12.5px;
  background: rgba(12, 8, 18, 0.7); border: 1px solid rgba(255, 255, 255, 0.16); color: #fff; cursor: pointer;
}
.badge-note:hover:not(:disabled):not(.fixe) { border-color: rgba(255, 255, 255, 0.35); }
.badge-note.fixe, .badge-note:disabled { cursor: default; }
.badge-note .avis { color: #fff; }
.badge-note.lien { color: #ffd9c9; font-weight: 600; }
.score { font-family: var(--mono); }
.score.pos { color: #6ee7a8; }
.score.mixte { color: #f0b73f; }
.score.neg { color: #ff8a80; }

.lien-texte { background: none; border: none; padding: 0; margin-top: 10px; color: var(--accent); font-size: 13px; font-weight: 600; }
.lien-texte:hover { text-decoration: underline; }

.sd-prices { position: sticky; top: 24px; display: flex; flex-direction: column; gap: 16px; }
.best { background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 20px; }
.best-top { display: flex; align-items: center; justify-content: space-between; }
.best-label { font-size: 12px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; }
.best-save { font-family: var(--mono); font-weight: 700; font-size: 13px; color: #fff; background: linear-gradient(135deg, #3ad07f, #1fa862); padding: 3px 9px; border-radius: 8px; }
.best-price { font-family: var(--mono); font-size: 40px; font-weight: 700; letter-spacing: -0.03em; color: var(--text); margin-top: 8px; line-height: 1; }
.best-was { font-size: 13px; color: var(--text-faint); margin-top: 6px; }
.buy { margin-top: 16px; width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 9px; padding: 12px; border-radius: 12px; font-size: 14.5px; font-weight: 700; background: var(--accent); color: var(--accent-ink); border: none; }
.buy:hover { background: var(--accent-hover); }
.buy svg { width: 17px; height: 17px; }
.wish {
  margin-top: 8px; width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 8px;
  padding: 10px; border-radius: 11px; font-size: 13px; font-weight: 600; cursor: pointer;
  background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim);
}
.wish:hover { color: var(--text); border-color: var(--border-strong); }
.wish.on { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 45%, transparent); background: var(--accent-soft); }
.wish svg { width: 16px; height: 16px; }

.compare { background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 14px 16px; }
.compare-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text-faint); font-weight: 700; padding: 4px 0 10px; }
.row { display: flex; align-items: center; gap: 10px; padding: 10px 0; border-top: 1px solid var(--border); }
.row-store { font-size: 13.5px; color: var(--text); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: flex; flex-direction: column; gap: 2px; }
.row-detail { font-size: 11.5px; color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; }
.infos { background: var(--surface); border: 1px solid var(--border); border-radius: 18px; padding: 14px 16px; display: flex; flex-direction: column; gap: 9px; font-size: 13px; color: var(--text-dim); }
.infos .compare-label { padding: 4px 0 2px; }
.info { display: flex; gap: 10px; }
.info-libelle { flex: 0 0 96px; color: var(--text-faint); }
.liens { display: flex; gap: 14px; flex-wrap: wrap; }
.liens .lien-texte { margin-top: 2px; }
.row-save { font-family: var(--mono); font-size: 10.5px; color: #1fa862; }
.row.oos { opacity: 0.5; }
.row.oos .row-price { text-decoration: line-through; }
.row-oos { font-family: var(--mono); font-size: 10px; font-weight: 700; color: #e0554e; text-transform: uppercase; letter-spacing: 0.03em; }
.row-price { font-family: var(--mono); font-size: 14px; font-weight: 600; color: var(--text); font-variant-numeric: tabular-nums; }
.row-buy { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; background: var(--surface-2); border: 1px solid var(--border); color: var(--text-dim); flex: none; }
.row-buy:hover { background: var(--accent); color: var(--accent-ink); border-color: transparent; }
.row-buy svg { width: 15px; height: 15px; }
.row-icon { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; background: none; border: 1px solid transparent; color: var(--text-faint); flex: none; }
.row-icon:hover { color: var(--text); border-color: var(--border); background: var(--surface-2); }
.row-icon svg { width: 15px; height: 15px; }
.row.muted { opacity: 0.55; }
.hidden-toggle { margin-top: 6px; background: none; border: none; color: var(--text-faint); font-size: 11.5px; font-family: var(--mono); padding: 6px 0 2px; cursor: pointer; }
.hidden-toggle:hover { color: var(--text-dim); }
.hidden-list .row:first-child { border-top: 1px solid var(--border); }
.disclaimer { font-size: 11px; color: var(--text-faint); line-height: 1.5; margin: 4px 2px 0; }

.spin { width: 18px; height: 18px; border-radius: 50%; border: 2px solid var(--border-strong); border-top-color: var(--accent); animation: spin 0.7s linear infinite; display: inline-block; }
@keyframes spin { to { transform: rotate(360deg); } }

.lightbox { position: fixed; inset: 0; z-index: 300; display: grid; place-items: center; padding: 4vh 5vw; background: rgba(6, 4, 10, 0.86); backdrop-filter: blur(6px); cursor: zoom-out; }
.lb-img { max-width: 100%; max-height: 92vh; border-radius: 12px; object-fit: contain; box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6); }
.lb-close { position: fixed; top: 20px; right: 22px; width: 42px; height: 42px; display: grid; place-items: center; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.18); background: rgba(14, 10, 20, 0.55); color: #fff; }
.lb-close svg { width: 20px; height: 20px; }
.lb-nav {
  position: fixed; top: 50%; transform: translateY(-50%); width: 48px; height: 48px; display: grid; place-items: center;
  border-radius: 50%; border: 1px solid rgba(255, 255, 255, 0.18); background: rgba(14, 10, 20, 0.55); color: #fff; cursor: pointer;
}
.lb-nav:hover { background: rgba(40, 30, 55, 0.85); border-color: rgba(255, 255, 255, 0.35); }
.lb-nav.prev { left: 20px; }
.lb-nav.next { right: 20px; }
.lb-nav svg { width: 24px; height: 24px; }
.lb-count {
  position: fixed; bottom: 22px; left: 50%; transform: translateX(-50%); font-family: var(--mono); font-size: 13px; color: #fff;
  background: rgba(14, 10, 20, 0.6); padding: 6px 12px; border-radius: 99px; border: 1px solid rgba(255, 255, 255, 0.14);
}

@media (max-width: 980px) {
  .sd-body { grid-template-columns: minmax(0, 1fr); }
  .sd-prices { position: static; }
}
@media (max-width: 820px) {
  .sd-header, .sd-body { padding-left: 22px; padding-right: 22px; }
}
</style>
