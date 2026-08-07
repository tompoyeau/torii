use crate::models::GameDto;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// `RiotClientInstalls.json` : liste les jeux Riot installés (dossiers) et le
/// chemin de `RiotClientServices.exe` (le client qui lance les jeux).
#[derive(Deserialize, Default)]
struct RiotInstalls {
    #[serde(default)]
    associated_client: HashMap<String, String>,
    #[serde(default)]
    rc_live: Option<String>,
    #[serde(default)]
    rc_default: Option<String>,
}

/// Catalogue Riot (fixe et minuscule) : (marqueur dans le chemin, titre, id produit
/// pour `--launch-product`). Tous gratuits → pas de notion de « possédé ».
const KNOWN: &[(&str, &str, &str)] = &[
    ("valorant", "VALORANT", "valorant"),
    ("league of legends", "League of Legends", "league_of_legends"),
    ("legends of runeterra", "Legends of Runeterra", "bacon"),
    ("teamfight", "Teamfight Tactics", "teamfighttactics"),
];

fn installs_file() -> PathBuf {
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(program_data)
        .join("Riot Games")
        .join("RiotClientInstalls.json")
}

fn load() -> Option<RiotInstalls> {
    let text = std::fs::read_to_string(installs_file()).ok()?;
    serde_json::from_str(&text).ok()
}

/// Scanne les jeux Riot installés (via le Riot Client). Pas de jaquette ni de
/// temps de jeu : Riot n'expose rien de tel aux tiers.
pub fn scan() -> Vec<GameDto> {
    let Some(installs) = load() else {
        return Vec::new();
    };
    let mut games = Vec::new();
    for dir in installs.associated_client.keys() {
        let lower = dir.to_lowercase();
        let Some(&(_, title, product)) = KNOWN.iter().find(|(marker, _, _)| lower.contains(marker))
        else {
            continue;
        };
        games.push(GameDto {
            id: format!("riot:{product}"),
            title: title.into(),
            platform: "riot".into(),
            installed: true,
            size_gb: GameDto::bytes_to_gb(super::dir_size(Path::new(dir))),
            install_dir: Some(dir.clone()),
            launch_target: product.into(),
            ..Default::default()
        });
    }
    games
}

/// Chemin de `RiotClientServices.exe` (pour lancer un jeu Riot).
pub fn client_path() -> Option<String> {
    let installs = load()?;
    installs.rc_live.or(installs.rc_default)
}
