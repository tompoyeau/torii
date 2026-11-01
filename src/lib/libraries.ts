import type { LibGame } from "../types";

/**
 * Fusionne les appareils d'une personne : un jeu présent sur deux PC ne compte qu'une fois.
 * Deux PC n'ont pas la même bibliothèque, et le serveur rend un instantané par appareil :
 * « la bibliothèque de quelqu'un » est donc leur union, faite côté client.
 */
export function fusionnerAppareils(snapshots: LibGame[][]): LibGame[] {
  const parCle = new Map<string, LibGame>();
  for (const liste of snapshots) {
    for (const jeu of liste) {
      const deja = parCle.get(jeu.key);
      if (!deja) {
        parCle.set(jeu.key, { ...jeu, platforms: [...jeu.platforms] });
        continue;
      }
      // Le même jeu sur deux machines : on cumule les launchers, et on récupère la
      // jaquette si l'un des deux appareils l'avait et pas l'autre.
      for (const p of jeu.platforms) if (!deja.platforms.includes(p)) deja.platforms.push(p);
      if (!deja.cover && jeu.cover) deja.cover = jeu.cover;
      if (!deja.steamAppId && jeu.steamAppId) deja.steamAppId = jeu.steamAppId;
      // Même règle que côté Rust : une seule copie possédée suffit à faire du jeu le sien.
      deja.familyShared = !!deja.familyShared && !!jeu.familyShared;
    }
  }
  return [...parCle.values()].sort((a, b) => a.title.localeCompare(b.title, "fr"));
}
