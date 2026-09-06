# ARCHITECTURE.md — Ludo

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
  `components/` (AppShell, GameCard, GameDetail, Sidebar, TopBar, SettingsView).
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
- 🔑 **Une classe partagée entre plusieurs vues vit dans `style.css`, jamais dans un
  `<style scoped>`.** `.chip` (puces de tri, filtres, « rafraîchir ») n'était déclarée que
  dans AppShell et StoreView : elle s'affichait donc en **bouton Windows brut** dans la
  bibliothèque d'un ami, dans « En commun » et dans la Wishlist, qui n'en déclaraient que
  les variantes (`.chip.refresh`). Un scoped ne porte QUE sur les éléments du composant —
  un enfant n'en hérite pas.
- 🔑 **`.cover-card` porte `width: 100%`, et ce n'est pas redondant.** La carte est un
  `<button>`, donc elle se dimensionne sur son contenu. Posée directement dans la grille
  elle est étirée par elle (bibliothèque, Boutique, Wishlist : rien ne se voyait) ;
  enveloppée dans un `<div>` — `FriendLibraryView` et `CommonView` mettent chaque carte
  dans une `.cell` pour lui accoler leurs pastilles — elle retombait sur la largeur de son
  texte, et chaque jaquette prenait la taille de sa ligne « Steam · 12 h · Non installé ».

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

- Puces de tri fonctionnelles : **Récemment joué** (`lastPlayedAt` desc), **A → Z** (`title`
  localeCompare fr), **Temps de jeu** (`hoursPlayed` desc). État `sort: SortKey` dans `useUi`
  (défaut "recent"), `sortGames()` dans AppShell appliqué après `filtered()`.
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
  monté globalement dans `App.vue`. Clic droit sur `GameCard` (`@contextmenu="openContext($event, game)"`).
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
  `AppShell` (à côté des puces de tri), affiché seulement si des genres existent.
- `availableGenres` (computed AppShell) : genres uniques des jeux non masqués, triés par nombre décroissant
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

## Tout en français, au possible

Les prix étaient déjà en euros (ITAD interrogé en `country=FR`), mais tout le reste
arrivait en anglais. Source par source, ce qui est francisable et ce qui ne l'est pas :

- **Steam** (`steam_store.rs`) : `l=french&cc=fr` sur `appdetails` ET `storesearch`.
  Description, genres et date de sortie deviennent français. 🔑 Le champ `name`, lui,
  n'est **pas** localisé par Steam (vérifié) — aucun risque de renommer la bibliothèque.
  ⚠️ La date devient « 24 févr. 2017 » : `parse_year` cherche 4 chiffres en 19xx/20xx, il
  s'en moque.
- **GOG** (`gog_store.rs`) : `?locale=fr-FR` sur l'API v2 → description et tags français.
  `globalReleaseDate` reste une date ISO.
- **Instant Gaming** : 🔑 **le chemin de recherche change avec la langue** —
  `/fr/rechercher/` et `/en/search/`. Mesuré : `/fr/search/` et `/fr/recherche/` répondent
  404 ; l'adresse française se lit dans le formulaire de la page d'accueil FR. On cherche
  en français **puis en anglais** : IG traduit certains titres (« Shadow of the Erdtree »
  → « L'ombre de l'Arbre-monde ») et le rapprochement par titre exact échouerait, ce qui
  ferait DISPARAÎTRE des offres. 🔑 L'URL d'un résultat venu du repli est francisée par
  simple remplacement `/en/` → `/fr/` : IG redirige (301) le slug anglais vers son
  équivalent français. Le suffixe « - PC (Steam) » et le marqueur `addtocart` sont
  identiques sur les deux sites, le parseur et le test de stock ne bougent pas.
- **IGDB** : ⚠️ **ne localise RIEN** — ni les résumés, ni les genres. Les descriptions IGDB
  restent donc en anglais. Les **genres**, eux, sont traduits par une table
  (`genre_fr`) : leur vocabulaire est **fermé** (23 entrées relevées sur `/genres`), donc
  la table est exhaustive et testée. 🔑 Elle s'applique **en sortie** (`traduire_lot`) et
  non à l'écriture : le cache garde les libellés d'origine, donc compléter la table plus
  tard ne coûte pas un retéléchargement de toute la bibliothèque.

- **Boutique** (`store::game`) : la fiche produit est enrichie par IGDB, donc son **genre**
  est déjà français (table `genre_fr`) mais sa description ne l'était pas. Quand le jeu
  porte un appid, on repasse par `metadata::enrich_one` pour prendre la description
  française de Steam. 🔑 C'est le **même cache disque que la bibliothèque** : un jeu déjà
  possédé ne coûte aucune requête, et consulter une fiche de la vitrine réchauffe le cache
  pour plus tard. ⚠️ Le **genre** n'y est volontairement pas remplacé — celui d'IGDB sert
  de clé au filtre par catégorie, et prendre celui de Steam ferait dire deux choses
  différentes au même jeu selon l'écran. ⚠️ Conséquence assumée : ouvrir la fiche d'un jeu
  Steam encore inconnu déclenche aussi l'appel taille (`api.steamcmd.net`), comme dans la
  bibliothèque.
- Le reste de la Boutique n'a rien à traduire : titres de jeux, noms de boutiques et studios
  sont des noms propres.

### 🔑 Le piège : IGDB gagnait toujours

Franciser les sources n'aurait presque rien changé à l'écran. IGDB remplit les
descriptions **en masse au chargement**, et `useLibrary.ensureEnriched` ne remplaçait
jamais une valeur déjà là (`cur.description ?? meta.description`) : la version française,
arrivée plus tard à l'ouverture de la fiche, partait à la poubelle.

D'où `GameMeta.localized` : vrai quand les métadonnées viennent d'une source interrogée en
français **et** identifiée par un identifiant sûr (appid Steam, id produit GOG). Dans ce
cas seulement, la description remplace celle d'IGDB.

⚠️ Le repli « recherche Steam par titre » (Epic, manuel) est explicitement remis à
`localized = false` dans `metadata::fetch` : son contenu est bien français, mais le **jeu
est deviné**. Une description française du mauvais jeu est pire qu'une bonne description
anglaise.

⚠️ `metadata_cache_v3` → **v4** : les entrées existantes contiennent des descriptions et
des genres anglais. Sans le bump, personne ne verrait le changement.

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
  chaque point de lancement (GameDetail onPlay/playFrom, ContextMenu, HeroFeatured). Le jeu remonte aussitôt
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

- 🔑 **Suggestions Steam** : `POST /v1/friends/invite` accepte `{ accountId }` en plus de
  `{ friendCode }`. Sans ça, une suggestion (qui rend un identifiant) ne pouvait pas mener
  à une invitation — le tuyau existait sans robinet. Aucun risque d'énumération : un
  identifiant fait 25 caractères aléatoires, et on ne l'obtient que par une suggestion,
  laquelle exige la découvrabilité des DEUX comptes.
- ⚠️ Le SteamID d'un ami n'est renvoyé dans le cercle que s'il est découvrable : c'est ce
  champ qui permet à `useFriendList` de fusionner sa ligne Torii et sa ligne Steam.

### Bibliothèques synchronisées — `server/src/library.js` + `libsync.rs` + vue Amis

- **Charge utile dans R2, index dans D1.** Une ligne par jeu et par personne = ~1 000
  écritures D1 par resynchronisation, contre 100 000/jour offertes : le service s'arrêterait
  à une centaine de joueurs. Dans R2 c'est **un objet par appareil**
  (`lib/<compte>/<appareil>.json`, ~120 Ko, 1 opération), 10 Go-mois gratuits et **trafic
  sortant gratuit** — ce dernier point est ce qui rend le mobile viable.
- La table `libraries` ne garde que l'**index** (empreinte, date, nombre de jeux) : savoir
  si une bibliothèque a changé **sans la télécharger**. R2 ne sait pas chercher, donc le
  croisement « qui possède ce jeu » se fait côté client, comme `useFriendsCommon` le fait
  déjà pour Steam.
