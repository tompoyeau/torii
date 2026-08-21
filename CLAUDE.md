# CLAUDE.md — Ludo

Launcher de jeux type Playnite, plus joli. **Tauri 2 + Vue 3 + TypeScript**, Windows.
Agrège jeux **installés** (fichiers locaux) et **possédés** (comptes en ligne), dont toute
la **bibliothèque familiale Steam**. Voir `README.md` pour la vue d'ensemble utilisateur.

## Commandes

```bash
npm run tauri dev                    # app complète (rebuild Rust auto au save)
npm run dev                          # frontend seul (données mock de secours)
npx vue-tsc --noEmit                 # type-check front (à lancer après toute modif front)

# Depuis src-tauri/ :
cargo check                          # compile-vérifier le natif
cargo test --lib                     # tests unitaires (parseurs)
cargo run --example scan             # jeux installés détectés
cargo run --example community        # jeux possédés + famille (via session stockée)
```

⚠️ **`cargo` n'est pas dans le PATH par défaut** dans un shell frais :
`export PATH="$HOME/.cargo/bin:$PATH"` avant tout cargo.

## Architecture

- **Front `src/`** : `types.ts`, `data/` (games.ts = fetchGames/enrichGames + mock, platforms.ts),
  `lib/` (tauri.ts pont commandes, covers.ts), `composables/` (useLibrary, useUi, useTheme),
  `components/` (Bureau*, Salon*, GameCard, GameDetail, Sidebar, TopBar, SettingsPanel).
- **Natif `src-tauri/src/`** : `lib.rs` (commandes + fenêtre login), `models.rs` (GameDto/GameMeta),
  `platforms/` (scan INSTALLÉS : steam/epic/gog/manual + agrégation `scan_all`),
  `accounts/` (POSSÉDÉS : secrets.rs = credentials.json, steam.rs = session/famille),
  `metadata/` (enrichissement Steam Store).
- Flux : front `fetchGames()` → commande `scan_library` → `platforms::scan_all` fusionne
  installés ∪ possédés (comptes) ∪ manuels, dédoublonné par id `"<plateforme>:<cible>"`.
- 🔑 `merge_owned` : un jeu **installé** Epic/GOG (scan local des manifestes/registre) n'a PAS de
  jaquette — seul le compte en ligne l'a. Donc merge_owned backfille `cover_url`/`hero_url` (+ playtime,
  last_played) depuis l'entrée possédée. Sinon Fortnite & co (installés) restent sans image. (Steam
  installé a déjà sa cover CDN par appid, pas concerné.)

## Conventions

- Rust : `GameDto`/`GameMeta` en `#[serde(rename_all = "camelCase")]` (le front reçoit du camelCase).
  Champs optionnels avec `#[serde(default)]`. `GameDto` dérive `Default` → construire avec
  `GameDto { ..Default::default() }`.
- Vue : `<script setup lang="ts">`, styles scoped, tokens CSS globaux dans `style.css`
  (thème clair/sombre via variables). Le modèle `Game` a beaucoup de champs **optionnels**
  (un scan local ne fournit pas genre/année/desc) → toujours garder les gardes `v-if`.
- Le front doit rester fonctionnel hors Tauri (navigateur) : les appels `invoke` sont
  try/catch → repli sur données mock. Ne pas casser ce chemin.
- Config/secrets stockés dans le dossier config de l'app (`%APPDATA%\com.tompo.ludo\`).
- 🔒 **Les identifiants sont chiffrés au repos** : `accounts/secrets.rs` écrit `credentials.dat`
  via **DPAPI** (`CryptProtectData`, portée utilisateur + entropie applicative), en FFI directe
  sur `crypt32` (pas de nouvelle dépendance). Un ancien `credentials.json` en clair est migré
  puis **supprimé** au premier chargement. Un blob illisible (autre compte Windows, fichier
  copié) → identifiants vides, l'utilisateur se reconnecte ; on ne supprime rien.
- **Bibliothèque affichée avant le réseau** : `scan_library` persiste son résultat
  (`platforms/library_cache.rs` → `library_cache_v1.json`), et `cached_library` le relit
  instantanément. Le front (`useLibrary.load`) affiche ce cache puis le remplace par le scan
  frais — d'où l'écran de démarrage qui s'efface en ~0,3 s au lieu d'attendre Steam/GOG/Epic.
  Le repli cache ne joue qu'au **premier** chargement (pas sur `reload()`), et n'écrase jamais
  un scan frais déjà arrivé.
- 🔑 **Revendeurs masqués : le filtre est côté RUST.** La vitrine, la recherche et la wishlist
  ne renvoient qu'UN prix par jeu (la meilleure offre) — le front n'a donc rien à re-filtrer et
  ne le peut pas. `metadata::store::cheapest(entry, excluded)` est le seul endroit qui choisit
  une offre ; `deals()` re-tarife en plus les jeux dont l'offre ITAD vient d'un revendeur masqué
  (un appel groupé). Seule la fiche produit reçoit toutes les offres et filtre côté front (elle
  affiche les masquées dans un repli « réafficher »).

