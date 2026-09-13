/**
 * Données fictives de la démo en ligne (et de `npm run dev`).
 *
 * 🔑 POURQUOI ELLES SONT TRADUITES. La démo est la première chose qu'un visiteur étranger
 * verra de Torii : une interface anglaise posée sur des genres, des succès et une
 * description en français dirait « traduction à moitié faite » avant même le premier clic.
 * Les titres de jeux, studios et boutiques restent des noms propres — sauf quand le jeu
 * porte lui-même un nom différent selon la langue (« Les Sims 4 »).
 */
export default {
  description:
    "Une expérience marquante saluée par la critique. Explore un monde façonné à la main, affine ton style de jeu au fil des heures et laisse la bande-son t'emporter. Chaque session te rapproche de la fin — ou d'un nouveau départ.",
  descriptionBoutique:
    "Un titre encensé par la critique. Explore un monde façonné à la main, affine ton style au fil des heures et laisse-toi porter par sa direction artistique.",
  studioFictif: "Studio Fictif",
  sims4: "Les Sims 4",

  genres: {
    action: "Action",
    actionAventure: "Action-Aventure",
    actionRpg: "Action-RPG",
    aventure: "Aventure",
    bacASable: "Bac à sable",
    cooperatif: "Coopératif",
    deckBuilding: "Deck-building",
    gestion: "Gestion",
    horreur: "Horreur",
    jeuDeRole: "Jeu de rôle",
    metroidvania: "Metroidvania",
    plateforme: "Plateforme",
    rogueLite: "Rogue-lite",
    reflexion: "Réflexion",
    simulation: "Simulation",
    tir: "Tir",
  },

  // Succès fictifs de la fiche d'exemple (Baldur's Gate 3). Les dates imitent le texte
  // que Steam renvoie lui-même dans cette langue.
  succes: {
    nom1: "Fuite de l'Avernus",
    desc1: "Prendre le contrôle du nautiloïde et vous enfuir des Enfers.",
    date1: "Débloqué le 30 août 2023 à 10h28",
    nom2: "De Charybde en Scylla",
    desc2: "Quitter l'acte 1 pour vous rendre dans un lieu bien plus sombre.",
    date2: "Débloqué le 19 nov. 2023 à 8h24",
    nom3: "La cité vous attend",
    desc3: "Quitter l'acte 2 pour rejoindre la Porte de Baldur.",
    date3: "Débloqué le 26 janv. 2024 à 14h31",
    nom4: "Tout est bien qui finit bien",
    desc4: "Terminer le jeu.",
    nom5: "L'appel du sang",
    desc5: "Boire le sang d'un ennemi vaincu.",
  },
} as const;