- 🔑 **Un objet par APPAREIL, jamais par compte** : deux PC n'ont pas la même bibliothèque,
  et au même endroit le dernier passé effacerait l'autre indéfiniment. « La bibliothèque de
  quelqu'un » = l'union de ses appareils, faite côté client. Plafond de 5 appareils.
- 🔑 **Synchroniser ≠ partager.** *Synchroniser* est une préférence **client** (envoyer sa
  bibliothèque, pour la retrouver sur son propre mobile) ; *partager* est un drapeau
  **serveur** (`accounts.share_library`, éteint par défaut) qui ouvre la lecture aux amis
  acceptés. Sa propre bibliothèque se lit toujours, partage éteint ou non.
- ⚠️ **Première donnée durable du service** : la promesse « aucun historique » ne couvre que
  la présence. Ici on sait ce que les gens **possèdent** — jamais ce qu'ils jouent ni quand.
  Le README du serveur a été réécrit en conséquence, ne pas le laisser mentir.
- 🔑 **Le Worker est l'unique porte** : le bucket R2 est privé (aucun domaine `r2.dev`), un
  objet n'en sort qu'après vérification jeton + amitié + partage.
- 🔑 **Ordre des suppressions** : l'objet R2 part AVANT sa ligne d'index (l'index est la
  seule carte qui y mène) ; à l'inverse, l'index est écrit APRÈS l'objet. `deleteMe` appelle
  `forgetAllLibraries` **hors transaction et en premier** — un compte effacé qui laisse ses
  bibliothèques dans le bucket, c'est une promesse rompue.
- Le serveur **normalise et tronque** tout ce qu'il reçoit (clé, titre, plateformes,
  jaquette HTTPS ; 5 000 jeux max) et jette les champs inconnus : une route authentifiée qui
  écrit dans R2 est sinon un hébergement de fichiers gratuit.
- L'empreinte cliente sert d'**ETag** → `If-None-Match` renvoie `304` sans lecture R2.
- ⚠️ `schema.sql` ne modifie pas une base existante (tout en `IF NOT EXISTS`) : les
  changements de schéma vivent aussi dans `server/migrations/` (`npm run migrate`).
- Testé de bout en bout contre `wrangler dev --local` (33 vérifications : normalisation,
  ETag/304, 404 sans amitié, 403 sans partage, plafonds, suppressions — bucket local
  vérifié vide après coup).

#### Côté client — `libsync.rs`

