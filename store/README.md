# Torii — la Boutique pour la version web

Worker Cloudflare qui rend, à un navigateur, ce que les commandes `store_*` de l'application
rendent à son interface : mêmes réponses, mêmes noms de champs. Steam choisit les jeux, le
comparateur (IsThereAnyDeal) donne le prix.

> Un navigateur ne peut pas faire ce que fait l'application installée : Steam n'autorise pas les
> appels depuis une page, et le proxy IGDB/ITAD non plus, volontairement. Tout ce qui se fait en
> Rust dans `src-tauri/src/metadata/` pour la Boutique se refait donc ici, côté serveur.

Trois Workers, trois rôles :

| Worker | Détient | Rôle |
|---|---|---|
| `proxy/` | les clés IGDB et ITAD | relaie, sans rien composer |
| `server/` | les comptes | sessions, amis, présence, bibliothèques |
| `store/` (celui-ci) | rien | compose la Boutique, à travers les deux autres |

## Ce que le service garantit

- **Aucune clé, aucun compte ici.** ITAD et IGDB passent par le proxy, la vérification d'un jeton
  par `torii-api`, tous deux en service binding.
- **Rien de personnel dans ce qui est mémorisé.** Une réponse ne dépend que de la région, de la
  langue et des revendeurs masqués : ce qu'un utilisateur a demandé sert aux autres.
- **Aucun HTML tiers ne sort d'ici.** Les descriptions et configurations de Steam sont converties
  en blocs typés ; seules les images et vidéos des serveurs de Steam sont gardées.

## Le quota du comparateur

🔑 **C'est la ressource la plus fragile, et elle n'est pas à nous seuls.** La clé ITAD est la même
pour le web et pour toutes les applications installées. Mesuré le 01/11/2026 : une vingtaine
d'appels en 4 secondes la saturent pour tout le monde, pendant une à deux minutes.

| Protection | Où | Ce qu'elle fait |
|---|---|---|
| Cache des réponses | `memo` | Dix minutes pour l'essentiel, par région, langue et revendeurs masqués |
| Cache des prix groupés | `prixGroupes` | Le proxy ne met pas les POST en cache : c'est fait ici |
| Vraie adresse du proxy | `versProxy` | Ce qu'une application PC a demandé sert au web, et inversement |
| Pause après un refus | `enPause` | Plus d'appel pendant `Retry-After`, 60 s par défaut, tout le centre de données le voit |
| Plafond d'envoi | `RL_ITAD` | Dix appels par 10 s pour le service entier |

⚠️ Pour essayer ce Worker sur le vrai comparateur : **pas de rafale**. Au moins 30 secondes entre
deux chargements de page qui ne sont pas déjà en cache.

🔑 **Une réponse incomplète ne se mémorise jamais.** Si une source ne répond pas, la réponse part
quand même, avec l'en-tête `x-torii-partiel: 1`, mais elle ne sera servie qu'à celui qui l'a
demandée. Instant Gaming fait exception : c'est du bonus, lu dans une page qui peut changer, et son
absence ne rend pas une fiche incomplète.

## Routes

Tout est préfixé `/v1`, et tout est en `GET`. Les routes attendent `Authorization: Bearer <jeton>`,
le même jeton de session que `torii-api`.

Paramètres communs : `region` (`FR` ; `US` par défaut), `langue` (`fr` ou `en` ; `en` par défaut),
et `exclus`, répété pour chaque revendeur masqué.

| Route | Commande de l'application | Rend |
|---|---|---|
| `GET /v1/home` | `store_home` | `StoreHome` |
| `GET /v1/shelf?kind=&tag=` | `store_shelf` | `StoreShelfItem[]` |
| `GET /v1/deals?page=&sort=` | `store_deals` | `StoreItem[]` |
| `GET /v1/tags` | `store_tags` | `StoreTag[]` |
| `GET /v1/browse?tags=1,2&sort=&page=` | `store_browse` | `StoreBrowse` |
| `GET /v1/game/{id}` | `store_game` | `StoreGame`, 503 sans le comparateur |
| `GET /v1/extras?appId=&dlc=1,2&tags=3,4` | `store_game_extras` | `StoreExtras` |
| `GET /v1/search?q=` | `store_search` | `StoreItem[]` |
| `GET /v1/suggest?q=` | `store_suggest` | `StoreSuggestion[]` |
| `GET /v1/id-for-steam/{appid}` | `store_id_for_steam` | `{ id }` |
| `GET /v1/share?titre=` | `lien_partage` | `{ lien }` |
| `GET /v1/owned?appId=` ou `?titre=` | `owned_game_sheet` | `{ fiche }` : `OwnedGameSheet`, `null` sans lien sûr avec Steam |
| `GET /v1/meta?appId=` ou `?titre=` | `enrich_game` | `{ meta }` : le descriptif IGDB, `null` s'il ne connaît pas le jeu |
| `GET /v1/players/{appid}` | `steam_current_players` | `{ joueurs }` |

Les types sont ceux de `src/types.ts`. Une erreur rend `{ error, message }`, avec un code stable :
`non_connecte` (401), `trop_de_requetes` (429), `indisponible` (503).

La wishlist n'est pas ici : elle vit encore sur le PC de chacun, et ce Worker ne retient rien.

⚠️ **Une panne de `torii-api` répond 503, pas 401.** Répondre 401 déconnecterait chaque visiteur qui
passe à ce moment-là.

## Mise en place

Prérequis : `proxy/` et `server/` déployés sur le même compte Cloudflare, sous leurs noms actuels.

```bash
cd store
npm install
npx wrangler deploy
```

`wrangler.toml` déclare le domaine `torii-store.topo-host.com` et les pages autorisées à appeler
le Worker depuis un navigateur (`ORIGINES`). Il n'y a aucun secret à poser.

⚠️ wrangler 4 obligatoire : wrangler 3 ignore les limites sans rien dire. Le Worker répond 500
« mal configuré » si un binding manque.

## Développer

```bash
npm test
```

Les tests tournent hors réseau, sous Node, sans rien à installer. Ils reprennent ceux de
`src-tauri/src/metadata/`, avec les mêmes relevés de réponses réelles.

🔑 **Le Rust reste la référence**, et les deux doivent dire la même chose : un changement de règle
dans `store.rs`, `vitrine.rs`, `fiche.rs`, `igdb.rs` ou `instant_gaming.rs` se reporte ici. Les
fonctions portent les mêmes noms, pour qu'on s'y retrouve de l'un à l'autre.

Ce qui diffère, et pourquoi :

- **La région et la langue sont passées en argument** (`ctx`), au lieu d'un état global : ici, les
  requêtes de deux utilisateurs différents tournent dans le même processus.
- **Pas de repli jeu par jeu pour les identifiants ITAD.** L'application le garde pour un proxy
  pas encore à jour ; ici, le proxy est déployé d'avance, et le repli serait une rafale.
- **`appdetails` n'est demandé qu'une fois par fiche**, là où l'application l'appelle deux fois :
  une réponse sert à la fiche Steam, à la description traduite et au repli d'Instant Gaming.
- **La taille d'installation n'est pas demandée** à `api.steamcmd.net` : la fiche Boutique ne l'affiche pas.
