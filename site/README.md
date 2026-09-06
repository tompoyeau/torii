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
| À envoyer | tout le contenu de `site/`, en conservant `captures/` |

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

## Les captures

Prises dans l'application réelle, puis retouchées avant publication :

- `bibliotheque.jpg` — telle quelle (seul le pseudo du propriétaire y figure) ;
- `amis.jpg` — **pseudos et avatars des amis remplacés** par des identités inventées ;
- `fiche-jeu.jpg` — la section « Amis qui possèdent ce jeu » a été **retirée**.

🔑 Ce sont des données personnelles de tiers : les amis n'ont pas consenti à figurer sur
une page publique. Toute nouvelle capture doit repasser par cette étape. Les originaux ne
sont **pas** dans le dépôt.

Format : 1600 px de large, JPEG qualité 90 (~100 à 340 Ko pièce). L'hébergement est
limité à 100 Mo.