## Pièges de plomberie (NE PAS refaire les erreurs)

- 🔑 **Écriture wishlist Steam : `store.steampowered.com/api/addtowishlist` est MORT.**
  Mesuré : il répond `200 {"success":false,"wishlistCount":0}` même avec un cookie store
  fraîchement régénéré et correctement typé (`aud: ["web:store"]`), donc l'échec est
  silencieux. L'écriture passe par `IWishlistService/AddToWishlist/v1` et
  `RemoveFromWishlist/v1` (form `access_token` + `appid`, réponse
  `{"response":{"wishlist_count":N}}`) — la même famille d'API que la lecture
  `GetWishlist`, avec le même WebAPIToken (`accounts::steam_access_token`). Vérifié en
  aller-retour réel : ajout 46→47, retrait 47→46.

- 🔑 **La capsule Steam `library_600x900` n'existe PAS pour tout jeu** : mesuré sur une
  wishlist réelle, **22 jeux sur 43** renvoient un 404 (nouveautés, jeux non sortis). Toute
  jaquette construite depuis `cdn…/{appid}/library_600x900.jpg` doit donc avoir un repli.
  `WishlistItem` porte `cover_fallback_url` (boxart ITAD, déjà présente dans la réponse
  `games/lookup/v1` — zéro appel de plus) et le front enchaîne capsule → boxart → dégradé.
- **Le fond des cartes à jaquette est mutualisé** dans `style.css` (`.cover-card` : rayon,
  ombre, survol, image, voile, titre incrusté, pastille de remise). Bibliothèque, Boutique
  et Wishlist l'utilisent ; chaque vue n'ajoute que ce qui lui est propre. Ne pas redéclarer
  `.cover`/`.cover-title` en scoped dans une vue : les trois grilles avaient divergé comme ça
  (rayon 12 vs 16, pas d'ombre, survol deux fois plus court, titre absent de la jaquette).

- **Une commande Tauri qui touche au réseau ou au disque DOIT être `async` + `spawn_blocking`.**
  Une commande synchrone s'exécute sur le **thread principal** (`body_blocking` dans
  `tauri-macros`) : elle y bloque la boucle d'événements → fenêtre « ne répond pas », tray
  inerte, et toute opération `run_on_main_thread` mise en file derrière. C'était le cas de
  `scan_library` (scan complet, réseau des comptes compris) jusqu'à la correction.
- **Un scan n'est pas une lecture locale.** `platforms::scan_all` rejoue toute la séquence
  réseau des comptes, dont un refresh GOG qui **fait tourner** le refresh token. Ne jamais
  l'appeler pour « savoir quels jeux existent » : `scan_library` mémorise son résultat dans
  l'état `LastScan`, que `enrich_igdb` réutilise. Deux scans concurrents = token GOG grillé.
- **`@tauri-apps/api` s'importe sans erreur dans un navigateur nu** : c'est `invoke` qui échoue
  (il lit `window.__TAURI_INTERNALS__`). Le pont `lib/tauri.ts` teste donc ce global
  (`hasTauriRuntime`) pour distinguer « hors Tauri » (repli silencieux) d'une commande qui a
  vraiment échoué (`console.error`) — les envelopper dans un même `try/catch` déguisait les
  erreurs backend en mode preview, mocks compris.
- **Les cinq flux de login passent par les mêmes briques** (`open_login_window`,
  `probe_login_window`, `poll_login_window`, `wait_for_capture`, `capture_channel`,
  `forget_credentials` dans `lib.rs`). Ajouter un launcher = fournir son URL, son script de
  capture et sa sonde, pas recopier 60 lignes.

## Pièges Steam déjà résolus (NE PAS refaire les erreurs)

- **`connect_steam` doit être `async`** : une commande sync bloque le thread principal → la
  WebView de login reste **blanche**. La lecture des cookies WebView2 doit se faire **sur le
  thread principal** (`run_on_main_thread` + canal), le polling sur `spawn_blocking`.
- **`dynamicstore/userdata` renvoie une 302** qui pose les cookies `steamCountry`/`Steam_Language`
  requis sur la requête rejouée. Avec `ureq::builder().redirects(0)`, la 302 revient en **`Ok`**
  (statut 3xx, pas `Err`) → lire `resp.all("set-cookie")`, réinjecter, rejouer (cf. `fetch_text`).
- **`ISteamApps/GetAppList` keyless est RETIRÉ** (404/403). Ne pas s'en servir pour les noms.
- **`?xml=1` de la page de jeux est MORT** (302 login, même profil public). La page React
  n'embarque que ~8 jeux récents. **Solution retenue** : extraire le JWT `WebAPIToken` de la page
  (cf. `extract_webapi_token`) et appeler la Web API avec `access_token=<jwt>`.
- **Jeux possédés + famille** : `IFamilyGroupsService/GetFamilyGroupForUser` → `GetSharedLibraryApps`.
  `app_type` y est **numérique** (1 = jeu ; tout est déjà jeux-only, pas de DLC).
  `rt_playtime` est en **minutes**. `owner_steamids` = strings (steamid > 2^53).
