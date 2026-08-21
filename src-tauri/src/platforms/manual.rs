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

/// Met à jour un jeu manuel existant (édition de sa fiche) et renvoie la liste à jour.
///
/// 🔑 L'`id` ne change PAS, même si le titre change : il sert de clé aux favoris, aux jeux
/// masqués, à l'historique de session et à la fiche ouverte côté front. Le recalculer
/// depuis le nouveau titre orphelinerait tout ça d'un coup.
pub fn update(config_dir: &Path, id: &str, input: ManualInput) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    let game = games
        .iter_mut()
        .find(|g| g.id == id)
        .ok_or_else(|| format!("Jeu manuel introuvable : {id}"))?;
    game.title = input.title;
    game.launch_target = input.launch_target;
    game.install_dir = input.install_dir;
    game.cover_url = input.cover_url;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str, exe: &str) -> ManualInput {
        ManualInput {
            title: title.into(),
            launch_target: exe.into(),
            install_dir: None,
            cover_url: None,
        }
    }

    #[test]
    fn add_update_remove_roundtrip() {
        let dir = std::env::temp_dir().join(format!("torii-manual-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        add(&dir, input("Mon Jeu", r"C:\Jeux\jeu.exe")).unwrap();
        let games = scan(&dir);
        assert_eq!(games.len(), 1);
        let id = games[0].id.clone();
        assert!(games[0].installed);

        // Édition : le titre change, l'id NON (favoris/masqués/sessions y sont accrochés).
        let mut edit = input("Mon Jeu — Édition Deluxe", r"D:\Jeux\deluxe.exe");
        edit.cover_url = Some("https://exemple/cover.jpg".into());
        update(&dir, &id, edit).unwrap();
        let after = scan(&dir);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, id, "l'id d'un jeu manuel doit rester stable");
        assert_eq!(after[0].title, "Mon Jeu — Édition Deluxe");
        assert_eq!(after[0].launch_target, r"D:\Jeux\deluxe.exe");
        assert_eq!(after[0].cover_url.as_deref(), Some("https://exemple/cover.jpg"));

        // Éditer un id inconnu échoue proprement.
        assert!(update(&dir, "manual:inexistant", input("X", "x.exe")).is_err());

        remove(&dir, &id).unwrap();
        assert!(scan(&dir).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
