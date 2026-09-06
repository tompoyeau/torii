# Le site de téléchargement de Torii

Page statique publiée sur `torii-app.fr` (hébergement OVH `hosting-free-100m`).
Aucune dépendance, aucun build : ce dossier **est** le site.

## Mettre en ligne

L'offre n'inclut **pas d'accès SSH** : le dépôt se fait par FTP, avec les identifiants de
l'espace client OVH.

| | |
|---|---|
| Serveur | `ftp.cluster121.hosting.ovh.net` (port 21) |
| Destination | le dossier **`www`** |
| À envoyer | tout le contenu de `site/`, en conservant `captures/` et `demo/` |

Le plus simple est FileZilla ou l'explorateur de fichiers de l'espace client. Tant que le
domaine n'est pas rattaché (24 à 48 h après la commande), le site répond sur
`toriian.cluster121.hosting.ovh.net`.

⚠️ **Ne pas oublier le `.htaccess`** : c'est un fichier caché, la plupart des clients FTP
ne l'affichent pas par defaut. Sans lui, pas de redirection vers HTTPS.

### Le SSL est deja actif — mais il ne suffit pas

OVH emet automatiquement un certificat Let's Encrypt pour tout nouveau domaine rattache a
un nouvel hebergement. Il n'y a donc **rien a activer** dans l'espace client : le menu
« Activer le certificat SSL » est vide parce que c'est deja fait.

🔑 **Mais un certificat actif ne force personne a l'emprunter.** `http://torii-app.fr`
repondait 200 en clair, et le navigateur affichait « Non securise » sur la page de
telechargement — le pire endroit possible. C'est le `.htaccess` qui corrige ca, pas le
panneau OVH, qui n'offre aucune option pour le faire.

## Comment le bouton connaît la dernière version

Le nom de l'installeur contient le numéro de version (`Torii_0.19.0_x64-setup.exe`) : il
n'existe donc **aucune URL fixe** vers « le dernier installeur ». `telechargement.js`
interroge l'API GitHub au chargement et remplit le bouton, la taille et la date.

🔑 **Il n'y a rien à modifier ici à chaque version.** Le site suit les releases tout seul.

⚠️ `latest.json` — le manifeste que l'auto-updater interroge — contiendrait pourtant
exactement ce qu'il faut, mais les fichiers de release GitHub sont servis **sans en-tête
CORS** : le navigateur refuse de les lire. L'API, elle, répond
`Access-Control-Allow-Origin: *`. Vérifié ; ne pas refaire le détour.

Et la page **fonctionne sans JavaScript** : les liens pointent alors vers la page des
versions. Le script ne fait qu'améliorer.

## La démo en ligne (`demo/`)

`torii-app.fr/demo/` fait tourner **l'application elle-même**, compilée pour le navigateur,
sur une bibliothèque fictive. Un visiteur peut l'essayer avant de télécharger un `.exe`
d'un éditeur qu'il ne connaît pas — ce qui, pour une application non signée, lève l'un des
deux freins (l'autre étant l'alerte SmartScreen).

```bash
npm run build:demo      # → site/demo/, à envoyer par FTP avec le reste
```

🔑 **`--base=./`, pas `--base=/demo/`.** Des chemins relatifs permettent de déposer le
dossier où l'on veut sans reconstruire. Avec un chemin absolu, un dossier renommé casse
tous les liens vers les assets.

🔑 **Aucun drapeau ne distingue la démo de l'application** : c'est le même build que
`npm run dev`. Ce qui les sépare à l'exécution, c'est `hasTauriRuntime()` — la présence du
pont natif. D'où le bandeau « Démo » (`DemoBanner.vue`), qui ne peut donc pas apparaître
par erreur dans l'application installée.

⚠️ **Les données fictives sont PUBLIÉES.** Trois endroits en contiennent, et les trois
portaient de vrais pseudonymes avant la mise en ligne de la démo :

| Fichier | Contenu |
|---|---|
| `src/data/games.ts` | `MOCK_GAMES` — vrais jeux, vraies jaquettes (CDN Steam), stats inventées |
| `src/composables/useFriends.ts` | `MOCK_FRIENDS` — amis fictifs, avatars générés |
| `src/composables/useFriendsCommon.ts` | `MOCK` — les mêmes amis, vue « en commun » |

Les pseudonymes et les couleurs d'avatar sont **les mêmes que sur les captures** du site :
une personne doit se ressembler d'un écran à l'autre.

La coquille `index.html` (racine du dépôt) porte un `noindex` : une application monopage
n'offre qu'un `<div>` vide à un robot, et serait indexée comme une page sans contenu.

## Les captures

Prises dans l'application réelle, puis **anonymisées** avant publication.

🔑 Les pseudonymes et les avatars des amis sont des **données personnelles de tiers** :
ils n'ont pas consenti à figurer sur une page publique. Toute nouvelle capture doit
repasser par cette étape. Les originaux ne sont **pas** dans le dépôt.

La méthode n'est pas le pavé noir — une capture barbouillée ne donne envie d'installer
rien du tout. On détecte les pixels du texte, on les efface avec la couleur du fond, et on
redessine un pseudonyme inventé dans la même police ; les avatars sont remplacés par des
disques dégradés à initiale, identiques à ceux que Torii génère lui-même. Le résultat reste
une capture vendeuse dans laquelle plus aucune personne réelle ne figure.

| Fichier | Vue | Anonymisé |
|---|---|---|
| `bibliotheque.jpg` | la bibliothèque complète (en-tête de page) | pseudo + avatar du propriétaire |
| `grille.jpg` | la grille de jaquettes | — |
| `hors-launcher.jpg` | la barre latérale, 8 plateformes | — |
| `amis.jpg` | la vue Amis | **12 pseudos + 12 avatars** |
| `commun.jpg` | le profil d'un ami, jeux en commun | 1 pseudo + 1 avatar |
| `fiche-jeu.jpg` | la fiche d'un jeu | **9 pseudos + 9 avatars** (pastilles « amis qui possèdent ») |
| `boutique.jpg` | la boutique et les prix | — (aucune personne) |

⚠️ Deux personnages apparaissent sur plusieurs captures : leur nom **et** la couleur de
leur avatar doivent rester identiques partout, sinon la mise en scène se trahit.

Format : 1000 à 1700 px de large, JPEG qualité 82 (70 à 310 Ko pièce). L'hébergement est
limité à 100 Mo.
