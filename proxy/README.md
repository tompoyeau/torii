# Torii — mini-proxy IGDB + IsThereAnyDeal

Petit Cloudflare Worker qui donne à Torii l'accès à :
- **IGDB** (métadonnées : genres cross-plateforme, descriptions, jaquettes…) sans exposer le secret Twitch ;
- **IsThereAnyDeal (ITAD)** (prix multi-boutiques de la Boutique) sans exposer la clé ITAD.

Même approche que Playnite. Gratuit (offre gratuite Cloudflare Workers).

> ⚠️ Après toute modification (y compris l'ajout de la partie ITAD ci-dessous),
> **redéploie le Worker** : `npx wrangler deploy` depuis ce dossier.

## Mise en place (une seule fois)

### 1. Créer une application Twitch (pour la clé IGDB)

IGDB appartient à Twitch → l'accès passe par une app Twitch.

1. Va sur **https://dev.twitch.tv/console/apps** (connexion avec ton compte Twitch,
   activation de la 2FA requise par Twitch).
2. **Register Your Application** :
   - **Name** : `Torii` (ou ce que tu veux, unique)
   - **OAuth Redirect URLs** : `http://localhost` (non utilisé, mais requis)
   - **Category** : Application Integration
   - **Client Type** : Confidential
3. Récupère le **Client ID**, puis **New Secret** → récupère le **Client Secret**.

### 2. Déployer le Worker sur Cloudflare

Prérequis : un compte **Cloudflare** (gratuit). Depuis le dossier `proxy/` :

```bash
npm install
npx wrangler login          # ouvre le navigateur pour autoriser Cloudflare
```

Poser les secrets (colle les valeurs quand demandé) :

```bash
npx wrangler secret put TWITCH_CLIENT_ID
npx wrangler secret put TWITCH_CLIENT_SECRET
```

### 2 bis. Clé IsThereAnyDeal (pour la Boutique)

1. Va sur **https://isthereanydeal.com/apps/my/** (crée un compte gratuit si besoin).
2. **Register a new app** : nom `Torii`, coche les scopes de lecture proposés, valide.
3. Récupère la **clé d'API** affichée, puis pose-la comme secret (depuis `proxy/`) :

```bash
npx wrangler secret put ITAD_API_KEY
```

Déployer :

```bash
npx wrangler deploy
```

Wrangler affiche l'URL publique, du type
`https://torii-igdb-proxy.<ton-sous-domaine>.workers.dev`.
**C'est cette URL qu'il faut donner à Torii** (voir intégration côté app).

Vérifie au passage que les deux limites sont bien attachées — wrangler les liste à la fin
du déploiement :

```
env.RL_TOTAL (900 requests/60s)      Rate Limit
env.RL_AMONT (600 requests/60s)      Rate Limit
```

⚠️ Si elles n'apparaissent pas, le Worker répond **503** à tout : c'est voulu (voir
« Protection » plus bas). Elles exigent **wrangler 4** ; wrangler 3 ignore silencieusement
la section `[[ratelimits]]`.

### 3. Tester

```bash
curl -X POST "https://torii-igdb-proxy.<sous-domaine>.workers.dev/games" \
  --data 'search "Fortnite"; fields name, genres.name; limit 1;'
```

Réponse attendue : un JSON avec `name: "Fortnite"` et `genres` (dont *Shooter*).
Relance la même commande avec `-D -` : la seconde doit afficher `CF-Cache-Status: HIT`.

## Fonctionnement

- L'app Torii POST une requête **Apicalypse** vers `/<endpoint>` du Worker.
- Le Worker obtient/met en cache un token d'app Twitch (client_credentials, ~60 j),
  ajoute les en-têtes `Client-ID` + `Authorization: Bearer`, et relaie vers
  `api.igdb.com/v4/<endpoint>`.
- Endpoints IGDB autorisés : `games`, `external_games`, `genres`, `covers`, `multiquery`.
- **ITAD** : `/itad/<endpoint>` (GET ou POST) est relayé vers
  `api.isthereanydeal.com/<endpoint>` avec la clé `ITAD_API_KEY` injectée. Endpoints
  autorisés : `deals/v2` (vitrine), `games/search/v1`, `games/info/v2`,
  `games/overview/v2`, `games/prices/v3`, `games/lookup/v1`.
- Les secrets (Twitch + ITAD) restent **côté Cloudflare** (jamais dans l'app ni le dépôt git).
- **Pas d'en-têtes CORS** : aucun client de Torii n'est un navigateur (tout part de Rust,
  et demain d'un client mobile). Les annoncer aurait surtout permis à une page web tierce
  de faire marteler ce proxy par le navigateur de ses visiteurs — autant d'adresses IP
  différentes, donc autant de limites contournées.

## Protection

L'URL de ce Worker part **en clair dans chaque version installée** : elle s'extrait du
binaire en quelques secondes. Aucun secret partagé ne peut donc distinguer Torii d'un
script. Deux mécanismes seulement tiennent la porte :

1. **Le cache.** Une réponse servie depuis le cache ne coûte ni appel IGDB, ni appel ITAD,
   ni token Twitch. C'est la meilleure défense parce qu'elle sert aussi les joueurs : tout
   le monde possède Fortnite ou Minecraft, et la première personne qui les demande paie
   pour toutes les suivantes. IGDB : 24 h (7 jours pour `genres`). ITAD : 10 min pour la
   vitrine, 5 min pour la recherche et les fiches, **rien** pour les prix.
2. **Deux limites par adresse IP.** `RL_AMONT` (600/min) ne compte que ce qui sort
   vraiment vers IGDB ou ITAD — un cache-hit ne consomme donc rien. `RL_TOTAL` (900/min)
   compte tout, et protège le quota de requêtes du compte Cloudflare, que le Worker
   `torii-api` partage avec celui-ci. Dépassement → **429** avec `Retry-After`.

Calibrage : le client se throttle déjà à 300 ms par appel, et une recherche par nom coûte
au plus deux appels — le plafond d'un Torii légitime est donc d'environ **200 appels/min**,
atteint seulement au tout premier lancement d'une grosse bibliothèque hors Steam. Les
limites laissent 3× cette marge, pour que plusieurs joueurs derrière la même IP
(colocation, réseau familial, campus) ne se gênent pas.

`PROXY_TOKEN` (secret optionnel) n'est **pas** une protection : il coupe le proxy pour tout
ce qui n'envoie pas le jeton, donc aussi pour toutes les versions de Torii déjà installées.
C'est un interrupteur d'urgence, à n'utiliser que pour fermer la porte en attendant mieux.

⚠️ L'enjeu n'est pas la facture — tout tient sur l'offre gratuite. C'est que **Twitch
révoque l'application IGDB** en cas d'abus : Torii perdrait d'un coup toute sa métadonnée
descriptive, pour tout le monde, sans recours rapide.

## Limites des API amont

- IGDB : **4 req/s**, jusqu'à **500 résultats/requête** (largement suffisant en batch).
- ITAD : limite non documentée, d'où le cache sur la vitrine et la recherche (les 429
  d'ITAD étaient le symptôme d'origine).
