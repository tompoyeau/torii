import { computed } from "vue";
import { etiquetteIntl, langueParDefaut, type Langue } from "../i18n";
import { nomDeRegion, REGIONS, type CodeRegion } from "../i18n/regions";
import { usePreferences } from "./usePreferences";

/** Les choix de langue et de région, tels que les proposent les Paramètres et le tuto. */
export function useLangueRegion() {
  const { prefs } = usePreferences();

  const langueChoisie = computed<Langue | "system">(() => prefs.language ?? langueParDefaut());
  function choisirLangue(choix: Langue | "system") {
    prefs.language = choix;
  }

  const regionChoisie = computed<CodeRegion | "system">(() => prefs.region ?? "system");
  function choisirRegion(choix: string) {
    prefs.region = choix === "system" ? null : (choix as CodeRegion);
  }

  /** Les régions, triées par leur nom dans la langue affichée — pas par code pays. */
  const regionsTriees = computed(() =>
    [...REGIONS].sort((a, b) =>
      nomDeRegion(a.code).localeCompare(nomDeRegion(b.code), etiquetteIntl.value),
    ),
  );

  return { langueChoisie, choisirLangue, regionChoisie, choisirRegion, regionsTriees };
}
