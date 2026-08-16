//! Wishlist « Torii » universelle : n'importe quel jeu (Steam ou non) que l'utilisateur
//! veut suivre. Persistée dans `wishlist_torii.json`. Pour les jeux Steam, on pousse
//! **en bonus** vers la vraie wishlist Steam (best-effort, côté `accounts::steam`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Une entrée de la wishlist Torii. `id` = identifiant ITAD (uuid) du jeu.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WishEntry {
    pub id: String,
    /// Appid Steam si le jeu existe sur Steam (permet le push vers la wishlist Steam).
    #[serde(default)]
    pub steam_appid: Option<u64>,
    pub title: String,
    #[serde(default)]
    pub cover_url: Option<String>,
}

fn file(config_dir: &Path) -> PathBuf {
    config_dir.join("wishlist_torii.json")
}

/// Charge la wishlist Torii.
pub fn load(config_dir: &Path) -> Vec<WishEntry> {
    std::fs::read_to_string(file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(config_dir: &Path, list: &[WishEntry]) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(file(config_dir), json).map_err(|e| e.to_string())
}

/// Ajoute une entrée (si absente) ; renvoie la liste à jour.
pub fn add(config_dir: &Path, entry: WishEntry) -> Result<Vec<WishEntry>, String> {
    let mut list = load(config_dir);
    if !list.iter().any(|e| e.id == entry.id) {
        list.push(entry);
    }
    save(config_dir, &list)?;
    Ok(list)
}

/// Retire une entrée par son id ; renvoie la liste à jour.
pub fn remove(config_dir: &Path, id: &str) -> Result<Vec<WishEntry>, String> {
    let mut list = load(config_dir);
    list.retain(|e| e.id != id);
    save(config_dir, &list)?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_remove_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ludo-wl-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load(&dir).is_empty());
        add(&dir, WishEntry { id: "itad-1".into(), steam_appid: Some(440), title: "TF2".into(), cover_url: None }).unwrap();
        add(&dir, WishEntry { id: "itad-2".into(), steam_appid: None, title: "Indie".into(), cover_url: None }).unwrap();
        // Pas de doublon.
        add(&dir, WishEntry { id: "itad-1".into(), steam_appid: Some(440), title: "TF2".into(), cover_url: None }).unwrap();
        assert_eq!(load(&dir).len(), 2);
        remove(&dir, "itad-1").unwrap();
        let after = load(&dir);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, "itad-2");
    }
}
