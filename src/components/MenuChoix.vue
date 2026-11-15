<script setup lang="ts" generic="Cle extends string">
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";

/**
 * Un bouton et sa liste déroulante, pour UN choix parmi plusieurs (plateforme, tri). Le
 * cousin à choix unique de `MenuCategories`, avec les mêmes manières : clic dehors et Échap
 * le referment, flèches haut/bas pour passer d'une option à l'autre.
 *
 * Emplacements : `icone` (dans le bouton), `option` (l'icône d'une option, reçoit `cle`),
 * `pied` (sous la liste, pour une action annexe).
 */
export interface OptionChoix<C extends string = string> {
  cle: C;
  nom: string;
  /** Nombre de jeux, quand on le connaît. */
  n?: number;
}

const props = defineProps<{
  options: OptionChoix<Cle>[];
  /** L'option retenue (aucune : `null`). */
  choisie: Cle | null;
  /** Texte du bouton. */
  libelle: string;
  /** Nom du menu pour les lecteurs d'écran, quand le libellé ne le dit pas (« Trier »). */
  nomMenu: string;
  /** Un choix restreint la vue : le bouton passe en couleur d'accent. */
  actif?: boolean;
  /** Nombre affiché dans le bouton, à côté du libellé. */
  n?: number;
}>();

const emit = defineEmits<{ choisir: [cle: Cle] }>();

const ouvert = ref(false);
const racine = ref<HTMLElement | null>(null);
const bouton = ref<HTMLElement | null>(null);
const liste = ref<HTMLElement | null>(null);

async function basculer() {
  ouvert.value = !ouvert.value;
  if (!ouvert.value) return;
  await nextTick();
  // Le focus sur l'option retenue : les flèches partent de là.
  (liste.value?.querySelector<HTMLElement>(".opt.on") ?? liste.value?.querySelector<HTMLElement>(".opt"))?.focus();
}

function fermer(rendreFocus: boolean) {
  ouvert.value = false;
  if (rendreFocus) bouton.value?.focus();
}

function choisir(cle: Cle) {
  emit("choisir", cle);
  fermer(true);
}

function deplacer(e: KeyboardEvent, sens: 1 | -1) {
  const items = [...(liste.value?.querySelectorAll<HTMLElement>(".opt, .pied-action") ?? [])];
  if (!items.length) return;
  e.preventDefault();
  const i = items.indexOf(document.activeElement as HTMLElement);
  items[Math.max(0, Math.min(items.length - 1, i + sens))].focus();
}

function surClic(e: MouseEvent) {
  if (ouvert.value && !racine.value?.contains(e.target as Node)) fermer(false);
}
/** Échap ferme le menu d'abord : `useRaccourcis` ignore un Échap déjà pris. */
function surTouche(e: KeyboardEvent) {
  if (e.key === "Escape" && ouvert.value) {
    e.preventDefault();
    fermer(true);
  }
}
onMounted(() => {
  document.addEventListener("click", surClic);
  document.addEventListener("keydown", surTouche);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", surClic);
  document.removeEventListener("keydown", surTouche);
});
</script>

<template>
  <div ref="racine" class="choix">
    <button
      ref="bouton"
      class="chip choix-btn"
      :class="{ active: props.actif }"
      aria-haspopup="menu"
      :aria-expanded="ouvert"
      :title="nomMenu"
      @click.stop="basculer"
    >
      <slot name="icone" />
      <span class="libelle">{{ libelle }}</span>
      <span v-if="n !== undefined" class="n">{{ n }}</span>
      <svg class="caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
    </button>

    <div v-if="ouvert" class="menu" role="menu" :aria-label="nomMenu">
      <div ref="liste" class="liste" @keydown.down="deplacer($event, 1)" @keydown.up="deplacer($event, -1)">
        <button
          v-for="o in options"
          :key="o.cle"
          class="opt"
          :class="{ on: o.cle === choisie }"
          role="menuitemradio"
          :aria-checked="o.cle === choisie"
          @click="choisir(o.cle)"
        >
          <span class="ico" aria-hidden="true"><slot name="option" :cle="o.cle" /></span>
          <span class="nom">{{ o.nom }}</span>
          <span v-if="o.n !== undefined" class="n">{{ o.n }}</span>
          <svg v-if="o.cle === choisie" class="coche" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
        </button>
        <div v-if="$slots.pied" class="pied" @click="fermer(false)"><slot name="pied" /></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.choix { position: relative; }
.choix-btn { display: inline-flex; align-items: center; gap: 7px; max-width: 240px; }
.choix-btn :deep(svg), .choix-btn :deep(.platform-icon) { width: 14px; height: 14px; flex: none; }
.choix-btn .caret { width: 13px; height: 13px; margin-left: -1px; opacity: 0.7; }
.libelle { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.n { font-family: var(--mono); font-size: 10.5px; color: var(--text-faint); font-variant-numeric: tabular-nums; }
.choix-btn.active {
  background: var(--accent-soft); color: var(--text);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent); font-weight: 600;
}
.choix-btn.active .n { color: var(--accent); }

.menu {
  position: absolute; top: calc(100% + 6px); left: 0; z-index: 40; min-width: 220px; width: max-content; max-width: 300px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 13px;
  box-shadow: var(--shadow-hero); overflow: hidden;
}
/* Le menu de droite (tri) s'ouvre vers la gauche, pour ne pas sortir de la fenêtre. */
.choix.a-droite .menu { left: auto; right: 0; }
.liste { max-height: 380px; overflow-y: auto; padding: 6px; display: flex; flex-direction: column; gap: 1px; }
.opt {
  display: flex; align-items: center; gap: 10px; padding: 8px 9px; border-radius: 9px;
  background: none; border: none; color: var(--text); font-size: 13px; text-align: left;
  width: 100%; cursor: pointer;
}
.opt:hover, .opt:focus-visible { background: var(--surface-2); outline: none; }
.opt.on { font-weight: 600; }
.ico { width: 16px; height: 16px; flex: none; display: grid; place-items: center; }
.ico:empty { display: none; }
.ico :deep(svg), .ico :deep(.platform-icon) { width: 16px; height: 16px; }
.nom { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.coche { width: 14px; height: 14px; flex: none; color: var(--accent); }
.pied { margin-top: 4px; padding-top: 4px; border-top: 1px solid var(--border); }
</style>
