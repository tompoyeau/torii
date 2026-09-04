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
- **Ce qu'on possède, en revanche, peut être conservé** — et c'est la seule donnée durable
  du service. Une bibliothèque synchronisée dit ce que quelqu'un **possède**, jamais ce
  qu'il joue ni quand. Rien n'est envoyé tant que la synchronisation n'est pas activée, et
  aucun ami ne peut la lire tant que le partage ne l'est pas non plus (deux réglages
  distincts, voir plus bas). Tout s'efface d'un appel, et la suppression du compte emporte
  les objets R2 avant les lignes qui y mènent.
- **Pas d'annuaire.** On ajoute un ami par **code d'ami**, jamais par e-mail : sinon
  tester une liste d'adresses suffirait à savoir qui utilise Torii. Les suggestions par
  SteamID exigent que **les deux** personnes se soient rendues découvrables.
- **Une session n'est pas éternelle.** Elle expire après six mois **sans usage** (Torii
  bat le cœur toutes les 30 s : quelqu'un qui s'en sert n'est jamais déconnecté), et
  `GET /v1/sessions` permet de voir et de fermer les appareils encore connectés. Un
  ménage nocturne efface les sessions mortes et les codes de connexion périmés — les deux
  seules tables dont la taille dépend du nombre de tentatives et non du nombre de comptes.

## Ce qui borne l'abus

Le service tient sur l'offre gratuite, et la ressource la plus fragile n'est pas la base
mais **l'envoi d'e-mails** : un compte Resend suspendu, c'est plus personne qui peut se
connecter, y compris les comptes existants.

- `RL_API` — **300 requêtes/min par IP**, toutes routes. Un client normal en fait deux.
- `RL_CODE` — **5 demandes de code/min par IP**. ⚠️ Les garde-fous de `login_codes` sont
  **par adresse e-mail** : ils protègent la boîte de quelqu'un, pas le service. 10 000
  adresses différentes les traversent sans en déclencher un seul.
- `RL_CODE_GLOBAL` — **20 codes/min pour le service entier**, une seule clé. C'est le seul
  rempart contre une attaque distribuée, où chaque requête vient d'une IP différente.

⚠️ Ces limites exigent **wrangler 4** : wrangler 3 ignore la section `[[ratelimits]]` sans
rien dire et déploie un Worker sans aucune limite. Le Worker répond 500 « mal configuré »
si les bindings manquent, pour que l'oubli se voie tout de suite.

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

### 3. Créer le bucket des bibliothèques

Les bibliothèques synchronisées vivent dans **R2** (un objet JSON par appareil), pas en D1
— voir *Bibliothèques synchronisées* plus bas pour le pourquoi.

```bash
npx wrangler r2 bucket create torii-libraries
```

> ⚠️ **Ne lui donne aucun accès public.** Pas de domaine `r2.dev`, pas de domaine
> personnalisé : le Worker est l'unique porte, c'est lui qui vérifie le jeton, l'amitié et
> le partage. Un bucket public rendrait toutes les bibliothèques lisibles par qui devine
> une URL.
>
> ℹ️ Activer R2 sur un compte Cloudflare demande de renseigner un moyen de paiement, même
> pour rester dans l'offre gratuite (10 Go-mois de stockage, 1 M d'écritures et 10 M de
> lectures par mois, trafic sortant gratuit). Rien n'est facturé tant qu'on reste dessous.

### 4. Déployer

```bash
npx wrangler deploy
```

### Faire évoluer une base déjà déployée

`schema.sql` décrit la base **telle qu'on la créerait aujourd'hui** ; il ne modifie pas une
base existante (tout y est en `IF NOT EXISTS`). Les changements de schéma vivent donc aussi
dans `migrations/`, à appliquer une fois :

```bash
npx wrangler d1 execute torii --remote --file=migrations/0001_libraries.sql
npx wrangler d1 execute torii --remote --file=migrations/0002_sessions.sql
```

> 🔴 **La migration passe AVANT le déploiement du Worker, jamais après.** `0002` ajoute la
> colonne `sessions.id`, que le nouveau code écrit à chaque ouverture de session : déployer
> d'abord, c'est une erreur SQL sur `verify` et `signup` — donc **plus personne ne peut se
> connecter** jusqu'à ce que la migration passe.

## L'envoi des e-mails

