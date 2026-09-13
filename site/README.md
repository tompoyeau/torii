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

🔑 **La démo n'est pas proposée sur un téléphone.** Les trois endroits qui y mènent — la
ligne sous le bouton de téléchargement, la question de la FAQ et la section « essaie sans
installer » — portent la classe `appel-demo`, masquée par
`@media (pointer: coarse) and (max-width: 900px)`. C'est l'interface d'une application de
bureau : sur un écran de téléphone elle n'est pas moins confortable, elle est inutilisable,
et l'ouvrir donne du produit l'exacte mauvaise impression.

Les **deux** conditions sont nécessaires : la largeur seule masquerait la démo à qui
travaille dans une fenêtre étroite sur un ordinateur (il lui suffit d'élargir), et
`pointer: coarse` seul écarterait les tablettes, où elle tient très bien.

⚠️ La règle doit rester **en fin de feuille** : `.essai-libre` porte `display: grid` à
spécificité égale, et l'écrase si on la place plus haut. Vérifié : la section restait
affichée pendant que les deux autres appels disparaissaient.

⚠️ Le lien `/demo/` reste accessible en direct, et la question correspondante reste dans le
JSON-LD (les données structurées ne connaissent pas l'appareil). Si un jour la démo doit se
défendre elle-même sur un téléphone, c'est dans `DemoBanner.vue` que ça se passe.

🔑 **Aucun drapeau ne distingue la démo de l'application** : c'est le même build que
`npm run dev`. Ce qui les sépare à l'exécution, c'est `hasTauriRuntime()` — la présence du
pont natif. D'où le bandeau « Démo » (`DemoBanner.vue`), qui ne peut donc pas apparaître
par erreur dans l'application installée.

⚠️ **Les données fictives sont PUBLIÉES.** Trois endroits en contiennent, et les trois
portaient de vrais pseudonymes avant la mise en ligne de la démo :

| Fichier | Contenu |
|---|---|
| `src/data/games.ts` | `mockGames()` — vrais jeux, vraies jaquettes (CDN Steam), stats inventées |
| `src/composables/useFriends.ts` | `MOCK_FRIENDS` — amis fictifs, avatars générés |
| `src/composables/useFriendsCommon.ts` | `MOCK` — les mêmes amis, vue « en commun » |

Les pseudonymes et les couleurs d'avatar sont **les mêmes que sur la capture** du site :
une personne doit se ressembler d'un écran à l'autre. (Les vues Amis et « en commun » ne
sont plus illustrées par des captures, mais les pseudonymes inventés restent partagés — la
démo, elle, les montre toujours.)

La coquille `index.html` (racine du dépôt) porte un `noindex` : une application monopage
n'offre qu'un `<div>` vide à un robot, et serait indexée comme une page sans contenu.

## La version anglaise (`en/`)

| Français | Anglais |
|---|---|
| `index.html` | `en/index.html` |
| `steam/index.html` | `en/steam/index.html` |
| `notes-de-version/` | `en/release-notes/` (générées, voir plus bas) |

⚠️ **À METTRE EN LIGNE AVEC LA VERSION DE TORII QUI PARLE ANGLAIS, PAS AVANT.** Le bouton
télécharge la dernière release : tant qu'elle est française, un visiteur de la page anglaise
installe une application qu'il ne peut pas lire. Même chose pour la démo reconstruite
(`demo/`), qui passe en anglais sur un navigateur anglais.

🔑 **LES DEUX VERSIONS SONT DEUX COPIES, À TENIR EN PHASE.** Même structure, mêmes
identifiants (`telechargement.js` et `animations.js` s'appuient dessus), mêmes dessins.
Une section ajoutée d'un côté doit l'être de l'autre. Pas de gabarit commun : le site n'a
pas de build, et un moteur de gabarits pour deux pages coûterait plus qu'il ne rapporte.

🔑 **`hreflang` RÉCIPROQUE, OU RIEN.** Chaque page d'une paire liste les deux versions et
`x-default` (qui vise l'anglais), à l'identique des deux côtés — dans le `<head>` et dans
`sitemap.xml`. Une annonce à sens unique est ignorée par les moteurs.

⚠️ **PAS DE REDIRECTION SELON LA LANGUE DU NAVIGATEUR.** Les robots d'indexation se
présentent souvent en anglais : rediriger `/` vers `/en/` rendrait la page française
invisible pour eux. Un lien « English » / « Français » est proposé, jamais imposé.

- **`telechargement.js` lit `<html lang>`** pour écrire le bouton (« Download Torii 0.20.3 »,
  « released September 9, 2026 ») — la langue de la page, pas celle du navigateur.
- **La démo s'ouvre avec `?lang=fr` depuis les pages françaises, `?lang=en` depuis les
  anglaises.** Sans paramètre, elle démarre en anglais — la langue par défaut de
  l'application, quel que soit le navigateur. ⚠️ Un lien vers `demo/` sans `?lang=fr` sur
  une page française enverrait donc ses visiteurs sur une démo anglaise. Le paramètre ne
  s'enregistre pas ; un choix fait dans les Paramètres de la démo passe devant.
- **Une adresse inconnue sous `/en/` tombe sur l'accueil français** : `.htaccess` renvoie
  toutes les 404 vers `/index.html`. Un `ErrorDocument` conditionnel (`<If>`) le réglerait, mais une
  directive mal acceptée par l'hébergement casse le site entier — pas tenté sans pouvoir
  le vérifier sur OVH.

### La capture anglaise

`captures/library-en.jpg` est prise **sur la démo**, pas sur une vraie bibliothèque : rien à
anonymiser, et elle se refait en une commande (`scripts/capture-demo.mjs`, mode d'emploi en
tête du fichier). Même taille que la française (1700×1020), JPEG qualité 82.

⚠️ L'ordre de la bibliothèque fictive est réglé pour elle : le jeu mis en avant est le plus
récent, et VALORANT ou Minecraft (sans jaquette publique) donnaient un bandeau uni et des
cartes vides en tête. Voir le commentaire de `mockGames()` dans `src/data/games.ts`.

## Les pages secondaires (`steam/`, `notes-de-version/`)

L'accueil vend le produit ; ces deux pages répondent à une question précise que l'accueil
ne peut pas viser sans se disperser. Elles partagent `styles.css` — tout ce qu'elles
utilisent y est regroupé sous « PAGES SECONDAIRES », et rien n'y touche à l'accueil.

🔑 **Ce sont des dossiers, pas des fichiers `.html`.** `steam/index.html` donne l'URL
`torii-app.fr/steam/` sans une ligne de réécriture dans `.htaccess`. Les liens internes
portent tous la barre finale : sans elle, Apache répond par une redirection avant de
servir la page.

| Page | Ce qu'elle vise | Écrite |
|---|---|---|
| `steam/` | « voir mes jeux Steam ailleurs que dans Steam » | à la main |
| `notes-de-version/` | ce qui change à chaque version | **générée** |

⚠️ **Les deux pages de notes sont générées — ne pas les modifier à la main.**

| Page | Source |
|---|---|
| `notes-de-version/index.html` | `CHANGELOG.md` (fait foi) |
| `en/release-notes/index.html` | `CHANGELOG.en.md` (traduction) |

```bash
npm run build:notes     # → les deux pages, à envoyer par FTP
```

**À relancer après chaque ajout aux deux changelogs, avant l'envoi FTP.** Les dates
viennent de l'API GitHub (un appel) ; sans réseau les pages se génèrent quand même, sans
les dates.

🔑 **Une version oubliée dans `CHANGELOG.en.md` ne disparaît pas de la page anglaise** :
elle y figure avec son texte français, balisé `lang="fr"` et étiqueté « Not yet
translated », et la génération l'annonce en console. Masquer une release aux anglophones
serait pire qu'une note en français. Vérifié en retirant temporairement la 0.20.3.

⚠️ Les libellés d'interface cités dans les notes anglaises (« No launcher », « Torii
network », « Report a problem »…) sont **ceux de l'application anglaise** : un utilisateur
doit retrouver le bouton dont parle la note.

🔑 **Pourquoi générer, alors que le bouton de téléchargement, lui, interroge l'API dans le
navigateur.** Parce que les deux ne jouent pas le même rôle. Le bouton affiche une donnée
d'une ligne, qu'aucun moteur n'a besoin de lire. Cette page-là n'existe **que** pour son
contenu : trente-cinq versions de prose française, soit la seule partie du site qui change
pour de bon et donne à un robot une raison de repasser. Un texte injecté par JavaScript
dépend du bon vouloir de celui qui passe — et un aperçu de lien partagé n'en voit jamais
rien. Il va donc dans le HTML, en dur.

⚠️ La section « ce que Torii ne fait pas » de `steam/` n'est pas de la modestie : c'est ce
qui distingue une page qui répond d'une page qui répète l'accueil en remplaçant « tes
launchers » par « Steam ». Si elle disparaît, la page devient du remplissage et sera
traitée comme tel.

⚠️ **La FAQ de `steam/` est dupliquée en JSON-LD dans la même page.** Les deux doivent
dire la même chose : un balisage qui ne correspond pas au texte affiché est une raison
d'être ignoré, parfois de se faire sanctionner. Cinq questions des deux côtés, vérifié.

## Une seule capture, et sept dessins

La page a compté sept copies d'écran ; elle n'en garde **qu'une**, celle de l'en-tête.
Les sections sont illustrées par des dessins au trait, écrits en SVG directement dans
`index.html`.

🔑 **Pourquoi.** Une capture par section coûtait trois choses à chaque évolution du
produit : plus d'un mégaoctet à servir, une anonymisation à refaire (voir plus bas), et
une image qui ment dès que l'interface bouge d'un pixel. Un dessin ne montre pas le
produit, il montre l'**idée** de la section — il ne vieillit donc pas, ne pèse rien, et
ne contient aucune donnée personnelle. Mais une page qui ne montre jamais le produit ne
convainc personne : d'où celle de l'en-tête, en grand, et la démo pour le reste.

| Fichier | Vue | Anonymisé |
|---|---|---|
| `captures/bibliotheque.jpg` | la bibliothèque complète (en-tête de page) | pseudo + avatar du propriétaire |

⚠️ **Si une capture est ajoutée un jour, l'anonymisation redevient obligatoire.** Les
pseudonymes et les avatars des amis sont des **données personnelles de tiers** : ils n'ont
pas consenti à figurer sur une page publique. La méthode n'est pas le pavé noir — une
capture barbouillée ne donne envie d'installer rien du tout. On détecte les pixels du
texte, on les efface avec la couleur du fond, et on redessine un pseudonyme inventé dans
la même police ; les avatars sont remplacés par des disques dégradés à initiale, identiques
à ceux que Torii génère lui-même. Les originaux ne sont **pas** dans le dépôt. Les mêmes
noms et les mêmes couleurs servent dans la démo : une personne doit se ressembler d'un
écran à l'autre.

Format : 1700 px de large, JPEG qualité 82 (310 Ko). L'hébergement est limité à 100 Mo.

### Les dessins

Sept `<svg class="illu">` dans `index.html`, sur une grille `viewBox="0 0 460 300"`
commune, plus une petite icône par carte de la section « pourquoi » (`.ico-carte`).

🔑 **Aucune couleur en dur.** Tout passe par les classes définies dans `styles.css`
(`cadre`, `doux`, `fin`, `trait`, `accent`, `plein`, `encre`, `aire`, `barre`,
`barre-faible`), qui pointent vers les jetons de la page. Les dessins suivent donc le
thème clair sans une seule règle de plus — et un changement d'accent les repeint tous.

⚠️ **`--surface-2` posé sur `--surface` ne se voit presque pas.** Dans l'application ces
deux fonds ne se touchent jamais sur un aplat ; dans un dessin, si. Sans le filet posé sur
`.doux`, les jaquettes et les vignettes disparaissent purement et simplement — constaté au
rendu, en sombre comme en clair.

Pour les relire tous en grand pendant qu'on les retouche : les extraire d'`index.html` et
les poser côte à côte dans une page à part. Les vérifier dans la page elle-même est
pénible, parce que l'apparition au défilement les laisse à `opacity: 0` tant que
l'observateur n'a pas déclenché, ce qu'un rendu automatisé ne provoque pas de façon fiable.
