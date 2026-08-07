# Publier une mise à jour de Torii

Torii intègre l'**auto-updater de Tauri**. Une fois la configuration ci-dessous
faite **une seule fois**, publier une nouvelle version se résume à : bump de version
→ tag → push. Tes utilisateurs (ex. ton pote) reçoivent la mise à jour automatiquement
au prochain lancement de l'app.

---

## Comment ça marche

1. L'app, au démarrage, interroge le manifeste
   `https://github.com/tompoyeau/torii/releases/latest/download/latest.json`.
2. Si la version publiée est plus récente que celle installée, une bannière
   « Mise à jour disponible » apparaît en bas à droite.
3. L'utilisateur clique **« Installer et redémarrer »** → l'installeur signé est
   téléchargé, vérifié avec la clé publique, installé, puis l'app redémarre.

La signature garantit qu'une mise à jour ne peut venir que de **toi** (celui qui
détient la clé privée). Sans elle, l'updater refuse le paquet.

⚠️ **La v0.1.0 déjà installée par ton pote ne contient pas l'updater.** Il doit
installer **à la main, une dernière fois**, le premier build produit par la CI
(la v0.2.0). À partir de là, tout est automatique.

---

## Configuration initiale (à faire UNE fois)

### 1. Mettre le projet sur GitHub

```bash
# depuis D:\dev\ludo (déjà un dépôt git local avec un premier commit)
git remote add origin https://github.com/tompoyeau/torii.git
git branch -M main
git push -u origin main
```

> Le dépôt peut rester **privé**, mais les **Releases doivent être publiques**
> pour que l'updater y accède sans authentification. Avec un dépôt public,
> c'est automatiquement le cas.

### 2. Ajouter les secrets de signature

La clé privée de signature est sur ta machine :
`C:\Users\tompo\.tauri\torii-updater.key` (le mot de passe est **vide**).

Dans **GitHub → le repo → Settings → Secrets and variables → Actions → New repository secret**,
crée :

| Nom du secret | Valeur |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | **tout le contenu** du fichier `torii-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | *(laisser vide)* |

Pour copier le contenu de la clé dans le presse-papier (PowerShell) :

```powershell
Get-Content $HOME\.tauri\torii-updater.key -Raw | Set-Clipboard
```

> 🔐 **Sauvegarde `torii-updater.key` en lieu sûr.** Si tu la perds, tu ne pourras
> plus signer de mises à jour et les clients installés ne pourront plus se mettre
> à jour (il faudra les réinstaller à la main avec une nouvelle clé). Elle n'est
> **pas** dans le dépôt git (ignorée via `.gitignore`).

---

## Publier une nouvelle version

1. **Bumper la version** dans les **trois** fichiers (garde-les identiques) :
   - `package.json` → `"version"`
   - `src-tauri/tauri.conf.json` → `"version"`
   - `src-tauri/Cargo.toml` → `version` de `[package]`

2. **Ajouter une section en tête de `CHANGELOG.md`** avec le numéro exact
   (`## 0.4.0`) et la liste des changements. La CI l'extrait automatiquement pour
   en faire le corps de la Release **et** les notes affichées dans la bannière.

3. Commit + tag + push :

   ```bash
   git add -A
   git commit -m "Torii 0.4.0"
   git tag v0.4.0
   git push && git push --tags
   ```

4. GitHub Actions (`.github/workflows/release.yml`) se déclenche sur le tag :
   il build l'app Windows, la **signe**, crée la Release `Torii vX.Y.Z` et y dépose
   l'installeur `.exe` + `latest.json`. Suivre l'avancement dans l'onglet **Actions**.

5. Terminé. Les apps déjà installées proposeront la mise à jour à leur prochain lancement.

> Le numéro de version qui fait foi pour l'updater est celui de
> `src-tauri/tauri.conf.json`. Le tag `vX.Y.Z` doit correspondre.

---

## Tester en local (optionnel)

Pour produire un build signé sans passer par la CI :

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $HOME\.tauri\torii-updater.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npm run tauri build
```

Les artefacts (installeur `.exe`, `.sig`, et `latest.json` si généré) sont dans
`src-tauri/target/release/bundle/`. En CI, `tauri-action` s'occupe d'assembler
`latest.json` et de l'uploader ; en local il faut le faire à la main si tu veux
tester le flux complet.
