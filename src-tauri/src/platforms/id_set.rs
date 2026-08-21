//! Petit magasin d'**ensemble d'identifiants** persisté en JSON (une liste triée).
//!
//! Trois listes de l'app partagent exactement cette mécanique — jeux masqués, jeux
//! favoris, revendeurs exclus du comparatif de prix. Elles vivaient dans trois fichiers
//! identiques au nom du fichier de données près ; un seul code paramétré les remplace.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Une liste d'identifiants persistée dans `<config_dir>/<fichier>`.
#[derive(Clone, Copy)]
pub struct IdSet(&'static str);

/// Jeux masqués par l'utilisateur (jeux non désirés, doublons cross-plateforme).
pub const HIDDEN: IdSet = IdSet("hidden.json");
/// Jeux épinglés par l'utilisateur (filtre « Favoris »).
pub const FAVORITES: IdSet = IdSet("favorites.json");
/// Jeux à ne JAMAIS diffuser aux amis. Indispensable : une application permanente
/// (Wallpaper Engine et consorts) annoncerait sinon une partie 24 h sur 24, et
/// certains jeux ne regardent personne.
pub const PRESENCE_MUTED: IdSet = IdSet("presence_muted.json");

/// Revendeurs masqués du comparatif de prix (choix perso, distinct de la
/// catégorisation officiel/revendeur qui, elle, est factuelle).
pub const EXCLUDED_STORES: IdSet = IdSet("excluded_stores.json");

impl IdSet {
    fn file(self, config_dir: &Path) -> PathBuf {
        config_dir.join(self.0)
    }

    /// Charge l'ensemble des identifiants (vide si le fichier est absent ou illisible).
    pub fn load(self, config_dir: &Path) -> HashSet<String> {
        std::fs::read_to_string(self.file(config_dir))
            .ok()
            .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
            .map(|v| v.into_iter().collect())
            .unwrap_or_default()
    }

    fn save(self, config_dir: &Path, set: &HashSet<String>) -> Result<(), String> {
        std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&sorted(set)).map_err(|e| e.to_string())?;
        std::fs::write(self.file(config_dir), json).map_err(|e| e.to_string())
    }

    /// Ajoute (`on = true`) ou retire un identifiant ; renvoie la liste triée à jour.
    pub fn set(self, config_dir: &Path, id: &str, on: bool) -> Result<Vec<String>, String> {
        let mut set = self.load(config_dir);
        if on {
            set.insert(id.to_string());
        } else {
            set.remove(id);
        }
        self.save(config_dir, &set)?;
        Ok(sorted(&set))
    }

    /// Vide la liste ; renvoie la liste à jour (donc vide).
    pub fn clear(self, config_dir: &Path) -> Result<Vec<String>, String> {
        self.save(config_dir, &HashSet::new())?;
        Ok(Vec::new())
    }
}

/// Liste triée (ordre stable sur disque comme dans la réponse au front).
fn sorted(set: &HashSet<String>) -> Vec<String> {
    let mut list: Vec<String> = set.iter().cloned().collect();
    list.sort();
    list
}

#[cfg(test)]
mod tests {
    use super::{EXCLUDED_STORES, HIDDEN};

    #[test]
    fn set_unset_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ludo-idset-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(HIDDEN.load(&dir).is_empty());
        HIDDEN.set(&dir, "epic:Dup", true).unwrap();
        let list = HIDDEN.set(&dir, "steam:440", true).unwrap();
        assert_eq!(list, vec!["epic:Dup".to_string(), "steam:440".to_string()]);
        let after = HIDDEN.load(&dir);
        assert!(after.contains("epic:Dup") && after.contains("steam:440"));

        // Retirer : l'id disparaît, l'autre reste. Persistance relue du disque.
        HIDDEN.set(&dir, "epic:Dup", false).unwrap();
        let after = HIDDEN.load(&dir);
        assert!(!after.contains("epic:Dup") && after.contains("steam:440"));

        // Chaque liste a son propre fichier : elles ne se marchent pas dessus.
        assert!(EXCLUDED_STORES.load(&dir).is_empty());
        EXCLUDED_STORES.set(&dir, "Kinguin", true).unwrap();
        assert!(HIDDEN.load(&dir).contains("steam:440"));

        EXCLUDED_STORES.clear(&dir).unwrap();
        assert!(EXCLUDED_STORES.load(&dir).is_empty());
        assert!(!HIDDEN.load(&dir).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
