use super::{vdf_all, vdf_first};
use crate::models::GameDto;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Scanne les jeux Steam à partir des manifestes `appmanifest_*.acf`.
pub fn scan() -> Vec<GameDto> {
    let Some(steam) = steam_path() else {
        return Vec::new();
    };
    let mut games = Vec::new();
    // Un même jeu peut apparaître dans plusieurs bibliothèques (ou le dossier
    // principal listé deux fois avec une casse différente) : on dédoublonne par id.
    let mut seen = HashSet::new();
    for lib in library_paths(&steam) {
        let apps_dir = lib.join("steamapps");
        let Ok(entries) = fs::read_dir(&apps_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("appmanifest_") && name.ends_with(".acf") {
                if let Some(game) = parse_manifest(&path, &apps_dir) {
                    if seen.insert(game.id.clone()) {
                        games.push(game);
                    }
                }
            }
        }
    }
    games
}

/// Localise Steam via le registre (chemin utilisateur puis installation machine).
pub(crate) fn steam_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
            if let Ok(p) = key.get_value::<String, _>("SteamPath") {
                return Some(PathBuf::from(p.replace('/', "\\")));
            }
        }
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for sub in [r"SOFTWARE\WOW6432Node\Valve\Steam", r"SOFTWARE\Valve\Steam"] {
            if let Ok(key) = hklm.open_subkey(sub) {
                if let Ok(p) = key.get_value::<String, _>("InstallPath") {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    None
}

/// Toutes les bibliothèques Steam (dossier principal + libraryfolders.vdf).
fn library_paths(steam: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam.to_path_buf()];
    let vdf = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = fs::read_to_string(&vdf) {
        for raw in vdf_all(&text, "path") {
            let pb = PathBuf::from(raw.replace("\\\\", "\\"));
            if !libs.contains(&pb) {
                libs.push(pb);
            }
        }
    }
    libs
}

/// Filtre les entrées qui ne sont pas des jeux (redistribuables, runtimes…).
fn is_tool(name: &str) -> bool {
    const NOISE: [&str; 5] = [
        "Redistributable",
        "Steamworks Common",
        "Proton",
        "Steam Linux Runtime",
        "Steamworks Shared",
    ];
    NOISE.iter().any(|n| name.contains(n))
}

fn parse_manifest(file: &Path, apps_dir: &Path) -> Option<GameDto> {
    let text = fs::read_to_string(file).ok()?;
    let appid = vdf_first(&text, "appid")?;
    let name = vdf_first(&text, "name")?;
    if is_tool(&name) {
        return None;
    }
    let installdir = vdf_first(&text, "installdir");
    let size_bytes = vdf_first(&text, "SizeOnDisk")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let last_played = vdf_first(&text, "LastPlayed")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|&t| t > 0);
    let install_dir = installdir
        .map(|d| apps_dir.join("common").join(d).to_string_lossy().into_owned());

    Some(GameDto {
        id: format!("steam:{appid}"),
        title: name,
        platform: "steam".into(),
        installed: true,
        size_gb: GameDto::bytes_to_gb(size_bytes),
        install_dir,
        cover_url: Some(format!(
            "https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg"
        )),
        hero_url: Some(format!(
            "https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_hero.jpg"
        )),
        launch_target: appid,
        last_played,
        ..Default::default()
    })
}
