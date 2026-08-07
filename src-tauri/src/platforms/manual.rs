use crate::models::GameDto;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Données fournies par l'utilisateur pour ajouter un jeu à la main.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualInput {
    pub title: String,
    /// Chemin de l'exécutable à lancer.
    pub launch_target: String,
    #[serde(default)]
    pub install_dir: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
}

fn store_path(config_dir: &Path) -> PathBuf {
    config_dir.join("manual_games.json")
}

/// Lit les jeux ajoutés manuellement.
pub fn scan(config_dir: &Path) -> Vec<GameDto> {
    let path = store_path(config_dir);
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<GameDto>>(&text).unwrap_or_default()
}

/// Ajoute un jeu manuel et renvoie la liste à jour.
pub fn add(config_dir: &Path, input: ManualInput) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    let slug: String = input
        .title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    games.push(GameDto {
        id: format!("manual:{slug}"),
        title: input.title,
        platform: "manual".into(),
        installed: true,
        install_dir: input.install_dir,
        cover_url: input.cover_url,
        launch_target: input.launch_target,
        ..Default::default()
    });
    persist(config_dir, &games)?;
    Ok(games)
}

/// Retire un jeu manuel par son id et renvoie la liste à jour.
pub fn remove(config_dir: &Path, id: &str) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    games.retain(|g| g.id != id);
    persist(config_dir, &games)?;
    Ok(games)
}

fn persist(config_dir: &Path, games: &[GameDto]) -> Result<(), String> {
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(games).map_err(|e| e.to_string())?;
    fs::write(store_path(config_dir), json).map_err(|e| e.to_string())
}