- **`appdetails` ne supporte pas le multi-appid** (renvoie seulement le 1er) et est **bridé**
  (~1 req/1.5s) → inutilisable pour nommer/typer des centaines de jeux. D'où l'approche WebAPI.
- Le profil Steam doit avoir « Détails du jeu » **public** pour que les possédés remontent.
- **DURÉES D'EXPIRATION (mesurées sur tokens réels)** : Steam cookie web `steamLoginSecure` = **~24 h**
  (JWT, pas d'auto-refresh → biblio famille casse chaque jour) ; GOG access = 1 h / refresh long
  (jusqu'à révocation) ; Epic access = 36 h / refresh = **365 j**. GOG+Epic OK (on redérive l'access
  à chaque scan via le refresh token stocké) ; Steam était le seul à casser quotidiennement.
- **Refresh token Steam (~200 j) — capture ⏳ EN VALIDATION** : `connect_steam` injecte
  `STEAM_CAPTURE_JS` (script passif hookant fetch/XHR) qui extrait le `nonce` (= refresh token) de la
  requête `login.steampowered.com/jwt/finalizelogin` et le remonte via `document.title` →
  `.on_document_title_changed` → `steam_refresh_token`. Rafraîchissement : `steam::refresh_web_cookie`
  (`IAuthenticationService/GenerateAccessTokenForApp` → nouveau cookie `steamLoginSecure=<id>%7C%7C<at>`),
  utilisé dans `owned_games` quand la session communautaire est vide (`refresh_steam_community`, persiste
  le cookie frais). ⚠️ NON testé (aucun refresh token Steam capté encore) : à valider au 1er login réel,
  et `GenerateAccessTokenForApp` peut renvoyer `AccessDenied` selon le type de token → repli possible
  vers le flux complet `finalizelogin` (transfer_info + settoken). Piège probable : titre trop long
  pour un JWT ~1000 c → vérifier que le token n'est pas tronqué.

## Comptes possédés — GOG (fait)

- **OAuth GOG Galaxy** (`accounts/gog.rs`) : client public `46899977096215655` (client_id +
  client_secret, les mêmes que tous les outils GOG open source — identifient l'app, pas l'user).
  `connect_gog` ouvre une `WebviewWindow` sur `auth.gog.com/auth`, **poll l'URL** (pas un cookie)
  jusqu'à la redirection `embed.gog.com/on_login_success?code=…`, échange le code contre des jetons.
- On stocke **seulement le refresh token** (`gog_refresh_token`) : l'access token expire en ~1 h,
  redérivé à chaque sync. GOG **fait tourner** le refresh token → `owned_games` persiste le nouveau.
- Jeux via `embed.gog.com/account/getFilteredProducts?mediaType=1&page=N` (Bearer token, paginé) :
  jeux-only, titres inclus. Jaquette portrait = `https:{image}_glx_vertical_cover.jpg` (transform CDN
  vérifiée). `image` de getFilteredProducts = préfixe `//images…/hash` sans extension.
- **Temps de jeu GOG** ✅ testé (compte réel : Witcher3 74h, Cyberpunk 62h) : `owned_games`
  fait UN appel bulk `gameplay.gog.com/users/{user_id}/statistics` (Bearer) → objet clé=game_id,
  valeur `{playtime (minutes), last_session (ISO), achievements?}`. `user_id` vient de la réponse
  du refresh token (peut être string OU number > 2^53). `last_session` ISO→Unix via `parse_iso_to_unix`
  (algorithme days_from_civil, testé). Appliqué par id produit dérivé de `GameDto.id` (« gog:<id> »).
- Lancement d'un jeu GOG **possédé non installé** (id produit numérique) → `goggalaxy://openGameView/{id}`.
- 🔑 Enrichissement GOG : utiliser l'id produit tiré de `game.id` (« gog:<id> »), PAS `launch_target`
  (= chemin exe pour un jeu installé). Sinon les jeux GOG installés ne s'enrichissent pas.
- 🔑 PIÈGE RÉSOLU (logins sociaux) : les boutons Google/Steam/Discord de GOG sont des
  `<a target="Login" class="ext-acc-*-popup-login">` → ils appellent `window.open`. Une
  `WebviewWindow` Tauri **ignore les popups par défaut** → le clic ne fait rien. Fix :
  `.on_new_window(|_,_| tauri::webview::NewWindowResponse::Allow)` sur le builder (WebView2
  crée la popup en partageant la session). Vaudra aussi pour les logins sociaux Epic.
- ⏳ NON testé end-to-end (login réel requis, `npm run tauri dev`). Compile + type-check + carte
  SettingsPanel fonctionnelle vérifiés.

## Comptes possédés — Epic (fait, calqué sur Legendary/Heroic)

- **OAuth Epic Launcher** (`accounts/epic.rs`) : client public `34a02cf8f4414e29b15921876da36f9a`
  (+ secret), Basic auth précalculé en base64. Token endpoint
  `account-public-service-prod03.ol.epicgames.com/account/api/oauth/token` (POST form,
  `token_type=eg1`). On stocke `epic_refresh_token` (access token ~8 h, redérivé à chaque sync).
- 🔑 CAPTURE DU CODE : Epic ne redirige pas avec `?code=` dans l'URL — il renvoie un **JSON**
  `{authorizationCode}` sur `/id/api/redirect`. Donc `connect_epic` ouvre la fenêtre sur `login_url()`
  (login?redirectUrl=…/id/api/redirect?clientId=…&responseType=code), injecte `EPIC_CAPTURE_JS`
  (`initialization_script`) qui lit le code dans `document.body.innerText` et le met dans
  `document.title` = `ludo-epic:<code>`, capté via `.on_document_title_changed` → canal mpsc.
  Plus `.on_new_window(Allow)` pour les logins sociaux. Poll code/fermeture jusqu'à ~3 min.
- Jeux : `launcher-public-service-prod06…/launcher/api/public/assets/Windows?label=Live` (Bearer)
  → {appName, catalogItemId, namespace}. Filtre `namespace=="ue"` écarté. Résolution titre/jaquettes
  via `catalog-public-service-prod06…/catalog/api/shared/namespace/{ns}/bulk/items?id={id}` : jeu de
  base = catégorie **`games`** présente (🔑 filtre positif — les assets UE/Fab Marketplace ont
  `plugins`/`asset-format`, jamais `games`) ET pas de `mainGameItem` (DLC) ET pas de `mods`.
  🔑 DÉDUP par `catalogItemId` : un asset marketplace a 1 exemplaire par version de moteur (mêmes
  catalogItemId/titre, appName `_5.3`/`_5.4`…) → sans dédup, x10 doublons. Cache versionné
  `epic_catalog_cache_v2.json` (v2 = filtre `games`). 🔑 Jaquette = `keyImages` type
  **`DieselGameBoxTall`** (portrait) et **`DieselGameBox`** (paysage/hero) — PAS DieselStoreFront*.
  `id="epic:<appName>"` (dedup avec installés), `launch_target=appName`.
- ⚠️ 1 appel catalogue **par jeu** (namespaces uniques). Le user a **436 jeux/DLC Epic** → en
  séquentiel ça bloquait l'app ~2 min au démarrage (perçu comme un crash). Fix : `resolve_all` résout
  **en parallèle** (`RESOLVE_WORKERS=16` via `std::thread::scope`), CACHÉ sur disque
  `epic_catalog_cache.json` (non-jeux cachés aussi ; échec réseau non caché → réessai).
  ✅ 1er scan ~11 s (393 jeux), scans suivants ~2 s (refresh+assets, reste du cache).
- ✅ TESTÉ end-to-end sur compte réel (356 jeux, jaquettes OK). Suit le flux Legendary
  (endpoints/params vérifiés sur sa source + API sondée en live).
- **Temps de jeu Epic** ✅ testé (98/356 jeux) : `fetch_playtime` (1 appel bulk)
  `library-service…/library/api/public/playtime/account/{account_id}/all` (Bearer) → liste
  `{artifactId, totalTime}` (totalTime en **secondes**, artifactId = **appName**). `account_id` vient
  de la réponse du refresh token (ajouté à `Tokens`). Appliqué par `asset.app_name`.

## Enrichissement à la demande (fait)

- Commande `enrich_game(id, platform, launch_target, title)` → `metadata::enrich_one`, appelée à
  **l'ouverture de la vue détail**. (L'enrichissement en masse `enrich_metadata` / `enrich_covers`
  a été supprimé : plus aucun appelant depuis le passage à l'enrichissement à la demande + IGDB.) Même cache disque `metadata_cache.json`, clé = id du jeu.
- Sources par plateforme (`metadata::fetch`) : **Steam** = `steam_store::appdetails` ;
  **GOG** = `gog_store::product` (API v2 publique `api.gog.com/v2/games/{id}`, UN appel → description
  HTML nettoyée + tronquée, captures via URL templatée `{formatter}`→`product_card_screenshot_748`,
  hero = 1re capture en `1600`, développeur, année via `globalReleaseDate`, genre via `tags`) ;
  **Epic/manuel** = repli recherche Steam par titre.
- **Taille des jeux non installés** : `GameMeta.size_gb`. GOG = champ `size` de l'API v2 (en **Mo**,
  ÷1024 → Go, gratuit dans l'appel d'enrich) — testé (Witcher3 80 Go). Steam = 2e appel à l'API tierce
  publique **api.steamcmd.net** (`steam_store::install_size_gb`) : somme des `manifests.public.size`
  des dépôts Windows, hors DLC (`dlcappid`) et hors langues ≠ anglais → **estimation indicative**
  (Portal2 11.9 Go, Witcher3 44 Go). Appelée seulement si `!installed` (d'où le param `installed`
  de `enrich_game`). Appliqué si `size_gb == 0` (n'écrase pas la taille disque réelle d'un installé).
  Front : stat row « Taille », gros chiffre « sur le disque » (installé) / « taille du jeu » (sinon).
