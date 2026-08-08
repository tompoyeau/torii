# Torii — mini-proxy IGDB + IsThereAnyDeal

Petit Cloudflare Worker qui donne à Torii l'accès à :
- **IGDB** (métadonnées : genres cross-plateforme, descriptions, jaquettes…) sans exposer le secret Twitch ;
- **IsThereAnyDeal (ITAD)** (prix multi-boutiques de la Boutique) sans exposer la clé ITAD.

Même approche que Playnite. Gratuit (offre gratuite Cloudflare Workers).

> ⚠️ Après avoir ajouté la partie ITAD (ci-dessous), **redéploie le Worker** (`npx wrangler deploy`).

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
npx wrangler secret put PROXY_TOKEN        # optionnel : un mot de passe au hasard
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

### 3. Tester

```bash
curl -X POST "https://torii-igdb-proxy.<sous-domaine>.workers.dev/games" \
  -H "x-proxy-token: <ton PROXY_TOKEN si défini>" \
  --data 'search "Fortnite"; fields name, genres.name; limit 1;'
```

Réponse attendue : un JSON avec `name: "Fortnite"` et `genres` (dont *Shooter*).

## Fonctionnement

- L'app Torii POST une requête **Apicalypse** vers `/<endpoint>` du Worker.
- Le Worker obtient/met en cache un token d'app Twitch (client_credentials, ~60 j),
  ajoute les en-têtes `Client-ID` + `Authorization: Bearer`, et relaie vers
  `api.igdb.com/v4/<endpoint>`.
- Endpoints IGDB autorisés : `games`, `external_games`, `genres`, `covers`, `multiquery`.
- **ITAD** : tout chemin `/itad/<endpoint>` (GET ou POST) est relayé vers
  `api.isthereanydeal.com/<endpoint>` avec la clé `ITAD_API_KEY` injectée. Utilisé par la
  Boutique : `deals/v2` (vitrine), `games/search/v1` + `games/prices/v3` (recherche),
  `games/info/v2` + `games/prices/v3` + `games/overview/v2` (fiche produit).
- Les secrets (Twitch + ITAD) restent **côté Cloudflare** (jamais dans l'app ni le dépôt git).

## Limites

- IGDB : **4 req/s**, jusqu'à **500 résultats/requête** (largement suffisant en batch).
- Le `PROXY_TOKEN` limite l'abus casual ; il sera embarqué dans l'app (donc semi-public),
  mais l'exposition se limite à des lectures de données de jeux publiques.
