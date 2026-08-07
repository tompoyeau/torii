# Torii — mini-proxy IGDB

Petit Cloudflare Worker qui donne à Torii l'accès aux métadonnées **IGDB**
(genres cross-plateforme : Fortnite = Shooter, etc.) sans exposer le secret Twitch.
Même approche que Playnite. Gratuit (offre gratuite Cloudflare Workers).

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
- Endpoints autorisés : `games`, `external_games`, `genres`, `covers`, `multiquery`.
- Le secret Twitch reste **côté Cloudflare** (jamais dans l'app ni le dépôt git).

## Limites

- IGDB : **4 req/s**, jusqu'à **500 résultats/requête** (largement suffisant en batch).
- Le `PROXY_TOKEN` limite l'abus casual ; il sera embarqué dans l'app (donc semi-public),
  mais l'exposition se limite à des lectures de données de jeux publiques.