- ⚠️ Cache métadonnées **versionné** : `metadata_cache_v2.json` (incrémenter le suffixe à chaque
  évolution du schéma `GameMeta` — ici ajout `size_gb` — pour ignorer les anciennes entrées).
- Front : `useLibrary.ensureEnriched(id)` (une fois/jeu, anti-doublon `enrichedIds`, fusion réactive
  sans écraser l'existant, résout les titres « App <id> »), déclenché par un `watch` sur
  `selectedGameId` dans GameDetail.vue. `enrichingId` → état « Chargement des détails… ».
- **Visionneuse de captures** (GameDetail.vue) : clic sur une capture → lightbox plein écran
  (`.lightbox`, `zoomIndex`), flèches ←/→ pour naviguer, Échap/clic fond pour fermer. Les captures
  GOG sont récupérées en `product_card_screenshot_748_2x` pour un zoom net.

## Liste d'exclusion / jeux masqués (fait)

- Masquer un jeu non désiré ou un doublon cross-plateforme. Persisté dans `hidden.json`
  (`platforms/id_set.rs` : `HIDDEN.load(dir)` / `HIDDEN.set(dir, id, on)` → liste d'ids ; le même
  module sert aux favoris `FAVORITES` et aux revendeurs masqués `EXCLUDED_STORES`). `scan_all` marque
  `GameDto.hidden` d'après cette liste. Commande `set_game_hidden(id, hidden)` → liste à jour.
- Front : `Game.hidden`, `useLibrary.setHidden(id, hidden)` (maj réactive + persiste). `matches()`
  cache les masqués de toutes les vues SAUF le filtre `"hidden"` (qui ne montre qu'eux). Bouton
  œil-barré au survol des cartes (`GameCard`, `@click.stop`), filtre sidebar « Masqués » (visible
  si count>0), compteurs sidebar excluent les masqués. Testé : hide/unhide roundtrip (Rust) + UI.

## Fusion des doublons cross-plateforme (fait)

- Un même jeu possédé sur plusieurs launchers → **une seule carte** avec plusieurs `sources`
  (`Game.sources: {platform, launchTarget, installed}[]`). Front-only : `mergeDuplicates()` dans
  `data/games.ts`, appelé dans `useLibrary.load` après `fetchGames`.
- Rapprochement par **titre normalisé strict** (`titleKey` : minuscules + alphanumérique seul, retire
  ™®/ponctuation/espaces). Fusion seulement si ≥2 plateformes distinctes et clé ≥3 car (anti-collision
  sur titres courts). Ex : « The Witcher 3: Wild Hunt » (Steam) + « THE WITCHER 3: WILD HUNT™ » (Epic)
  → même clé. ⚠️ Tradeoff assumé : 2 jeux différents au même titre exact fusionneraient (rare).
- Carte primaire = installé d'abord, puis avec jaquette, puis ordre plateforme. `installed` = un des
  sources installé ; `hoursPlayed` = max ; `sizeGb` = source installée. `matches()`/compteurs sidebar :
  un jeu fusionné apparaît sous CHACUNE de ses plateformes (`sources.some`).
- Détail : bouton « Jouer » avec chevron → menu **« Jouer depuis… »** (une entrée par source, badge
  installé/non). `launchSource(platform, target)`. Carte : sous-titre « Steam · GOG · … ». Testé UI + algo.

## Riot Games (fait — installés uniquement)

- Riot = jeux **gratuits** → pas de bibliothèque possédée, pas d'API tierce de librairie/temps de jeu
  (RSO réservé aux devs approuvés). Donc **scan installé + lancement** seulement (pas de login).
- `platforms/riot.rs` : lit `%ProgramData%\Riot Games\RiotClientInstalls.json` (`associated_client` =
  dossiers installés, `rc_live`/`rc_default` = chemin `RiotClientServices.exe`). Catalogue fixe `KNOWN`
  (marqueur chemin → titre + id produit) : valorant, league_of_legends, bacon (LoR), teamfighttactics.
  `id="riot:<product>"`, `launch_target=<product>`, taille = `dir_size`. Testé : LoL 37.6 Go + Valorant 31 Go.
- Lancement (`platforms::launch` cas `"riot"`) : `RiotClientServices.exe --launch-product=<id> --launch-patchline=live`.
- ⚠️ Pas de jaquette (aucune source en ligne ; gradient de secours) ni temps de jeu. Front : PlatformId
  `"riot"`, couleur `--riot` (#ff4655), icône PlatformIcon, entrée sidebar, `PLATFORM_ORDER`.

## Tri de la bibliothèque (fait)

- Puces Bureau fonctionnelles : **Récemment joué** (`lastPlayedAt` desc), **A → Z** (`title`
  localeCompare fr), **Temps de jeu** (`hoursPlayed` desc). État `sort: SortKey` dans `useUi`
  (défaut "recent"), `sortGames()` dans BureauView appliqué après `filtered()`.
- 🔑 `lastPlayedAt` (Unix, pour le tri) ajouté à `Game` en plus de `lastPlayed` (chaîne d'affichage) :
  `fromDto` gardait seulement la chaîne relative. Fusion : `lastPlayedAt`/`hoursPlayed` = max des sources.

## Ubisoft Connect (fait — installés uniquement)

- Comme Riot : biblio en ligne = login + scraping fragile → on fait **scan installé + lancement** seulement.
- `platforms/ubisoft.rs` : registre `HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\<gameId>\InstallDir`
  (+ variante sans WOW6432Node). Titre = dernier segment du dossier d'install. `id="ubisoft:<gameId>"`,
  `launch_target=<gameId>`, taille = `dir_size`. Testé : R6 Siege 75 Go, AC Shadows 168 Go, AC Black Flag, Roller Champions.
- Lancement (`platforms::launch` cas `"ubisoft"`) : `open_uri("uplay://launch/<gameId>/0")`.
- Front : PlatformId `"ubisoft"`, couleur `--ubisoft` (#2aa3ee), icône swirl, entrée sidebar, `PLATFORM_ORDER`.
- 😎 « RollerChampions » (Ubisoft) fusionne avec « Roller Champions » (Epic) via le titre normalisé.

## Menu contextuel, ajout manuel, auto-update (fait)

- **Menu contextuel (clic droit)** : `useContextMenu.ts` (singleton `{open,x,y,game}`) + `ContextMenu.vue`
  monté globalement dans `App.vue`. Clic droit sur `GameCard`/`SalonTile` (`@contextmenu="openContext($event, game)"`).
  Items : Jouer, Voir la fiche, (Ré)favori, Masquer/Réafficher, puis **Désinstaller** (si `installed`) ou
  **Retirer de la bibliothèque** (si `platform==="manual"` → `removeManual`). Position rabattue dans le viewport
  (mesure après `nextTick`). Ferme au clic-fond/Échap/scroll.
- **Ajout manuel d'un jeu** : bouton « Ajouter » dans `TopBar` → `useUi.addGameOpen` → `AddGameModal.vue`
  (titre + chemin exe requis, dossier + jaquette optionnels). Ponts `addManualGame`/`removeManualGame` dans
  `lib/tauri.ts` → commandes Rust déjà enregistrées (`platforms/manual.rs`). `useLibrary.addManual` insère le
  jeu créé dans le store sans re-scan (via `fromDto` exporté) ; `removeManual` le retire. Ouvre la fiche après ajout.
- **Auto-update de l'app** (plugin `updater` Tauri 2) : `useUpdater.ts` (check au démarrage → `available` →
  `downloadAndInstall` avec progression → `relaunch`) + `UpdateBanner.vue` (bannière bas-droite). Silencieux hors Tauri.
  - Rust : deps `tauri-plugin-updater` + `tauri-plugin-process`, enregistrés dans `lib.rs`. Permissions
    `updater:default`/`process:default`/`process:allow-restart` dans `capabilities/default.json`.
  - Config `tauri.conf.json` : `bundle.createUpdaterArtifacts:true` + `plugins.updater` (pubkey + endpoint
    `github.com/tompoyeau/torii/releases/latest/download/latest.json`, `windows.installMode:"passive"`).
  - **Clés de signature** : `~/.tauri/torii-updater.key` (+ `.pub`), mot de passe **vide**, hors repo (gitignore
    `*.key`). Régénérer : `node_modules/.bin/tauri signer generate -w <path> --ci -p ""`.
  - **Release** : `.github/workflows/release.yml` (tauri-action, déclenché sur tag `v*`) build+signe+publie la
    Release Windows avec `latest.json`. Secrets repo : `TAURI_SIGNING_PRIVATE_KEY` (+ password vide). Voir `RELEASE.md`.
  - ⚠️ La v0.1.0 (sans updater) ne s'auto-met pas à jour : 1re install manuelle de la v0.2.0, puis automatique.
    Bumper la version dans les **3** fichiers (package.json, tauri.conf.json, Cargo.toml) à chaque release.

## Filtre par catégorie / genre (fait)

- Filtrage par **genre**, qui se **combine** aux filtres sidebar (plateforme/favoris/…) + recherche + tri.
  État `genre: string|null` dans `useUi` (`setGenre`, null = toutes). Menu déroulant dans l'en-tête de
  `BureauView` (à côté des puces de tri), affiché seulement si des genres existent.
- `availableGenres` (computed BureauView) : genres uniques des jeux non masqués, triés par nombre décroissant
  (compteurs affichés). `shownGames` applique `g.genre === genre` après le filtre courant. Menu = bouton
  `.genre-btn` (actif en accent si un genre est choisi) + popover `.genre-menu` avec « Toutes les catégories »
  en tête. Ferme au clic-dehors (listener document).
- La métadonnée descriptive est peuplée en masse par **IGDB** (voir ci-dessous), source unique tous launchers.

## Métadonnée via IGDB + mini-proxy (fait) — SOURCE UNIQUE

- La métadonnée descriptive (genre, description, captures, hero, studio, année, jaquette) n'existe pas en local et
  Steam ignore les jeux hors-Steam (Fortnite/Valorant/WoW). Source = **IGDB** (base cross-plateforme, Twitch). Comme
  IGDB exige un token Twitch non-embarquable, on passe par un **mini-proxy Cloudflare** (`proxy/`, déployé sur
  `torii-igdb-proxy.toriiapp.workers.dev`) qui détient le secret — exactement l'approche Playnite. Setup dans `proxy/README.md`.
- `metadata/igdb.rs::fill_metadata` renvoie un `IgdbMeta` complet par jeu (genre/description/coverUrl/heroUrl/developer/
  year/screenshots). **Steam en masse** via `external_games` (`external_game_source = 1` & `uid = appids`) → `games`
  (exact, 2 appels/500 jeux) ; **non-Steam par nom** (`where name = "X"` exact, repli `search` + sélection du nom
  normalisé). Champs via const `FIELDS`. Images `images.igdb.com/.../t_{taille}/{image_id}.jpg` (cover=cover_big_2x,
  hero=1re artwork/capture en 1080p). Cache `igdb_meta_cache_v1.json`, throttle 300 ms (<4 req/s). Commande `enrich_igdb`
  (événement `igdb-batch`) ; front `enrichIgdb`/`useLibrary.fillIgdb`. Testé réel : **822/887 (93 %)**, dont 818 genre /
  821 jaquette / 821 description. `cargo run --example genres`.
- **Fusion front (`fillIgdb`)** : remplit chaque champ SANS écraser ce que le launcher a fourni. 🔑 Jaquette/hero =
  **launcher d'abord, IGDB en repli** (décision user) → capsules Steam conservées, IGDB comble les manquantes.
- Données JOUEUR (temps de jeu, installé, possédé, famille) = toujours 100 % des launchers, jamais IGDB.
  Taille de téléchargement (non-installés) conservée sur steamcmd.net/API GOG (IGDB ne l'a pas) via l'enrich lazy `enrich_game`.
- ⚠️ `search` IGDB est fuzzy (remonte DLC/jeux voisins) → toujours filtrer par nom normalisé, jamais le 1er résultat brut.
  `~"x"` sans wildcards ne matche pas (`~ *"x"*` = contains) ; `="x"` = exact sensible à la casse. Ratés : Overwatch 2
  (absent d'IGDB en jeu de base) + ~7 % niche. Proxy URL en dur (`PROXY_URL`). Recherche Steam-par-titre pour jaquettes RETIRÉE.

## Détection des parties — `procwatch.rs` (fait)

- Un **seul fil** surveille les process et date « Récemment joué », **y compris pour les
  parties lancées hors de Torii** (Steam, bureau, raccourci). Remplace l'ancien
  `game_watch_loop` (sysinfo) ET complète l'enregistrement au clic sur « Jouer ».
- 🔑 **Ne pas revenir à `sysinfo`.** Mesuré sur 370 process : `refresh_processes_specifics`
  avec chemins = **11,9 ms** par passage, et l'ancien suivi le faisait toutes les 3 s
  *pendant la partie*. Ici : `K32EnumProcesses` (tableau de PID brut) puis
  `QueryFullProcessImageNameW` sur les seuls PID **nouveaux** → **0,42 ms** par tick
  mesuré, 28× moins. FFI directe sur `kernel32`, aucune dépendance (comme la DPAPI).
  `sysinfo` a été retiré du `Cargo.toml`.
- Rythme : 5 s au repos, **15 s dès qu'un jeu tourne** (on n'attend plus qu'une fermeture,
  autant se faire oublier pendant que le joueur joue). Zéro appel système tant que la
  bibliothèque n'a pas été scannée (`targets` vide).
- 🔑 La date posée est l'**heure de démarrage réelle du process** (`GetProcessTimes`,
  FILETIME → Unix), pas l'instant de la détection : ça absorbe la latence du sondage et
  date correctement un jeu déjà lancé quand Torii s'ouvre. `playhistory::record_at` ne
  **recule** jamais une date connue.
- Rapprochement par préfixe de chemin sur `install_dir` (repli : l'exe pour un jeu manuel
  sans dossier). Le `\` final dans `under()` évite qu'un dossier voisin plus long
  (« Portal 2 Demo ») passe pour le jeu (« Portal 2 »).
- `start_game_watch` ne détecte plus rien : il **arme** juste le jeu dont la fermeture doit
  ramener la fenêtre (option « revenir à la fermeture »). Sans ça, Torii surgirait à la fin
  de n'importe quelle partie lancée ailleurs.
- Front : événement `game-launched` → `useLibrary.notePlayed(id, at)` (maj du store sans
  re-persister, le backend l'a déjà fait).
- ⚠️ Comportement assumé : une application Steam permanente (Wallpaper Engine…) est bien
  détectée comme « en cours » — c'est ce que fait Steam aussi. Elle est datée de son vrai
  démarrage, pas remise en tête à chaque ouverture de Torii.
- Diagnostic : `cargo run --release --example watch` (jeux détectés + heure de démarrage).

## Dernière session « maison » (fait)

- Pour les jeux sans stats de launcher (Riot/EA/Battle.net/Ubisoft/manuel…), Torii enregistre l'instant du
  **clic sur Jouer** comme date de dernière session. `platforms/playhistory.rs` : `last_played.json` (id → Unix),
  `record(dir, id)`/`load(dir)`. Commande `record_launch(id)`. `scan_all` fusionne : `last_played = max(launcher, maison)`.
- Front : `useLibrary.markPlayed(id)` (maj optimiste `lastPlayedAt`/`recent` + persiste via `recordLaunch`), appelé à
  chaque point de lancement (GameDetail onPlay/playFrom, ContextMenu, SalonHero, HeroFeatured). Le jeu remonte aussitôt
  dans « Récemment joué ». ⚠️ Limite assumée : lancement HORS Torii = non capté (le user était OK). Piste future :
  surveiller le process du jeu pour le vrai temps de jeu (plus fragile).

## Service social — `server/` (côté serveur fait, client à brancher)

- **Worker distinct du `proxy/`** : celui-ci détient comptes, amis et présence (base D1),
  l'autre relaie IGDB/ITAD sans rien retenir. Secrets et risques différents → deux
  déploiements. Toutes les routes sont préfixées `/v1` (l'auto-updater fait cohabiter des
  versions, et le mobile viendra s'y brancher). Détail complet dans `server/README.md`.
- **Connexion par code à 6 chiffres reçu par e-mail**, jamais de mot de passe : rien de
  réutilisable à voler côté serveur, et la récupération de compte EST la connexion. Codes
  et jetons ne sont stockés que hachés (SHA-256 + poivre `PEPPER`).
- 🔑 **On ajoute un ami par code d'ami, jamais par e-mail** : chercher par adresse
  transformerait le service en annuaire de « qui utilise Torii ». Même raison pour les
  suggestions par SteamID, qui exigent que **les deux** comptes soient découvrables.
- 🔑 **`PUT /v1/presence` renvoie le cercle complet** : le battement de cœur (30 s) sert
  aussi de lecture, ce qui divise le trafic par deux. ~2 880 requêtes/jour et par personne,
  pour 100 000 offertes → une trentaine de testeurs avant de devoir passer au push.
- **Aucun historique** : la présence porte sa date de péremption et n'est jamais archivée.
  Un compte sans battement depuis 90 s repasse `offline`, ce qui veut dire « Torii fermé »
  et non « ne joue pas » — vocabulaire à respecter dans l'interface.
- ⚠️ **`DEV_CODES=1` rend le code dans la réponse HTTP** (pour tester sans domaine
  d'expédition). Actif en production, il laisse entrer n'importe qui.
- Le vrai envoi d'e-mails exige **un domaine à soi** (impossible depuis `*.workers.dev`)
  et wrangler 4 (`wrangler email sending enable <domaine>`).

### Côté client — `social.rs`

- Client de l'API + **battement de cœur** (30 s) qui publie la présence et reçoit le cercle
  en retour, réémis au front par l'événement `torii-circle`. `TORII_API` surcharge l'URL
  pour développer contre `npx wrangler dev` sans recompiler.
- 🔑 Le jeton de session vit dans `credentials.dat` (donc chiffré DPAPI), comme les jetons
  des launchers — jamais dans le `localStorage` de la WebView.
- 🔑 **`share_presence` est faux par défaut** (`social_prefs.json`) : le fil tourne mais
  n'envoie RIEN tant que l'utilisateur n'a pas activé le partage. Le couper efface la
  présence immédiatement au lieu d'attendre la péremption serveur.
- Jeux jamais diffusés : `id_set::PRESENCE_MUTED` (`presence_muted.json`), même mécanique
  que masqués/favoris. Indispensable pour les applications permanentes type Wallpaper
  Engine, qui annonceraient une partie 24 h sur 24.
- La décision de publication est isolée dans `presence_for()` — fonction pure, testée :
  c'est là que se joue la promesse faite à l'utilisateur, elle doit être vérifiable.
- « Absent » = `GetLastInputInfo` (un appel système, aucun hook clavier). Une partie en
  cours prime toujours sur l'inactivité.
- Clé de jeu cross-launcher : `game_key()` normalise le titre (minuscules, alphanumérique)
  → « THE WITCHER 3: WILD HUNT™ » et « The Witcher 3: Wild Hunt » se rejoignent. À
  remplacer par l'id IGDB quand il sera persisté.
- ⚠️ Les ponts `torii*` de `lib/tauri.ts` **laissent remonter les erreurs**, contrairement
  au reste du fichier : les messages du serveur sont écrits pour être affichés tels quels.

## Prochaines étapes

1. **Comparateur de prix** : wishlist (à capter) × CheapShark / IsThereAnyDeal.
2. Temps de jeu Steam local (`localconfig.vdf`).
3. Peupler les genres plus largement sans saturer l'API (pour un filtre catégorie plus riche).
