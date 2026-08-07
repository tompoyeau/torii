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

fn manifests_dir() -> Option<PathBuf> {
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dir = PathBuf::from(program_data)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    dir.is_dir().then_some(dir)
}