C'est la seule dépendance externe, et elle demande **un domaine à toi** : on ne peut pas
expédier depuis `*.workers.dev` (ni SPF ni DKIM n'y sont possibles).

Le domaine du projet est **`topo-host.com`**, avec un sous-domaine par application.

> ℹ️ **Pourquoi pas l'envoi natif de Cloudflare ?** Email Sending exige le plan **Workers
> payant** ; sur le plan gratuit, `wrangler email sending enable` répond
> `Unauthorized [code: 2036]`. Le code sait malgré tout s'en servir (binding `EMAIL`) : le
> jour où le plan change, il suffit de décommenter `[[send_email]]` dans `wrangler.toml`,
> l'ordre de priorité dans `deliver()` fait le reste.
>
> Email **Routing** (gratuit) ne remplace pas l'envoi : il reçoit et transfère, mais un
> Worker ne peut qu'y *répondre* à un message entrant, jamais en initier un.

### Voie retenue : Resend (offre gratuite)

1. Créer un compte sur [resend.com](https://resend.com) — 3 000 e-mails/mois, 100/jour.
2. **Domains → Add Domain** → `topo-host.com`, puis ajouter les enregistrements DNS
   proposés (SPF, DKIM, DMARC) dans Cloudflare. Quelques minutes de propagation.
3. **API Keys → Create**, portée *Sending access* : le strict nécessaire.
4. Poser la clé et l'expéditeur, puis redéployer :

```bash
npx wrangler secret put RESEND_API_KEY   # colle la clé quand c'est demandé
npx wrangler secret put EMAIL_FROM       # torii@topo-host.com
npx wrangler deploy
```

> 🔑 Pose la clé **toi-même** : `wrangler secret put` la lit sur l'entrée standard, elle
> ne transite ni par un fichier du dépôt, ni par l'historique du terminal.

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
| `GET /v1/sessions` | Les appareils connectés à ce compte. Ne renvoie jamais d'empreinte de jeton. |
| `DELETE /v1/sessions/{id}` | Déconnecte un appareil. |
| `DELETE /v1/sessions` | Déconnecte tous les **autres** appareils, jamais celui-ci. |
| `PATCH /v1/me` | Nom affiché, SteamID, découvrabilité. |
| `GET /v1/friends` | Amis (avec présence), demandes reçues, demandes envoyées. |
| `POST /v1/friends/invite` | `{ friendCode }`. Inviter quelqu'un qui nous a déjà invité vaut acceptation. |
| `POST /v1/friends/respond` | `{ accountId, accept }`. |
| `DELETE /v1/friends/{id}` | Retire un ami ou annule une demande. |
| `POST /v1/friends/code` | Régénère son code d'ami (l'ancien cesse de marcher). |
| `POST /v1/friends/suggestions` | `{ steamIds }` → ceux qui sont sur Torii **et** découvrables. |
| `PUT /v1/presence` | Publie son état **et renvoie le cercle complet**. |
| `DELETE /v1/presence` | Disparaître immédiatement (mode invisible). |
| `PUT /v1/library` | Dépose la bibliothèque d'un appareil (`{ deviceId, deviceName, digest, games }`). |
| `GET /v1/library` | Index : mes appareils, et ceux des amis qui partagent. Aucune lecture R2. |
| `GET /v1/library/{compte}/{appareil}` | La bibliothèque elle-même. Gère `If-None-Match` → `304`. |
| `DELETE /v1/library/{appareil}` | Oublie un appareil (objet R2 compris). |
| `DELETE /v1/library` | Cesser de synchroniser : tout part. |

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

## Bibliothèques synchronisées

### Pourquoi R2, et pas une table de jeux

Une ligne par jeu et par personne, c'est ~1 000 lignes écrites à chaque resynchronisation.
Le plan gratuit D1 en offre **100 000 par jour** : une centaine de joueurs et le service
s'arrête, pour 5 Go de stockage. Dans R2, la même bibliothèque est **un seul objet**
(~120 Ko, une opération), les 10 Go gratuits en absorbent des dizaines de milliers, et le
trafic sortant — celui que le mobile consommera — ne coûte rien.

D1 garde en échange un **index minuscule** (table `libraries`) : une ligne par appareil,
avec l'empreinte, la date et le nombre de jeux. C'est ce qui permet de savoir **sans rien
télécharger** si une bibliothèque a changé. Ce que R2 ne sait pas faire — chercher — se
fait côté client, comme `useFriendsCommon` croise déjà les jeux Steam.

### Un objet par appareil

Clé R2 : `lib/<compte>/<appareil>.json`. Deux PC n'ont pas la même bibliothèque (launchers
connectés, jeux installés) ; s'ils écrivaient au même endroit, le dernier passé effacerait
l'autre indéfiniment. « La bibliothèque de quelqu'un » est donc l'**union de ses appareils**,
faite côté client. Cinq appareils par compte au maximum.

### Synchroniser ≠ partager

| Réglage | Où il vit | Ce qu'il ouvre |
|---|---|---|
| **Synchroniser** | côté client (`social_prefs.json`) | Envoyer sa bibliothèque au serveur, pour la retrouver sur ses **propres** appareils (le mobile). Rien ne part tant que c'est éteint. |
| **Partager** | côté serveur (`accounts.share_library`, `PATCH /v1/me`) | Autoriser ses **amis acceptés** à la consulter. Éteint, elle reste lisible par soi seul. |

Éteindre le partage ne supprime rien ; `DELETE /v1/library` si.

### Ce qui est stocké

Par jeu : la clé cross-launcher (`igdb:1942`, sinon `title:<titre normalisé>` — la même que
la présence), le titre, les plateformes, la jaquette. **Rien d'autre** : le serveur
normalise et tronque tout ce qu'il reçoit, et jette les champs qu'il ne connaît pas. Sans
ça, une route authentifiée qui écrit dans R2 devient un hébergement de fichiers gratuit.

### Économie de requêtes

L'empreinte fournie par le client sert d'**ETag**. Un client à jour envoie `If-None-Match`
et repart avec un `304` : ni lecture R2, ni transfert. Combiné à l'index (qui dit ce qui a
bougé avant même de demander), une bibliothèque stable ne coûte pratiquement rien.
