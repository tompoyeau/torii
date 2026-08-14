# Changelog

Les notes de chaque version sont extraites automatiquement par la CI (section
`## X.Y.Z` correspondant au tag) et affichées dans la bannière de mise à jour.
Ajoute une section en tête avant de tagger une nouvelle version.

## 0.9.3
- **Barre latérale repensée** : navigation plus nette, icônes officielles des launchers, et un élément sélectionné bien plus lisible.
- **Amis** déplacé dans la barre du haut, avec une pastille indiquant le nombre d'amis en ligne.
- Nouveau clic droit **« Ouvrir l'emplacement du fichier »** sur un jeu installé, pour ouvrir son dossier dans l'explorateur.
- Correction : certains jeux **Epic désinstallés apparaissaient encore comme installés** (ex. Palia) — ils sont désormais correctement détectés.
- L'**icône de profil** en haut affiche maintenant ta vraie identité Steam (pseudo et avatar).
- Nettoyage de l'interface : la fausse barre de stockage a été retirée ; le bouton en bas de la barre indique désormais **« Connecter un launcher »** ou **« Gérer les connexions »** selon l'état de tes comptes.
- Réduction des **faux positifs antivirus** (Windows Defender).

## 0.9.2
- Nouvelle section **Wishlist** : ta liste de souhaits Steam avec, pour chaque jeu, le meilleur prix du moment, la remise et le plus bas prix historique — pour repérer d'un coup d'œil ce qui est en promo.
- Correction de la **liste d'amis Steam** qui restait vide pour les comptes ayant une URL personnalisée.
- Boutique : **flèches de navigation** (et clavier) dans la visionneuse de captures d'écran.
- Nouveau bouton **« Voir dans la boutique »** sur la fiche d'un jeu, pour comparer ses prix en un clic.

## 0.9.1
- Correction de la **liste d'amis Steam vide** chez certains utilisateurs : la session communautaire est désormais régénérée automatiquement (comme la bibliothèque), et la connexion Steam génère un cookie propre même en présence d'une ancienne session héritée d'une version antérieure. Si ta liste d'amis n'apparaissait pas, reconnecte ton compte Steam une fois.

## 0.9.0
- Nouvelle section **En commun** : retrouve tous les jeux que tu partages avec tes amis Steam, avec un filtre multi-amis pour voir ce que vous possédez tous et jouer ensemble.
- Sur chaque fiche de jeu : les **amis qui possèdent aussi le jeu** (cliquables vers leur profil Steam) et, pour les jeux du partage familial, le **nombre de copies disponibles dans la famille**.
- Correction de la **désinstallation des jeux Epic** : Epic s'ouvre désormais directement sur le bon jeu.
- Nouvelles icônes plus reconnaissables pour Riot, Ubisoft Connect et Battle.net.

## 0.8.0
- Nouvelle **Boutique** : découvre des jeux à acheter et compare les prix sur toutes les boutiques PC (Steam, GOG, Epic, Humble, Fanatical…) en euros, avec le plus bas prix historique et un lien d'achat direct. Recherche avec suggestions instantanées. Les prix Instant Gaming sont aussi affichés sur la fiche produit.
- Nouveau panneau **Amis** : retrouve tes amis Steam au même endroit, vois qui est en ligne et à quoi il joue, avec un rafraîchissement en direct.

## 0.7.0
- Dernière session enregistrée au lancement depuis Torii : les jeux sans statistiques (Riot, EA, Battle.net…) affichent désormais leur dernière date de jeu et remontent dans « Récemment joué ».

## 0.6.1
- Fiche de jeu : correction du bandeau de statistiques qui sortait de l'écran quand il y avait des captures, et zone « À propos » élargie.

## 0.6.0
- Métadonnées enrichies via IGDB pour tous les jeux : description, captures d'écran, studio, année et jaquette de secours — y compris pour les jeux hors Steam (Fortnite, Valorant, Battle.net…) qui n'avaient rien jusqu'ici.

## 0.5.0
- Genres via IGDB : le filtre par catégorie fonctionne désormais pour toute la bibliothèque, y compris les jeux hors Steam (Fortnite, Valorant, WoW…).

## 0.4.0
- Bannière de mise à jour repensée (affichage corrigé : boutons pleine largeur, notes lisibles).
- Notes de version enrichies (vrai changelog par version).

## 0.3.0
- Filtre par catégorie : filtre les jeux par genre, combinable avec les plateformes et la recherche.

## 0.2.0
- Menu contextuel (clic droit) sur les jeux : jouer, favori, masquer, désinstaller.
- Ajout manuel d'un jeu (titre, exécutable, jaquette).
- Mises à jour automatiques de l'application.
