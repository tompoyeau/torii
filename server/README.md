# Torii — API du service social

Worker Cloudflare qui porte les **comptes**, les **amis** et la **présence** (« qui joue à
quoi, maintenant »). Base **D1** (SQLite).

> Distinct de `proxy/` — volontairement. Le proxy relaie IGDB/ITAD et ne retient rien ;
> celui-ci détient des comptes et des relations. Secrets, risques et rythme de déploiement
> différents, donc deux Workers.

## Ce que le service garantit

- **Aucun mot de passe** n'existe : on se connecte avec un code à 6 chiffres reçu par
  e-mail. Rien de réutilisable n'est stocké côté serveur.
- **Rien de secret en clair** : codes de connexion et jetons de session ne sont conservés
  que hachés (SHA-256 + poivre serveur `PEPPER`).
- **Aucun historique de jeu.** La présence porte sa date de péremption et n'est jamais
  archivée. On ne peut pas reconstituer qui a joué à quoi la semaine dernière — c'est
  volontaire, et c'est ce qui rend cette base peu intéressante à voler.
- **Pas d'annuaire.** On ajoute un ami par **code d'ami**, jamais par e-mail : sinon
  tester une liste d'adresses suffirait à savoir qui utilise Torii. Les suggestions par
  SteamID exigent que **les deux** personnes se soient rendues découvrables.

## Mise en place (une seule fois)

Prérequis : `npx wrangler login` (déjà fait si tu as déployé `proxy/`).

```bash
cd server
npm install
```

### 1. Créer la base

```bash
npx wrangler d1 create torii
```

Reporte le `database_id` renvoyé dans `wrangler.toml`, puis crée les tables :

```bash
npx wrangler d1 execute torii --remote --file=schema.sql
```

### 2. Poser le poivre

Chaîne aléatoire longue, jamais commitée. Sans elle le Worker refuse de démarrer.

```bash
npx wrangler secret put PEPPER
```

> Générer une valeur : `node -e "console.log(crypto.randomUUID()+crypto.randomUUID())"`
>
> ⚠️ **La changer déconnecte tout le monde** et invalide les codes en cours : toutes les
> empreintes stockées deviennent incomparables. C'est une valeur qu'on pose une fois.

### 3. Déployer

```bash
npx wrangler deploy
```

## L'envoi des e-mails

C'est la seule dépendance externe, et elle demande **un domaine à toi** : on ne peut pas
expédier depuis `*.workers.dev` (ni SPF ni DKIM n'y sont possibles).

Le domaine du projet est **`topo-host.com`**, avec un sous-domaine par application.

```bash
npx wrangler email sending enable topo-host.com
npx wrangler secret put EMAIL_FROM      # torii@topo-host.com
```

puis décommente le bloc `[[send_email]]` dans `wrangler.toml` et redéploie.

### En attendant : le mode développement

```bash
npx wrangler secret put DEV_CODES       # valeur : 1
```

`POST /v1/auth/request-code` renvoie alors le code **dans la réponse HTTP** au lieu de
l'envoyer. Tout est testable sans domaine.

> ⚠️ **Jamais en production.** Actif, ce mode laisse n'importe qui se connecter avec
> n'importe quelle adresse : il suffit de lire la réponse. À retirer
> (`npx wrangler secret delete DEV_CODES`) dès que l'envoi réel fonctionne.

## Le nom de domaine de l'API

L'API répond sur **`torii-api.topo-host.com`** (route à ajouter sur le Worker, onglet
*Settings → Domains & Routes*). Cette adresse est codée en dur dans chaque version
installée de Torii : passer par un domaine à soi, et non par `*.workers.dev`, permet de
déménager plus tard sans republier l'application chez tout le monde.

> ⚠️ **Un seul niveau de sous-domaine.** Le certificat SSL gratuit de Cloudflare couvre
> `topo-host.com` et `*.topo-host.com`, mais **pas** `*.torii.topo-host.com` : une adresse
> comme `api.torii.topo-host.com` exigerait un certificat payant (Advanced Certificate
> Manager). D'où `torii-api.topo-host.com` plutôt que `api.torii.topo-host.com`.
>
> Convention proposée pour la suite : `torii.topo-host.com` reste libre pour un site de
> présentation, `<app>-api.topo-host.com` pour les API des autres projets.

## Développer en local

```bash
npx wrangler d1 execute torii --local --file=schema.sql
npx wrangler dev --local
```

Les secrets locaux vivent dans `.dev.vars` (ignoré par git) :

```
PEPPER=poivre-de-dev-non-secret
DEV_CODES=1
```

## Routes

Tout est préfixé `/v1` : l'auto-updater fait cohabiter des versions de Torii pendant des
semaines, et l'application mobile viendra s'y brancher.

Les routes privées attendent `Authorization: Bearer <jeton>`.

| Route | Rôle |
|---|---|
| `POST /v1/auth/request-code` | Envoie un code à une adresse. Réponse identique que le compte existe ou non. |
| `POST /v1/auth/verify` | `{ email, code, device }` → jeton de session. Crée le compte à la première connexion. |
| `POST /v1/auth/logout` | Révoque la session courante (les autres appareils restent connectés). |
| `GET /v1/me` | Le compte connecté. |
| `PATCH /v1/me` | Nom affiché, SteamID, découvrabilité. |
| `GET /v1/friends` | Amis (avec présence), demandes reçues, demandes envoyées. |
| `POST /v1/friends/invite` | `{ friendCode }`. Inviter quelqu'un qui nous a déjà invité vaut acceptation. |
| `POST /v1/friends/respond` | `{ accountId, accept }`. |
| `DELETE /v1/friends/{id}` | Retire un ami ou annule une demande. |
| `POST /v1/friends/code` | Régénère son code d'ami (l'ancien cesse de marcher). |
| `POST /v1/friends/suggestions` | `{ steamIds }` → ceux qui sont sur Torii **et** découvrables. |
| `PUT /v1/presence` | Publie son état **et renvoie le cercle complet**. |
| `DELETE /v1/presence` | Disparaître immédiatement (mode invisible). |

### Pourquoi `PUT /v1/presence` renvoie les amis

Le client bat le cœur toutes les 30 s. Faire de ce battement la lecture du cercle divise
le trafic par deux — une requête au lieu de deux. À 30 s d'intervalle, une personne
génère ~2 880 requêtes/jour ; le forfait gratuit Cloudflare en offre 100 000, soit une
trentaine de testeurs. Au-delà, il faudra passer au push (WebSocket + Durable Objects,
plan payant).

## États de présence

| État | Sens exact |
|---|---|
| `in-game` | Un jeu de la bibliothèque tourne (détecté par `procwatch`, tous launchers). |
| `online` | Torii est ouvert, aucun jeu détecté. |
| `away` | Torii est ouvert mais la machine est inactive. |
| `offline` | **Aucune présence reçue depuis 90 s** — donc Torii fermé, PC éteint ou hors ligne. Ça ne veut pas dire « ne joue pas ». |

## À venir

La synchronisation de bibliothèque (pour l'application mobile) n'existe pas encore. C'est
une donnée durable et volumineuse, à l'opposé de la présence : elle aura sa propre table,
versionnée par appareil source, pour que deux PC ne s'effacent pas mutuellement.
