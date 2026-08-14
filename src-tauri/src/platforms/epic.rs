use crate::models::GameDto;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EpicItem {
    display_name: String,
    #[serde(default)]
    install_location: String,
    #[serde(default)]
    install_size: u64,
    app_name: String,
    #[serde(default)]
    main_game_app_name: String,
    #[serde(default)]
    catalog_namespace: String,
    #[serde(default)]
    catalog_item_id: String,
    #[serde(default, rename = "bIsIncompleteInstall")]
    incomplete: bool,
}

/// Scanne les jeux Epic à partir des manifestes JSON `*.item`.
pub fn scan() -> Vec<GameDto> {
    let Some(dir) = manifests_dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut games = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(item) = serde_json::from_str::<EpicItem>(&text) else {
            continue;
        };

        // On ne garde que les jeux de base installés (pas les DLC / composants).
        let is_base = item.main_game_app_name.is_empty() || item.main_game_app_name == item.app_name;
        if item.incomplete || !is_base || item.install_location.is_empty() {
            continue;
        }
        // Manifeste potentiellement périmé : Epic ne supprime pas toujours le `.item` quand
        // le jeu est désinstallé ou son dossier déplacé/effacé (ex. Palia laissait un manifeste
        // pointant vers un dossier disparu → jeu marqué installé à tort). On n'accepte l'entrée
        // que si le dossier d'installation existe réellement ; sinon le jeu reste visible en tant
        // que possédé/non installé via le compte Epic.
        if !std::path::Path::new(&item.install_location).is_dir() {
            continue;
        }

        // Cible de lancement = AppName Epic → deeplink `com.epicgames.launcher://`.
        // On lance TOUJOURS via le launcher (et non l'exe en direct) : certains jeux
        // exigent le SDK Epic Online Services actif (ex. UNCHARTED échoue en direct),
        // et rien dans le manifeste ne permet de les distinguer de façon fiable de ceux
        // qui tourneraient en standalone. Passer par Epic garantit un lancement propre.
        let launch_target = item.app_name.clone();

        games.push(GameDto {
            id: format!("epic:{}", item.app_name),
            title: item.display_name,
            platform: "epic".into(),
            installed: true,
            size_gb: GameDto::bytes_to_gb(item.install_size),
            install_dir: Some(item.install_location),
            launch_target,
            ..Default::default()
        });
    }
    games
}

/// Chemin de l'`EpicGamesLauncher.exe`, lu depuis le handler de protocole enregistré
/// (`HKCR\com.epicgames.launcher\shell\open\command` = `"…\EpicGamesLauncher.exe" "%1"`).
/// Sert à démarrer le launcher avec un deeplink en argument (fiable même launcher fermé).
pub fn launcher_exe() -> Option<PathBuf> {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(r"com.epicgames.launcher\shell\open\command")
        .ok()?;
    let cmd: String = key.get_value("").ok()?;
    // Premier token entre guillemets = chemin de l'exécutable.
    let exe = PathBuf::from(cmd.split('"').nth(1)?);
    exe.is_file().then_some(exe)
}

/// Identifiant **canonique complet** d'un jeu Epic pour les deeplinks autres que `launch`.
///
/// Le launcher tolère le simple `AppName` (artifactId) pour `action=launch`, mais pour
/// `action=uninstall` (comme pour `install`) il attend l'identifiant triple
/// `namespace:catalogItemId:artifactId` — sinon le deeplink est reçu (Epic s'ouvre) mais
/// aucun jeu n'est ciblé. On relit le manifeste `*.item` dont l'`AppName` correspond pour en
/// extraire le namespace + catalogItemId, et on renvoie l'identifiant avec les deux-points
/// **encodés `%3A`** (format des liens « Install » officiels d'Epic).
pub fn full_app_id(app_name: &str) -> Option<String> {
    let dir = manifests_dir()?;
    for entry in fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        let Ok(item) = fs::read_to_string(&path).and_then(|t| {
            serde_json::from_str::<EpicItem>(&t).map_err(std::io::Error::other)
        }) else {
            continue;
        };
        if item.app_name == app_name && !item.catalog_namespace.is_empty() && !item.catalog_item_id.is_empty()
        {
            return Some(format!(
                "{}%3A{}%3A{}",
                item.catalog_namespace, item.catalog_item_id, item.app_name
            ));
        }
    }
    None
}

fn manifests_dir() -> Option<PathBuf> {
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dir = PathBuf::from(program_data)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    dir.is_dir().then_some(dir)
}
