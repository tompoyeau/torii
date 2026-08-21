# Torii ⛩️

> Nom de code du dépôt : `ludo` (crate Rust, package npm). Le produit s'appelle **Torii**,
> en référence aux portails 鳥居 des sanctuaires japonais — la porte d'entrée vers tes jeux.

Bibliothèque de jeux multi-plateformes — une alternative à Playnite avec une interface
moderne. Application **Tauri 2 + Vue 3 + TypeScript**, pour Windows.

Agrège les jeux **installés** (fichiers locaux) et **possédés** (comptes en ligne),
y compris toute la **bibliothèque familiale Steam**.

## Fonctionnalités

- **Deux modes d'affichage**
  - **Bureau** — barre latérale + grille de jaquettes, orienté souris/clavier.
  - **Salon** — interface « canapé » cinématique (grandes tuiles, rangées horizontales,
    hero en carrousel), **fenêtrée et pilotable à la souris** (pas un plein écran exclusif).
- **Vue détail** par jeu (bannière, temps de jeu, statut, — description/genre/captures à venir).
- **Filtres** : Tous, Mes jeux, Famille, Récents, Favoris, Installés, et par plateforme.
- **Thème** clair / sombre, jaquettes réelles (CDN Steam) avec repli en dégradé généré.
- **Lancement** direct des jeux (`steam://`, URI Epic, exécutable GOG/manuel).

## Sources de données

| Source | Méthode | Donne |
|---|---|---|
| **Steam installés** | registre + `libraryfolders.vdf` + `appmanifest_*.acf` | jeux installés, taille, dernière partie |
| **Epic installés** | manifestes JSON `ProgramData\Epic\…\*.item` | jeux installés |
| **GOG installés** | registre `HKLM\…\GOG.com\Games` | jeux installés |
| **Manuel** | `manual_games.json` (dossier config) | jeux ajoutés à la main |
| **Steam possédés + famille** | login intégré → cookie → **WebAPIToken** → `GetSharedLibraryApps` | toute la biblio familiale (jeux, noms, temps de jeu) |

### Connexion Steam (sans clé API)

Le bouton « Se connecter avec Steam » ouvre la **fenêtre de login officielle** ; on récupère
le cookie de session (store + communauté), on en extrait le jeton **WebAPIToken** (JWT) embarqué
dans la page, puis on appelle la Web API Steam :

1. `IFamilyGroupsService/GetFamilyGroupForUser` → identifiant du groupe familial,
2. `IFamilyGroupsService/GetSharedLibraryApps` → **tous les jeux de la famille** (les tiens +
   ceux partagés par tes proches), avec noms et temps de jeu — jeux uniquement, pas de DLC.

Repli sur `GetOwnedGames` (tes jeux seuls) si tu n'es pas dans une famille.
Une **clé API** reste disponible en option avancée dans les réglages.

> ⚠️ La récupération des jeux **possédés** nécessite que « Détails du jeu » soit **public**
> dans la confidentialité Steam.

## Prérequis

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- Windows : « Desktop development with C++ » (MSVC) + WebView2 (présent sur Win 11)

## Démarrer

```bash
npm install
npm run tauri dev      # application desktop complète
npm run dev            # frontend seul dans un navigateur (données fictives de secours)
```

## Architecture

### Frontend (`src/`)

```
src/
├── types.ts                 # Game, GameDto, Platform, LibraryFilter…
├── data/                    # games.ts (fetchGames + fusion des doublons + mock), platforms.ts
├── lib/                     # tauri.ts (pont commandes), covers.ts (dégradés, dates)
├── composables/             # useLibrary (scan/filtre), useUi (mode/détail/réglages), useTheme
└── components/              # Bureau*, Salon*, GameCard, GameDetail, Sidebar, TopBar, SettingsPanel
```

`fetchGames()` appelle la commande `scan_library` sous Tauri, sinon renvoie des données
fictives (utile pour `npm run dev`).

### Natif (`src-tauri/src/`)

```
src-tauri/src/
├── lib.rs                   # commandes Tauri + fenêtre de login Steam
├── models.rs                # GameDto, GameMeta (sérialisés vers le front)
├── platforms/               # scan des jeux INSTALLÉS : steam, epic, gog, manual + agrégation
├── accounts/                # jeux POSSÉDÉS : secrets (credentials.json), steam (session/famille)
└── metadata/                # enrichissement en ligne (Steam Store) — cf. limites ci-dessous
```

### Commandes Tauri

| Commande | Rôle |
|---|---|
| `scan_library` | agrège installés + possédés + manuels (et mémorise le résultat) |
| `cached_library` | dernier scan relu du disque : affichage immédiat au démarrage |
| `launch_game` | lance un jeu (steam:// / URI Epic / exe) |
| `connect_steam` / `disconnect_steam` | login intégré / déconnexion |
| `get_settings` | état des comptes connectés |
| `set_steam_key` | clé API Steam (option avancée) |
| `add_manual_game` / `remove_manual_game` | jeux manuels |
| `enrich_game` | métadonnées d'un jeu, à l'ouverture de sa fiche |
| `enrich_igdb` | métadonnées descriptives de toute la bibliothèque (IGDB) |

### Exemples de test (dossier `src-tauri`)

```bash
cargo run --example scan        # jeux installés détectés
cargo run --example watch       # jeux détectés en cours d'exécution
cargo run --example community   # jeux possédés + famille (via session stockée)
cargo run --example owned <CLE> # jeux possédés via clé API
```

## Limites connues / à faire

- **Genre / description / captures** : l'enrichissement en masse (`appdetails`) est **désactivé**
  (Steam bride l'API, inefficace sur des centaines de jeux). À réintroduire en **« à la demande »**,
  au moment d'ouvrir la vue détail d'un jeu.
- **Epic / GOG possédés** : connexion de compte non implémentée (seuls les installés remontent).
- **Temps de jeu** : disponible pour Steam ; pas encore pour Epic/GOG.
- **Comparateur de prix** (idée) : croiser la wishlist avec un agrégateur type CheapShark / IsThereAnyDeal.
- Le jeton de session Steam expire (~semaines) : prévoir une reconnexion quand il devient invalide.

## Inspiration

Approche des jeux possédés et de la bibliothèque familiale inspirée de
[Playnite](https://github.com/JosefNemec/Playnite) (open source, MIT).
