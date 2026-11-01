/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** « 1 » dans la version web (`.env.web`). Voir `lib/mode.ts`. */
  readonly VITE_TORII_WEB?: string;
  /** Adresse de l'API, pour essayer la version web sur des services locaux. */
  readonly VITE_TORII_API?: string;
  /** Adresse du Worker de la Boutique, pour la même raison. */
  readonly VITE_TORII_STORE?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