- `partageables()` = **fonction pure et testée**, au même titre que `presence_for` : c'est
  elle qui tient la promesse « un jeu masqué ou muet ne sort pas d'ici ». Elle regroupe
  aussi les entrées par clé de jeu → c'est ce qui produit « je l'ai sur Steam ET sur GOG »
  (le scan rend une entrée par launcher, l'ami veut une ligne par jeu).
- **Empreinte FNV-1a 64 bits** (`empreinte()`), volontairement non cryptographique et sans
  dépendance nouvelle : elle répond à « est-ce que ça a changé ? », rien d'autre. Sans
  elle, chaque démarrage réécrirait le même objet. 🔑 Elle est mémorisée **avec l'id du
  compte** (`SocialPrefs::last_library_sync`) : sinon, changer de compte laisserait le
  nouveau vide pour toujours. Un `BTreeMap` + tri des plateformes rendent l'empreinte
  indépendante de l'ordre du scan — sinon on renverrait tout à chaque fois.
- Déclenchée **après chaque `scan_library`**, dans un fil détaché : un scan ne doit jamais
  attendre le réseau. `sync()` ne renvoie une erreur que si l'envoi échoue vraiment —
  « éteint » et « pas connecté » sont des situations normales (`skipped`).
- Commandes : `library_sync(force)`, `library_index`, `library_of`, `library_forget_device`,
  `library_set_sync(enabled)`. ⚠️ **Couper la synchronisation efface le serveur** : une
  bibliothèque figée que les amis continueraient de voir serait pire que pas de
  bibliothèque du tout.
- Interface : deux interrupteurs dans Réglages → Réseau Torii (« Synchroniser ma
  bibliothèque » = pref locale, « Visible par mes amis Torii » = `share_library` serveur,
  désactivé tant que la synchro est éteinte), état du dernier envoi, bouton « Synchroniser
  maintenant », liste des autres appareils avec « Retirer ». Validé en preview.
  🔑 PIÈGE CSS : `.pane-hint` porte un `margin-top: -10px` (il est fait pour se glisser
  **sous un titre**) — l'utiliser après une ligne d'interrupteur le fait chevaucher le
  bouton. D'où `.sync-state` pour la ligne d'état.

#### Vue « bibliothèque d'un ami » — `FriendLibraryView.vue` + `useFriendLibrary.ts`

- Section à part entière (`friendLibrary` dans `useUi`), pas une modale : elle se parcourt
  et se filtre comme la sienne. 🔑 `friendLibraryId` fait partie de l'**instantané de
  navigation** — sans lui, le retour souris restaurait la section mais pas de qui il
  s'agit, et rejouait la bibliothèque du dernier ami consulté.
- `useFriendLibrary` fait l'**union des appareils** d'une personne (deux PC = deux
  instantanés) et le croisement avec notre bibliothèque. C'est ici que ça se passe et pas
  côté serveur : R2 ne sait pas chercher, c'est le compromis assumé du choix d'archi.
  Cache mémoire par compte pour ne pas retélécharger à chaque aller-retour.
- « Tu l'as aussi » repose sur `gameKeyOf`, **troisième copie** de `social::game_key()`
  (Rust, `useFriendList`, ici). Les trois doivent rester d'accord, sinon le croisement ne
  se déclenche jamais.
- Filtres : Tous / Que tu n'as pas / Vous l'avez tous les deux. Le jeu qu'on possède
  affiche NOTRE fiche (clic → détail) ; sinon une carte synthétique, comme `CommonView`.
- Point d'entrée : bouton au survol sur les fiches et lignes d'amis, **uniquement** si la
  personne partage (`hasLibrary`) — proposer un écran vide serait pire que rien.
- 🔑 L'index est chargé dans `useTorii.start()` **et** à l'ouverture de la vue Amis. Il ne
  l'était d'abord qu'à l'ouverture des Réglages : le bouton n'apparaissait donc qu'après un
  détour par les Réglages, c'est-à-dire jamais. Attrapé en preview.
- ⚠️ PIÈGE CSS : dimensionner `.platform-icon`, **pas** `svg` — Epic et Ubisoft rendent une
  image, pas un SVG, et le PNG s'affichait en pleine taille.

#### « En commun » alimenté par les deux sources — `useFriendsCommon.ts`

- La vue croisait uniquement **mes jeux Steam × mes amis Steam** (commande Rust
  `friends_common`). Les bibliothèques Torii s'y ajoutent comme **deuxième source**, ce qui
  élargit la vue des deux côtés : un ami absent de Steam peut désormais apparaître, et un
  de MES jeux GOG/Epic/manuel peut enfin être « en commun ».
- Fusion par **clé de titre** (`keyOfTitle`, encore la même normalisation que
  `social::game_key`) : un jeu possédé des deux côtés donne UNE carte dont les
  propriétaires sont cumulés, jamais deux. Vérifié en preview en forçant le cas.
- Identité des amis : un ami Torii dont le SteamID est connu ET déjà présent dans la liste
  Steam compte sous son SteamID (une seule pastille pour une seule personne) ; sinon
  `torii:<accountId>`, avec une pastille « Torii » pour expliquer d'où il sort. ⚠️ Un ami
  Torii **non découvrable** qui est aussi ami Steam apparaîtra deux fois : on n'a aucun
  moyen de savoir que c'est la même personne (même limite que `useFriendList`).
- `commonCount` est **recalculé** sur la liste fusionnée (le backend ne connaît que Steam).
- ⚠️ Le garde « Steam non connecté » ne bloque plus toute la vue : il ne s'affiche que si
  `readable` est vide aussi — sinon la vue serait inaccessible à quelqu'un qui n'a pas
  Steam mais dont les amis partagent leur bibliothèque Torii.
- `ownersOf` (utilisé par la fiche d'un jeu) croise désormais par clé de titre et non par
  appid, sans quoi un jeu GOG/Epic n'aurait jamais de propriétaire.
- ⚠️ Asymétrie à connaître : le côté Steam vient du backend (qui a déjà fait
  « mes jeux ∩ amis »), le côté Torii est filtré contre `useLibrary.games`. Un jeu que le
  backend croit à moi mais absent du scan local (ou masqué) ne reçoit donc pas de
  propriétaires Torii.


#### « En commun » : le partage familial n'est pas de la possession

Une copie familiale Steam n'appartient pas à celui qui l'emprunte, et c'est **une** licence
— elle ne se joue qu'à une personne à la fois. Comptés comme possédés, ces jeux
produisaient des lignes illisibles : impossible de savoir, en les regardant, qui possède
vraiment quoi, ni si deux personnes d'une même famille pourraient y jouer ensemble.

🔑 **La règle est donc : « en commun » = possédé des deux côtés.** Les jeux du partage
familial sont exclus de la vue, des deux côtés, dans `useFriendsCommon` :
`myByKey` saute `g.familyShared`, et `toriiOwnersByKey` saute `jeu.familyShared`.

- ⚠️ Le croisement Steam (`friends_games::fetch_live`) était **déjà** propre : il passe par
  `GetOwnedGames` pour moi comme pour mes amis, et cet endpoint ne rend que le possédé.
  Seule la source Torii pouvait faire entrer du familial — inutile d'aller filtrer côté
  Rust.
- Ces jeux restent **visibles ailleurs**, là où c'est leur place : dans la bibliothèque
  d'un ami (avec la pastille « Famille Steam ») et dans la sienne propre, sous le filtre
  « Famille » de la barre latérale. C'est « ce qu'il peut jouer » ; « En commun », c'est
  « ce que vous possédez ».
- 💭 Une première version affichait au contraire ces jeux avec des pastilles de provenance
  et un avertissement « une seule licence », en conservant les membres du groupe familial
  pour savoir si l'ami puisait au même pot. Abandonné : beaucoup de machinerie, un
  avertissement subtil à lire, et une dépendance à un endpoint Steam non documenté — pour
  un résultat moins clair que de simplement ne pas les compter.

#### Invitation à partager — `LibraryInvite.vue`

- Le partage est éteint par défaut (et le reste après mise à jour : `sync_library` faux,
  `share_library` à 0 en base). Conséquence assumée mais gênante : **personne ne découvre
  la fonctionnalité**. D'où un bandeau dans la vue Amis, à l'endroit où la proposition a
  du sens, calqué sur `ToriiPanel`.
- 🔑 **Une proposition se fait une fois** : « Non merci » est définitif
  (`libraryInviteDismissed` dans `ludo-prefs`), même principe que `steam_auto_linked`.
- 🔑 Le bandeau propose les **deux usages séparément** — « Partager avec mes amis »
  (synchro + partage) et « Seulement mes appareils » (synchro seule, pour retrouver sa
  bibliothèque sur son mobile sans la montrer à personne). Il allumait d'abord les deux
  d'un bloc, ce qui laissait croire qu'emporter sa bibliothèque impliquait de la partager,
  alors que le modèle les distingue depuis le début.
- ⚠️ CSS : `.invite` en `align-items: stretch` et `.sub` **sans `max-width`** — en
  `flex-start` avec une colonne de 60 caractères, le texte laissait une bande vide à
  droite alors que le bandeau, lui, va jusqu'au bord.
- ⚠️ N'apparaît que si la synchro n'a **jamais** été activée. Quelqu'un qui synchronise
  pour son mobile sans partager a fait un choix délibéré ; le bandeau ne le harcèle pas.


#### Présence **par source** dans la vue Amis — `useFriendList` + `FriendsView`

- `UnifiedFriend` porte `steamState` et `toriiState` (`null` = pas ami de ce côté) en plus
  de `state`, qui reste l'agrégat servant au classement. 🔑 L'agrégat seul effaçait
  l'essentiel : quelqu'un d'ami des deux côtés, en ligne sur Steam avec Torii fermé,
  s'affichait « en ligne » sans qu'on puisse savoir que Torii ne voit rien de ce qu'il joue.
- La vue rend donc **une pastille par canal** (Torii, Steam) au lieu d'une pastille
  « Torii + Steam » : allumée = présent sur ce canal, éteinte = relation existante mais pas
  connecté. L'infobulle donne la phrase exacte (« Torii fermé : ce qu'il joue hors Steam
  reste invisible »). `offlineHint` distingue aussi le cas « des deux côtés ».
- Vérifié en preview sur les trois cas : ami des deux côtés présent partout, ami des deux
  côtés présent d'un seul (pastille Steam éteinte), ami d'une seule source.

#### Partage familial Steam : accès n'est pas possession

- `LibGame.familyShared` dit qu'un jeu n'arrive QUE par le groupe familial Steam. Sans lui,
  « ce que mon ami possède » comptait comme sien un jeu qui appartient à son frère et qui
  repartira le jour où celui-ci le retire du partage.
- 🔑 **Une seule source possédée suffit à faire du jeu le sien** : `partageables()` amorce
  le drapeau à vrai puis fait un `&=` sur chaque source de la même clé (le `merge()` des
  appareils côté front applique la même règle avec `&&`). Un jeu emprunté sur Steam mais
  acheté sur GOG est bien à lui.
- ⚠️ Il entre dans `empreinte()`. Sans ça, une bibliothèque dont seul le statut familial
  change ne repartirait jamais. Conséquence attendue : au premier scan après cette version,
  toutes les bibliothèques sont réenvoyées une fois.
- Le serveur ne retient `familyShared` que **strictement égal à `true`** : la route écrit
  dans R2, un client ne doit pas pouvoir y glisser une chaîne. Absent = possédé, donc
  l'immense majorité des lignes n'a pas ce champ. ⚠️ Le drapeau vient du client de l'ami :
  tant qu'il n'est pas à jour, ses jeux familiaux restent indistincts.

#### Page profil d'un ami — `FriendProfileView.vue`

- Cliquer sur quelqu'un ouvrait sa **page Steam dans le navigateur** : on quittait Torii
  pour une page qui ne connaît que Steam, ne dit rien de ses jeux GOG/Epic et n'a aucun
  bouton « voir sa bibliothèque ». Tout le monde a désormais sa page dans l'application
  (section `friendProfile`).
- 🔑 `friendProfileKey` porte la **clé unifiée** (`UnifiedFriend.key` : `torii:<id>` ou
  `steam:<id>`), **pas** un identifiant de compte Torii. C'est ce qui permet à un ami Steam
  sans compte Torii d'avoir lui aussi une page. Un ami qui a un compte Torii a toujours la
  clé `torii:<toriiId>` (la fusion des deux sources garde la clé Torii) — d'où le
  `` `torii:${friendLibraryId}` `` du retour de `FriendLibraryView`.
- 🔑 **La page d'un ami Steam est presque vide, et c'est le propos** : elle dit ce que Torii
  ne peut pas savoir de lui **et pourquoi**, et propose le geste qui y remédie (ton code
  d'ami, copiable sur place). Le renvoyer vers Steam sans explication se lisait comme une
  panne.
- ⚠️ **Ne jamais écrire « il n'a pas de compte Torii »** : on n'en sait rien. `toriiId`
  absent veut seulement dire qu'il n'est pas dans TES amis Torii — il peut très bien
  utiliser Torii sans que vous y soyez liés. Les deux écrans vides s'en tiennent à ce qui
  est vérifiable (« pas dans tes amis Torii », « le partage est éteint »).
- ⚠️ `friendProfileKey` fait partie de l'**instantané de navigation**, exactement comme
  `friendLibraryId` : sans lui, le retour souris restaurerait la section mais pas de qui
  il s'agit.
- **Hiérarchie Amis → Profil → Bibliothèque.** Le retour de `FriendLibraryView` pointe donc
  sur le profil (« Profil »), pas sur la liste. Les **boutons bibliothèque et corbeille des
  lignes d'amis ont été retirés** : tout ce qui concerne une personne se fait sur sa page,
  la liste ne fait que mener à elle.
- **Retirer un ami vit derrière l'écrou** de la page profil, avec une **modale** de
  confirmation (et non plus la confirmation en place des lignes) : le geste est
  indéfaisable — il faudra une nouvelle demande acceptée des deux côtés. L'écrou n'apparaît
  que s'il a quelque chose dedans, donc jamais pour un ami Steam pur.
- 🔑 La page **relit l'index des bibliothèques** à l'ouverture. Tout son contenu en dépend
  (partage-t-il, combien de jeux) et l'index n'est chargé qu'au démarrage et à l'ouverture
  de la vue Amis — même piège que celui qui avait rendu le bouton « voir sa bibliothèque »
  invisible en pratique.
- 🔑 **L'aperçu tient sur UNE rangée, en CSS pur** (`.strip`) : première rangée explicite,
  rangées suivantes en `grid-auto-rows: 0` et `overflow: hidden`. `row-gap: 0` est
  indispensable — sinon les rangées invisibles laissent quand même leurs gouttières, soit
  une bande vide sous la ligne. Le nombre de tuiles visibles suit la largeur tout seul,
  sans mesure JS ; le `slice(0, 16)` ne sert qu'à ne pas construire 464 tuiles cachées.
- La présence par canal vit dans `lib/friendPresence.ts` (`sourcesOf`, `offlineHintOf`),
  partagée avec `FriendsView` : deux écrans qui décriraient différemment la même personne
  se liraient comme une contradiction.

#### Profil Steam : fenêtre Torii, et pourquoi pas intégré — `open_web_window`

- 🔑 **Une page Steam ne PEUT PAS être intégrée dans l'interface.** Mesuré :
  `steamcommunity.com` sert `X-Frame-Options: SAMEORIGIN` et
  `frame-ancestors 'self' https://steamloopback.host https://store.steampowered.com/`.
  C'est un refus de Steam, pas une limite de Tauri — inutile de réessayer en iframe. La
  seule voie technique restante serait une WebView enfant native (multiwebview Tauri 2,
  derrière la feature `unstable`), qui flotte **au-dessus** de l'interface Vue, ne défile
  pas avec la page et doit être repositionnée à la main à chaque redimensionnement.
- Le profil s'ouvre donc dans une **fenêtre Torii** (label `torii-web`, réutilisée d'un
  profil à l'autre) au lieu du navigateur — sortir de l'application pour une information
  qu'on venait y chercher, c'était la perdre au passage.
- 🔑 **Liste blanche de domaines** (`DOMAINES_WEB`), pas un simple « c'est de l'HTTPS » :
  la WebView partage la session de l'application, cookies Steam compris. Y charger une
  adresse quelconque venue du front reviendrait à offrir un navigateur — et une session
  connectée — à qui saurait glisser une URL dans une liste d'amis.
- Le front retombe sur `openExternal` si le natif refuse (hors Tauri, domaine hors liste) :
  un clic sans effet serait pire que le navigateur.
- ⚠️ `on_window_event` ne s'applique qu'au label `main`, donc fermer cette fenêtre ne
  déclenche ni la règle du tray ni `exit(0)`.

- **RESTE À FAIRE** : l'application mobile.

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

## Fenêtre et zone de notification

- 🔑 **`close_to_tray` est VRAI par défaut** (`impl Default for WindowPrefs`) : fermer la
  fenêtre garde Torii en tâche de fond. Sinon la détection de parties et la présence
  s'arrêtent dès qu'on range la fenêtre — ce que personne n'associe à un clic sur la croix.
  Décocher la case rétablit une vraie fermeture ; « Quitter » reste dans le menu du tray.
- ⚠️ Un `window_prefs.json` existant garde ses valeurs : le nouveau défaut ne vaut que pour
  les fichiers absents ou incomplets (donc les nouvelles installations).

## Journal et bandeau de notification

- **`journal.rs`** : `logs/torii.log` dans le dossier de config. Ligne au démarrage,
  **paniques Rust** (emplacement + pile), et erreurs d'interface remontées par
  `main.ts` (`window.onerror`, promesses rejetées, `app.config.errorHandler`) via la
  commande `log_front_error`. Rotation à 512 Ko vers `torii.log.1`. Bouton « Ouvrir le
  journal » dans Réglages → À propos.
- 🔑 Ce que le journal NE capte PAS : un arrêt violent du process (violation d'accès,
  plantage WebView2). Le signe est alors **deux lignes de démarrage sans ligne d'arrêt
  entre elles** — d'où la journalisation des arrêts volontaires (tray, fermeture).
- `journal::init` est appelé en TOUT DÉBUT de `setup` : une panique plus tôt ne laisserait
  aucune trace.
- **`toast.rs`** : bandeau en haut à droite, fenêtre sans décoration, `always_on_top`,
  `skip_taskbar`, et surtout **`focused(false)`** — voler le clavier à quelqu'un qui joue
  serait pire que de ne rien afficher. Contenu injecté par `initialization_script`
  (`window.__TOAST__`) plutôt que par la query string : pas d'échappement d'URL, et
  `public/toast.html` écrit le texte avec `textContent` — un pseudo vient d'ailleurs, il
  ne doit jamais être interprété comme du HTML.
- 🔑 Le fond de `toast.html` doit être **opaque** : la fenêtre n'est pas déclarée
  transparente, donc un fond transparent laisse apparaître le blanc par défaut de la
  WebView (cadre clair autour du bandeau). Même raison pour l'absence d'arrondi.
- ⚠️ Un jeu en plein écran **exclusif** masque le bandeau : Steam y arrive en s'injectant
  dans le jeu, ce qu'on ne fait pas. En fenêtré sans bordure, ça marche.
- 🔑🔑 **Ne JAMAIS construire la fenêtre depuis le fil principal.** Une commande Tauri
  synchrone s'exécute sur le fil principal ; y appeler `WebviewWindowBuilder::build()`
  fige la boucle d'évènements, car la création d'une WebView2 attend une réponse que
  seule cette boucle pourrait délivrer. Symptômes vécus, et tous trompeurs : la fenêtre
  **existe** mais reste invisible et non positionnée, l'interface devient blanche, Windows
  la déclare pourtant « répond » (boucle de messages imbriquée), et **aucune ligne de
  journal** n'est écrite après l'appel. `toast::show` fait donc tout sur un fil dédié.
- 🔑 Le placement doit lire `position()` de l'écran, pas seulement `size()` : sur plusieurs
  écrans, celui de gauche a des coordonnées **négatives** et le bandeau atterrissait sur
  l'écran voisin. L'écran de référence est celui de la fenêtre `main`, à défaut le principal.
- Le journal du bandeau est volontairement bavard (construction / placé / affiché / fermé) :
  c'est une ligne `demande :` **sans suite** qui a désigné le coupable ci-dessus.
- Déclencheur : `social::signaler_lancements` compare le couple (ami, jeu) d'un battement
  à l'autre. 🔑 Le **premier** cercle reçu ne notifie rien (`amorce`) — sinon tous ceux qui
  jouent déjà paraîtraient venir de commencer au démarrage de Torii. Réglage
  `notify_friend_launch`, activé par défaut.

## Cycle de vie d'un compte Torii

### Inscription différée (le compte naît au pseudo, pas avant)

- `POST /v1/auth/verify` avec `deferProfile: true` **ne crée rien**. Il renvoie
  `{ created, needsProfile, signupToken }` ; le compte est créé par
  `POST /v1/auth/signup { signupToken, displayName }`, et seulement là.
- 🔑 C'est ce qui permet à la fenêtre d'inscription (`ToriiSignInDialog`) de verrouiller
  l'étape du pseudo — ni croix, ni Échap, ni clic à côté — **sans piéger personne** :
  fermer Torii à ce moment n'abandonne rien à nettoyer, puisque rien n'existe.
- Le laissez-passer est **signé, pas stocké** : `<adresse base64url>.<expiration>.<HMAC au
  poivre>`, 15 minutes. Aucune table, donc aucune ligne morte à ramasser.
- Rejouer un laissez-passer ne crée pas de doublon : le compte existant l'emporte, sous
  son pseudo d'origine, et on se contente d'ouvrir une session.
- ⚠️ **Sans le drapeau, comportement d'avant à l'identique.** Les clients déjà installés
  chez les joueurs s'inscrivent comme ils l'ont toujours fait. Ne pas casser ça.

### Un compte Steam = un compte Torii

- `steam_id` porte un **index UNIQUE partiel**, et `updateMe` refuse en 409
  (`steam_deja_lie`) avec la marche à suivre. Les deux, pas l'un ou l'autre : le contrôle
  applicatif ne voit pas deux requêtes simultanées.
- Ce que ça réparait : deux comptes portant le même SteamID rendaient les suggestions
  ambiguës, la fusion côté client arbitraire (le premier arrivé absorbe l'identité Steam,
  le second s'affiche en double et paraît mort) et la présence contradictoire.
- ⚠️ Ça reste du **premier arrivé, premier servi** : rien ne prouve encore qu'on possède
  le compte Steam déclaré. La vraie réponse est une connexion Steam OpenID au moment du
  lien — non fait.

### Suppression

- `DELETE /v1/me` efface présence, amitiés (les deux sens), sessions (tous les appareils),
  code de connexion en cours, puis le compte. En `batch()`, donc en transaction.
- 🔑 Suppression **explicite table par table** plutôt que de s'en remettre au
  `ON DELETE CASCADE` : des clés étrangères non appliquées ne lèvent aucune erreur, elles
  laissent juste des lignes orphelines. La cascade reste, comme filet.
- Côté client, le jeton local n'est effacé qu'**en cas de succès** (l'inverse de la
  déconnexion) : sinon un échec réseau laisserait un compte vivant que plus personne ne
  peut atteindre pour le supprimer.
- L'interface demande de **recopier son pseudo**. C'est la seule action indéfaisable de
  l'application, et elle voisine un « Déconnecter » anodin.

## Rapprochement Steam ↔ Torii

- `useTorii.reconcilierSteam()` fait remonter le SteamID dans le compte Torii dès que les
  **deux** connexions existent. Appelé aux trois moments où l'état change : démarrage,
  connexion Torii, et juste après une connexion Steam (`AccountsSettings.onConnect`).
- 🔑 **Une seule fois**, mémorisé par `steamAutoLinked` dans `social_prefs.json`. Sans ce
  drapeau, « jamais lié » et « visibilité éteinte à la main » sont indiscernables — le
  SteamID est vide dans les deux cas — et on rallumerait à chaque démarrage ce que la
  personne vient d'éteindre. Un défaut se propose, il ne se réimpose pas.
- 🔑 SteamID et visibilité partent **ensemble** : se déclarer visible sans identifiant lié
  afficherait un interrupteur allumé qui ne rapproche rien.

## Jeux détectés hors launcher — `platforms/detected.rs`

Un jeu qui ne vient d'aucun launcher scanné (Genshin, Dofus, un jeu Game Pass, un `.exe`
posé sur le disque) n'existait pas pour Torii : ni « Récemment joué », ni présence chez
les amis. Le surveillant l'adopte désormais tout seul.

- **Classifieur = Windows lui-même.** `HKCU\System\GameConfigStore\Children` est la liste
  de la Game Bar (une entrée par jeu lancé au moins une fois : `MatchedExeFullPath`,
  `WorkingDirectory`, `LastAccessed`). Relevé réel : 278 entrées, **zéro** navigateur /
  Discord / IDE. S'y ajoute la règle de chemin `C:\XboxGames\` (Game Pass). Index en
  mémoire, relu au plus toutes les 30 s — l'entrée d'un jeu **naît à son premier
  lancement**, donc un index périmé doit être rafraîchi avant de conclure « pas un jeu ».
- **Garde-fous** (`plausible`) : dossiers système, clients de launcher (steam.exe,
  upc.exe, riotclientservices.exe…), utilitaires embarqués (crashhandler, updater,
  easyanticheat…).
- **Moteurs partagés** (`runtime_partage` : java/javaw/python/node) : le même exécutable
  fait tourner n'importe quel jeu, donc ni son nom ni son dossier n'apprennent rien.
  C'est là que les autres champs de la fiche Game Bar tranchent — `Title` s'il existe,
  sinon **`Arguments`** (`minecraft` pour Minecraft, relevé en réel). Sans nom
  exploitable (argument long, chemin, tirets) on s'abstient plutôt que d'inventer, et la
  racine surveillée est le **chemin exact du moteur**, jamais son dossier — sans quoi
  tout programme Java du même dossier passerait pour le jeu.
  ⚠️ Écarter `javaw.exe` (ce que faisait la 0.16.0) revient à écarter Minecraft.
- **Titre deviné** (`title_and_root`) : on remonte les dossiers en sautant les étages
  techniques (`Binaries\Win64`, `runtime`…), on s'arrête sur une étagère (`E:\Games`,
  `steamapps\common`, `AppData\Local`), et on retient le dossier qui parle du même jeu
  que l'exécutable. Si aucun ne parle, le **nom le plus informatif** l'emporte entre
  dossier et exécutable (`…\Ubisoft\r6s\RainbowSix.exe` → « Rainbow Six », pas « r6s »).
  Puis `pretty()` : décollage `motMot`, séparateurs, mots parasites finaux (win64,
  shipping, launcher, vulkan…), MAJUSCULES capitalisées sauf sigle d'un seul mot.
- **Correction par IGDB** : `igdb::recognize(titre deviné)` (recherche tolérante + garde-fou
  d'inclusion, ≥ 5 caractères) rend le vrai nom et la jaquette. Tourne sur son propre fil
  et **renomme aussi la cible en cours**, sinon les amis garderaient le titre deviné
  jusqu'au prochain démarrage.
- **Cycle de vie** : entrée `detected:<slug>` dans `detected_games.json` (plateforme
  `detected`, « Hors launcher » côté front), reprise par `scan_all`. L'id ne change
  jamais, même quand IGDB corrige le titre (favoris / masqués / historique / présence en
  dépendent). « Retirer de la bibliothèque » range l'exécutable dans
  `detected_ignored.json` — sans quoi la partie suivante le ferait revenir. Édition
  possible : `update_manual_game` / `remove_manual_game` routent sur le préfixe de l'id.
- Événement `game-detected` (DTO complet) émis deux fois — à la découverte puis après
  IGDB ; `useLibrary.noteDetected` insère/rafraîchit la carte sans re-scanner.
- **Sursis** (`SURSIS`, `MAX_SURSIS` dans `procwatch`) : Windows n'inscrit un jeu dans sa
  liste qu'au moment où il le remarque, ce qui arrive souvent APRÈS le démarrage du
  process à la toute première partie. Un process inconnu au profil de jeu est donc
  rejugé à chaque passage pendant 2 min, et non une seule fois. Table plafonnée à 24
  entrées, le plus ancien évincé au profit du nouveau venu (c'est lui qui vient de
  démarrer).
- **Cible provisoire** (`Target.provisoire`) : un jeu tout juste découvert est tenu hors
  de la présence tant qu'IGDB n'a pas tranché. Sinon les amis reçoivent le titre deviné
  puis le vrai quelques secondes plus tard, et `signaler_lancements` (qui compare les
  titres) leur compte deux lancements. `liberer()` lève la réserve dans TOUS les cas —
  succès, jeu introuvable, panne réseau — sans quoi une recherche infructueuse
  retiendrait la présence pour toute la session.
- Diagnostic : `cargo run --release --example detect` — ce que Torii ferait de la liste
  Game Bar, sans rien écrire. Sur la machine de dev : 31 exécutables encore installés,
  27 déjà dans la bibliothèque, 0 écarté, 3 jeux détectés (Minecraft, Dolphin,
  Rainbow Six).

### « Hors launcher » = une catégorie, deux plateformes (fait)

Ajout manuel et détection automatique sont deux façons d'arriver au même endroit — un jeu
qu'aucun launcher ne fournit. Ils ne font donc plus qu'**une entrée** dans la barre
latérale, un seul libellé sur les cartes, une seule icône et une seule couleur.

- 🔑 **Fusion d'AFFICHAGE seulement. Les identifiants `manual:` et `detected:` restent
  distincts en base, et il ne faut pas y toucher** : ils décident de qui sait éditer et
  supprimer le jeu (`manual::update` / `detected::forget`, qui n'ont pas le même effet —
  le second inscrit l'exécutable chez les refusés), et un id de jeu détecté ne change
  JAMAIS, sous peine d'orpheliner favoris, historique, présence et jeux muets.
- `HORS_LAUNCHER` / `estHorsLauncher` (`data/platforms.ts`) : **une seule définition** des
  membres de la catégorie. La barre latérale, son compteur, le filtre de la grille et les
  menus « Modifier » / « Retirer » s'y réfèrent tous. Deux listes écrites à la main, et un
  jeu finit par apparaître dans la catégorie sans y être éditable.
- Le filtre `horsLauncher` n'est **pas** un `PlatformId` : la liste de la barre latérale
  porte donc un `id` (le filtre) **et** un `icon` (la plateforme dont elle emprunte
  l'icône). Ils coïncident partout sauf ici. ⚠️ `isPlatformView` (AppShell) reste faux pour
  cette vue, donc pas de bouton « Installés uniquement » — sans objet, ces jeux le sont tous.
- Rien à migrer : le filtre courant n'est jamais persisté sur disque (`DefaultFilter` se
  limite à `all` / `recent` / `favorite` / `installed`).
- ⚠️ Ce qu'on perd, assumé : l'icône radar disait « Torii l'a trouvé tout seul ». C'est une
  information sur la mécanique de Torii, pas sur le jeu — et deux icônes de couleurs
  différentes dans une catégorie unique se lisaient comme un défaut d'affichage.

### 🔑 Le sosie : un jeu de launcher pris pour un jeu hors launcher

Le doublon le plus visible de Torii. On achète un jeu, on le lance depuis Steam dans la
foulée ; la bibliothèque date d'avant l'achat, donc `match_id` ne le reconnaît pas et
`adopt` le classe « Hors launcher ». Le scan suivant ajoute le **vrai** jeu Steam à côté
de son sosie : le même jeu, deux fois, pour toujours.

Deux réponses, et il faut les deux — l'une évite le mal, l'autre répare ce qui existe déjà.

- **Prévention** (`procwatch::jeu_de_launcher_frais`) : avant de déclarer un exécutable
  hors launcher, on **rejoue le relevé des jeux installés** et on regarde s'il tombe
  dedans. 🔑 C'est possible parce que `platforms::scan_installed` ne coûte **aucun appel
  réseau** — manifestes et registre seulement. Un vrai rafraîchissement, lui, interroge
  Steam, GOG et Epic en ligne : impensable au moment où une partie démarre.
  Si c'est un jeu de launcher, on l'adopte sous sa **vraie identité** (`Adoption::Rattrape`)
  : la présence annonce le bon titre, la carte apparaît tout de suite, et **rien** n'est
  écrit dans `detected_games.json`. La cible étant posée, le relevé ne se rejoue pas au
  passage suivant — sans quoi il tournerait à chaque tick pendant les deux minutes de sursis.
- **Filet** (`platforms::est_un_sosie`, appelé par `scan_all`) : un jeu détecté dont le
  dossier vit sous celui d'un vrai jeu est écarté **et effacé**. C'est ce qui répare les
  doublons déjà en base. ⚠️ Son exécutable ne rejoint **pas** les refusés — contrairement à
  `forget` : le jeu n'est pas indésirable, il est déjà connu par ailleurs. L'y mettre
  empêcherait une détection légitime le jour où il serait désinstallé du launcher.
- ⚠️ `normalize` / `sous` sont désormais **définies une seule fois**, dans `platforms`.
  Elles l'étaient en trois exemplaires (procwatch, detected, mod) dont un qui ne coupait
  pas le `\` final. Prévention et filet doivent comparer les chemins à l'identique : deux
  versions qui divergent, et l'une déclare hors launcher ce que l'autre reconnaît.
- `est_un_sosie` est isolée et testée parce que c'est **la seule règle de l'application qui
  supprime une entrée de bibliothèque**. Le test qui compte est celui du dossier voisin :
  « Portal 2 Demo » ne doit pas disparaître parce que « Portal 2 » existe.

## Durcissement du proxy — cache, limites par IP, listes blanches (fait)

Préparation d'une diffusion large. Le constat de départ : `PROXY_TOKEN` était « optionnel »
et n'avait **jamais été posé en production** — le proxy IGDB/ITAD répondait 200 à n'importe
quel `curl`. Et il ne pouvait pas en être autrement : son URL part en clair dans chaque
binaire installé, donc aucun secret partagé ne distingue Torii d'un script.

- 🔑 **Deux défenses, pas trois.** Le **cache** (une réponse servie depuis le cache ne
  coûte ni appel IGDB, ni token Twitch — et elle sert aussi les joueurs, puisque tout le
  monde demande les mêmes jeux populaires) et une **limite par IP**. Le reste est du
  théâtre. `PROXY_TOKEN` est conservé comme interrupteur d'urgence, et le README dit
  désormais qu'il n'est pas une protection.
- **IGDB n'avait aucun cache** (seul `/itad/` en avait un). Une requête POST ne se cache
  pas telle quelle : la clé est une URL GET synthétique `<origin>/__cache/igdb/<endpoint>?q=<sha256(corps)>`.
  🔑 Bâtie sur l'`origin` **entrante** — le cache Cloudflare est cloisonné par zone, une
  clé pointant ailleurs ne serait jamais relue.
- ⚠️ Contrairement à ce que laisse croire la doc Cloudflare, le cache **fonctionne bien
  sur `*.workers.dev`** (vérifié : `CF-Cache-Status: HIT` sur le déploiement réel). Pas
  besoin de domaine personnalisé pour ça.
- `RL_AMONT` (600/min) n'est décomptée **qu'en cas de miss** — un joueur croise surtout des
  jeux déjà demandés par d'autres, un script fabrique des requêtes inédites et paie
  chacune des siennes. `RL_TOTAL` (900/min) compte tout et protège le quota de requêtes du
  compte (100 000/jour, **partagé avec `torii-api`**).
- 🔑 CALIBRAGE : le client se throttle à 300 ms/appel (`CALL_DELAY_MS`) et une recherche par
  nom coûte 2 appels → plafond légitime ≈ **200 appels/min**, au tout premier lancement
  seulement. Les limites laissent 3× de marge (colocation, réseau familial).
- ⚠️ **wrangler 4 obligatoire** : wrangler 3 ignore *silencieusement* `[[ratelimits]]` et
  déploie un Worker sans limite (`--dry-run` affiche « No bindings found »). Le Worker
  répond donc **503** si les bindings manquent, plutôt que de tourner grand ouvert — c'est
  précisément l'erreur de `PROXY_TOKEN` qu'on ne refait pas.
- Listes blanches : les endpoints ITAD sont désormais bornés comme ceux d'IGDB (relevés sur
  **tout l'historique** de `store.rs`, pas seulement la version courante — les anciens
  clients doivent continuer de marcher). Corps borné à 16 Ko, vérifié **après lecture**
  aussi (un envoi `chunked` n'annonce aucune taille). Pas d'en-têtes CORS : aucun client
  n'est un navigateur, et les annoncer aurait permis à une page tierce de faire marteler le
  proxy par le navigateur de ses visiteurs — donc depuis autant d'IP différentes.

### 🔑 Le piège corrigé au passage : une panne était mémorisée comme une absence

`igdb_meta_cache_v1.json` est un `HashMap<String, Option<IgdbMeta>>` où `None` signifie
« cherché, absent d'IGDB — ne plus chercher ». Or `query()` renvoyait `Option` et
**écrasait la différence** entre « IGDB a répondu, rien trouvé » et « l'appel a échoué » :
un Wi-Fi coupé au premier lancement privait définitivement les jeux concernés de
description, genre et jaquette. Le cache n'a pas d'expiration — c'était sans retour.

Poser une limite sans corriger ça aurait transformé chaque 429 en dégât permanent.

- `query` renvoie maintenant `Reponse::{Corps, Panne, Limite}` ; `name_meta` renvoie
  `Issue::{Trouve, Absent, Panne, Limite}`. **Seul `Absent` entre en cache.**
- `steam_metas` renvoie une `PasseSteam { metas, interroges, limite }` : `interroges` liste
  les appids réellement tranchés. Un appid absent d'un lot dont l'appel a échoué reste
  inconnu ; un appid qu'`external_games` n'a rattaché à rien est, lui, vraiment absent.
- Sur `Panne` au 1ᵉʳ appel de `name_meta`, on ne tente pas le repli `search` : un « rien
  trouvé » au second appel ne prouverait rien de plus, et mémoriserait une absence fausse.
- `Limite` interrompt la passe (les appels suivants seraient refusés pareillement et
  compteraient quand même), et la passe incomplète est tracée dans le journal.

## Durcissement de l'API sociale (fait) — sessions, ménage, limites, appareils

- **Les sessions ne mouraient jamais.** `authenticate` ne regardait pas l'âge du jeton,
  `last_seen_at` n'était jamais mis à jour, rien ne purgeait la table. Désormais
  `SESSION_TTL` = 6 mois **d'inactivité** (Torii bat le cœur toutes les 30 s : un appareil
  utilisé ne tombe jamais), la session morte est supprimée au moment où on la découvre, et
  le ménage nocturne ramasse le reste.
- 🔑 `REFRESH_MIN` = 24 h, et **surtout pas une écriture par requête** : à 30 s de
  battement ce serait 2 880 écritures/jour et par appareil, contre 100 000/jour offertes
  pour TOUT le service. Une écriture par appareil et par jour suffit à distinguer un
  appareil vivant d'un appareil abandonné.
- **Écran « mes appareils »** (Réglages → Réseau Torii) : `GET /v1/sessions`,
  `DELETE /v1/sessions/{id}`, `DELETE /v1/sessions` (tous les autres). Colonne `id`
  ajoutée (migration `0002`) — un identifiant **public**, distinct de `token_hash` qui ne
  sort jamais du serveur. `current` marque cet appareil : sans lui, le seul faux pas
  possible de l'écran serait de se déconnecter soi-même.
  ⚠️ Ne pas confondre avec les appareils de `libraryIndex` juste au-dessus dans les mêmes
  réglages : ceux-là ont **déposé une bibliothèque**, ceux-ci ont une **session ouverte**.
- **Trois limites par IP** (`RL_API` 300/min, `RL_CODE` 5/min, `RL_CODE_GLOBAL` 20/min sur
  une clé unique). 🔑 La globale n'est pas un doublon : les garde-fous de `login_codes`
  sont **par adresse e-mail**, donc 10 000 adresses les traversent sans en déclencher un
  seul, et une attaque distribuée a autant d'IP qu'elle veut. Ce qu'on protège, c'est le
  compte Resend — suspendu, plus personne ne se connecte, comptes existants compris.
- **HMAC** pour le laissez-passer d'inscription, au lieu de `SHA256(poivre + message)`.
  ⚠️ NE PAS étendre `hmac` aux jetons ni aux codes : `hash` y range des valeurs à forte
  entropie qu'on ne fait que comparer, et changer sa formule invaliderait d'un coup toutes
  les sessions ouvertes. HMAC ne sert que là où le CLIENT rapporte le message.
- **`body()` borne la lecture après coup**, pas seulement sur `content-length` : un envoi
  `chunked` n'annonce aucune taille et traversait l'ancien contrôle. 🔑 `updateMe` est le
  seul appelant qui distingue « corps vide » de « corps illisible » — les autres finissent
  de toute façon en 400 faute du champ attendu, alors que lui répondait **200 avec le
  compte inchangé** : un appel refusé qui a l'air d'avoir réussi.
- Déclencheur cron `17 4 * * *` → `menage()`. `presence` n'est purgée qu'au bout d'un mois :
  une ligne par compte, réécrite au même endroit, donc elle ne croît pas — l'effacer plus
  tôt coûterait une écriture pour la recréer au retour de la personne.

## Accessibilité (fait) — langue, contrastes, focus, modales

- **`<html lang="fr">`** ([index.html](index.html)). C'était `en` : un lecteur d'écran lisait
  toute l'interface avec une voix anglaise. Une ligne, et c'est la plus rentable du lot.
- **Contrastes.** Mesurés dans l'application qui tourne, pire cas sur les quatre fonds
  (`--bg`, `--surface`, `--surface-2`, `--surface-3`) :

  | | avant (pire cas) | après |
  |---|---|---|
  | `--text-faint` sombre | 3,01 | **4,60** |
  | `--text-faint` clair | 2,69 | **4,54** |
  | `--accent` comme texte, clair | 3,31 | **4,60** |
  | `--accent-ink` sur `--accent`, clair | 3,74 | **5,45** |

  🔑 Vérifier sur `--surface-3` et pas sur `--bg` : un texte peut passer sur le fond de page
  et échouer dans un menu, et c'est le cas qu'on ne voit jamais à l'œil.
  ⚠️ L'accent du thème CLAIR a été assombri (#ec4b30 → #c63118) : le blanc posé dessus ne
  passait pas sur les boutons d'action, et l'accent sert aussi de couleur de TEXTE à
  49 endroits. Le thème sombre était déjà bon (6,7:1) et ne bouge pas. Remonter
  `--text-faint` rapproche forcément « faint » de « dim » — c'est le prix d'un texte
  lisible, pas un réglage à défaire.
- **`outline: none` retiré des 9 champs de saisie.** Ils retombent sur l'anneau global
  `:focus-visible` de `style.css` : un seul style de focus pour toute l'application, au
  lieu d'un simple changement de bordure — qui était le seul indice dans la recherche.
- **`useFocusTrap`** ([src/composables/useFocusTrap.ts](src/composables/useFocusTrap.ts)),
  posé sur 6 surfaces : AddGameModal, ToriiSignInDialog, SettingsView, GameDetail,
  StoreGameDetail, confirmation de retrait d'ami. Trois responsabilités, et il faut les
  trois : entrer (le focus va dans la modale), tourner en rond (Tab du dernier revient au
  premier), **rendre** (à la fermeture, le focus retourne au bouton qui a ouvert).
  ⚠️ Le conteneur doit porter `tabindex="-1"` — repli quand la modale n'a aucun élément
  focusable, sans quoi le focus retombe sur `<body>`. Écouteur en **capture** sur le
  document : un champ de la modale peut traiter Tab lui-même, on décide avant lui.
- **`role="status"` + `aria-live="polite"`** sur le conteneur des toasts. 🔑 Sur le
  CONTENEUR, qui existe en permanence : une région live ajoutée en même temps que son
  contenu n'est pas annoncée.
- Échap ferme le menu des catégories et rend le focus au bouton. Il ne se fermait qu'au
  clic à côté — un geste que le clavier n'a pas.
- ⚠️ **Correction d'un constat d'audit erroné** : les menus déroulants (catégories,
  lancement, écrou, présence) SONT accessibles au clavier — ce sont de vrais `<button>`
  portant déjà `aria-expanded`. Il n'y manquait que `aria-haspopup` et Échap. Ne pas
  repartir de l'idée qu'ils sont à refaire.

## Durcissement de la WebView (fait) — CSP et portée du protocole `asset`

`tauri.conf.json` avait `"csp": null` et `assetProtocol.scope: ["**"]` : aucune barrière
dans la WebView, et la possibilité d'y lire **n'importe quel fichier du disque**. Rien
d'exploitable — un seul `v-html`, sur des SVG locaux — mais dans Tauri une injection front
donne accès à `invoke`, donc au lancement de processus. C'est le genre de dette qu'on paie
le jour où quelqu'un ajoute un `v-html` de trop.

- **La directive qui compte, c'est `script-src 'self'`** : sans `'unsafe-inline'`, un
  `<img onerror=…>` injecté ne s'exécute pas. Le reste est du renfort.
- ⚠️ **`img-src` reste large (`https:`), et c'est délibéré.** Les jaquettes viennent de CDN
  qu'on ne construit pas : Steam, IGDB, GOG, Epic, ITAD, Instant Gaming renvoient leurs URL
  dans leurs réponses d'API, et leurs noms d'hôtes changent. Une liste blanche incomplète =
  des jaquettes manquantes chez certains, en silence. Un hôte d'images n'exécute rien : on
  échange une protection quasi nulle contre un risque de régression réel.
- `connect-src 'self' ipc: http://ipc.localhost` peut, lui, être serré : **le front ne fait
  aucun `fetch`** (vérifié), tout passe par le pont Tauri.
- 🔑 **`dangerousDisableAssetCspModification: ["style-src"]`** — le piège du lot. Tauri
  ajoute ses propres hachages aux directives, et la spec CSP dit qu'une directive
  contenant un hash **ignore `'unsafe-inline'`**. Or l'application a 13 liaisons `:style`
  (le dégradé de chaque jaquette, la position du menu contextuel…) : si `'unsafe-inline'`
  devenait inerte, la grille perdrait tous ses fonds. Le drapeau interdit à Tauri de
  toucher `style-src`, donc notre `'unsafe-inline'` reste effectif.
- `devCsp` est posée en parallèle (Vite + HMR : `'unsafe-inline'`, `'unsafe-eval'`,
  `ws://localhost:1420`) pour que `npm run tauri dev` continue de marcher.
- **Portée `asset` = les extensions d'images, pas `**`.** La liste reflète exactement le
  filtre du sélecteur de jaquette (`AddGameModal.browseCover`). Les motifs sont écrits en
  classes de caractères (`**/*.[jJ][pP][gG]`) parce que les globs sont **sensibles à la
  casse** et qu'un chemin peut être saisi à la main. Ça ne protège pas d'une lecture
  d'image, mais ça ferme `credentials.dat`, les documents et le code source.
  ⚠️ Ne pas restreindre à un dossier : la jaquette d'un jeu manuel est un fichier que
  l'utilisateur choisit **où il veut** et qui reste sur place (cf. `displayableCover`).
- Vérifié : le `dist/index.html` produit ne contient **aucun script ni style en ligne**
  (un `<script src>` et un `<link rel=stylesheet>`), donc `script-src 'self'` passe sans
  hachage. Et la politique appliquée telle quelle dans un navigateur ne produit **aucune
  violation** sur la grille, une fiche de jeu et les Paramètres.

**✅ Vérifié sur un vrai binaire de production** (`npm run tauri build`, `dev=0`, lancé seul
le 2026-09-06) : « démarrage » à 09:00:50, « bibliothèque synchronisée (848 jeux) » à
09:00:55, et cinq fichiers de configuration réécrits — dont `library_cache_v1.json` **cinq
secondes** après le lancement, contre sept au démarrage de référence. Le front atteint donc
bien le natif à travers l'IPC sous la CSP de production.

⚠️ **Reste non exercé : la portée `asset`.** Il n'y a aucun jeu manuel avec une jaquette
locale sur la machine de dev, donc le protocole `asset` n'a jamais été sollicité. Si les
motifs étaient faux, le symptôme serait une jaquette manquante sur un jeu ajouté à la main
— jamais une application cassée.

### 🔑 `cargo build --release` NE teste PAS la CSP de production

Piège coûteux, qui m'a fait annoncer à tort que la CSP cassait l'application. `tauri-build`
émet `cargo:rustc-cfg=dev` pour **tout** build lancé hors du CLI Tauri — `cargo check`,
`cargo build`, **et `cargo build --release`**. Et `dev` veut dire que c'est la `devCsp`,
la permissive, qui s'applique. Un binaire release construit à la main ne prouve donc
**rien** sur la politique réelle.

Le vérifier en une commande, avant de conclure quoi que ce soit :

```bash
for d in src-tauri/target/release/build/ludo-*/; do
  printf '%s dev=%s\n' "$d" "$(grep -c 'cfg=dev' "$d/output")"
done
```

Seul `npm run tauri build` produit un `dev=0`. ⚠️ Il finit en erreur sur la signature de
l'updater sans `TAURI_SIGNING_PRIVATE_KEY` — mais **le `.exe` est écrit avant** cette
étape, donc il reste testable (les bundles MSI/NSIS aussi).

Corollaire à retenir : chercher la politique dans le `.exe` ne marche pas non plus, les
assets du front y sont **compressés** (ni le titre HTML ni le nom du bundle JS ne s'y
retrouvent). L'absence d'une directive dans le binaire ne prouve rien.

## Cadence adaptative du battement (fait) — le plafond passe de ~34 à ~160 joueurs

Le battement était à **30 s en permanence**, et chacun fait une requête Worker **et** une
écriture D1 : 2 880 par jour et par joueur laissant Torii ouvert, contre 100 000/jour
offertes pour *chacune* de ces deux ressources — donc **~34 joueurs allumés en continu**.
⚠️ Ce n'était pas une découverte : `server/README.md` écrivait déjà « soit une trentaine de
testeurs ». Le calcul était posé, il n'avait simplement jamais été suivi d'effet.

- **Trois rythmes** (`cadence_pour`, pur et testé, comme `presence_for` juste à côté) :
  **30 s** si quelqu'un joue (moi ou un ami), **90 s** si des amis sont en ligne sans
  jouer, **5 min** si personne n'est en ligne. Journée réaliste (2 h de jeu, 4 h avec des
  amis, 18 h seul) : ~620 requêtes/jour au lieu de 2 880 → **~160 joueurs**, ~350 pour une
  journée sans personne.
- 🔑 **Ce qu'on ne ralentit pas** : le bandeau « un ami lance un jeu ». Son délai est borné
  par le rythme, donc 90 s au pire *quand des amis sont connectés* — le seul moment où ça
  compte. Dès que quelqu'un joue, on est à 30 s. C'est tout l'objet des trois paliers
  plutôt qu'un seul repli.
- 🔑 **La rétention voyage avec le battement** (`Presence.ttl`, borné 60–900 s par le
  serveur). Une valeur fixe ne peut pas convenir aux deux extrêmes : à 90 s un joueur au
  repos disparaîtrait entre deux battements, à 900 s quelqu'un qui ferme Torii en pleine
  partie resterait « en jeu » un quart d'heure. ⚠️ Le champ **absent** = ancien
  comportement (90 s) : les versions déjà installées ne changent pas d'un iota, et un test
  vérifie qu'il ne part pas quand il n'est pas posé.
- ⚠️ **Poule et œuf, résolu par le rythme PRÉCÉDENT** : la rétention doit couvrir l'attente
  qui suit, mais celle-ci dépend du cercle… qui arrive dans la réponse de cette requête. On
  se fie donc au rythme précédent (exactement la durée qu'on vient d'attendre, et le
  meilleur prédicteur), plancher à `NORMAL`. Exception volontaire : en partie on annonce la
  rétention la plus COURTE, pour ne pas rester « en jeu » après avoir fermé Torii.
- 🔑 **L'attente est découpée en tranches de 10 s** et on bat immédiatement si le jeu local
  a changé. Sans ça, lancer un jeu au rythme de repos mettrait cinq minutes à s'afficher
  chez les amis. C'est aussi ce qui rattrape le démarrage : `procwatch` n'a pas encore
  scanné au premier battement, donc un jeu déjà lancé est publié ≤10 s après.
- Le battement a lieu **avant** la première attente : la boucle dormait d'abord, ce qui
  laissait 30 s avant d'apparaître en ligne (et aurait fait 5 min avec les nouveaux rythmes).

Au-delà de quelques centaines de joueurs, il restera le push (WebSocket + Durable Objects)
ou le plan payant (5 $/mois, 50 M d'écritures D1/mois).

## Prochaines étapes

1. **Comparateur de prix** : wishlist (à capter) × CheapShark / IsThereAnyDeal.
2. Temps de jeu Steam local (`localconfig.vdf`).
3. Peupler les genres plus largement sans saturer l'API (pour un filtre catégorie plus riche).

### Diffusion à grande échelle — lots restants

(les quatre lots sont faits ; voir les sections dédiées plus haut)
