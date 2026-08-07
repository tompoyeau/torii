//! Jeux favoris : ids des jeux épinglés par l'utilisateur (filtre « Favoris »).
//! Persistés dans `favorites.json`. Même mécanique que la liste d'exclusion (`hidden`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn file(config_dir: &Path) -> PathBuf {
    config_dir.join("favorites.json")
}

/// Charge l'ensemble des ids favoris.
pub fn load(config_dir: &Path) -> HashSet<String> {
    std::fs::read_to_string(file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

fn save(config_dir: &Path, set: &HashSet<String>) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let mut list: Vec<&String> = set.iter().collect();
    list.sort();
    let json = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    std::fs::write(file(config_dir), json).map_err(|e| e.to_string())
}

/// Épingle (`favorite=true`) ou retire un jeu des favoris ; renvoie la liste à jour.
pub fn set(config_dir: &Path, id: &str, favorite: bool) -> Result<Vec<String>, String> {
    let mut set = load(config_dir);
    if favorite {
        set.insert(id.to_string());
    } else {
        set.remove(id);
    }
    save(config_dir, &set)?;
    let mut list: Vec<String> = set.into_iter().collect();
    list.sort();
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::{load, set};

    #[test]
    fn favorite_unfavorite_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ludo-fav-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(load(&dir).is_empty());
        set(&dir, "steam:440", true).unwrap();
        set(&dir, "gog:123", true).unwrap();
        let after = load(&dir);
        assert!(after.contains("steam:440") && after.contains("gog:123"));

        // Retirer un favori : l'id disparaît, l'autre reste. Persistance relue du disque.
        set(&dir, "steam:440", false).unwrap();
        let after = load(&dir);
        assert!(!after.contains("steam:440") && after.contains("gog:123"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
