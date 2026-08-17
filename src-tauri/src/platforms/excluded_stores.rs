//! Liste d'exclusion de boutiques : noms de revendeurs que l'utilisateur ne veut
//! plus voir dans le comparatif de prix de la Boutique. Persistée dans
//! `excluded_stores.json` (choix perso, distinct de la catégorisation officiel/revendeur).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn file(config_dir: &Path) -> PathBuf {
    config_dir.join("excluded_stores.json")
}

/// Charge l'ensemble des noms de boutiques masquées.
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

/// Masque (`excluded=true`) ou réaffiche une boutique ; renvoie la liste à jour.
pub fn set(config_dir: &Path, name: &str, excluded: bool) -> Result<Vec<String>, String> {
    let mut set = load(config_dir);
    if excluded {
        set.insert(name.to_string());
    } else {
        set.remove(name);
    }
    save(config_dir, &set)?;
    let mut list: Vec<String> = set.into_iter().collect();
    list.sort();
    Ok(list)
}

/// Réaffiche toutes les boutiques (vide la liste d'exclusion).
pub fn clear(config_dir: &Path) -> Result<Vec<String>, String> {
    save(config_dir, &HashSet::new())?;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::{clear, load, set};

    #[test]
    fn exclude_reinclude_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ludo-excl-stores-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(load(&dir).is_empty());
        set(&dir, "Instant Gaming", true).unwrap();
        set(&dir, "Kinguin", true).unwrap();
        let after = load(&dir);
        assert!(after.contains("Instant Gaming") && after.contains("Kinguin"));

        // Réafficher : le nom disparaît, l'autre reste. Persistance relue du disque.
        set(&dir, "Instant Gaming", false).unwrap();
        let after = load(&dir);
        assert!(!after.contains("Instant Gaming") && after.contains("Kinguin"));

        // Tout réafficher : liste vidée.
        clear(&dir).unwrap();
        assert!(load(&dir).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
