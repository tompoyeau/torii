//! Historique de lancement « maison » : Torii enregistre l'instant où l'utilisateur
//! clique sur **Jouer**. Ça fournit une date de « dernière session » pour les jeux dont
//! le launcher ne l'expose pas (Riot, EA, Battle.net, Ubisoft, manuel…). Limite assumée :
//! si le jeu est lancé HORS de Torii, on ne capte rien — mais c'est mieux que rien, et
//! pour les jeux qui ont déjà une vraie date (Steam/GOG/Epic) on garde la plus récente.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// id du jeu → horodatage Unix (secondes) du dernier lancement via Torii.
type History = HashMap<String, i64>;

fn store_path(config_dir: &Path) -> PathBuf {
    config_dir.join("last_played.json")
}

/// Charge l'historique des lancements (vide si absent).
pub fn load(config_dir: &Path) -> History {
    std::fs::read_to_string(store_path(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Enregistre « maintenant » comme dernier lancement du jeu `id`. Renvoie l'horodatage posé.
pub fn record(config_dir: &Path, id: &str) -> Result<i64, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    record_at(config_dir, id, now)
}

/// Enregistre une date de session précise (heure de démarrage réelle du process).
/// 🔑 Ne recule JAMAIS la date connue : une partie détectée après coup (Torii ouvert
/// alors que le jeu tournait déjà) ne doit pas écraser un lancement plus récent.
pub fn record_at(config_dir: &Path, id: &str, at: i64) -> Result<i64, String> {
    let mut history = load(config_dir);
    let now = history.get(id).copied().unwrap_or(0).max(at);
    history.insert(id.to_string(), now);
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&history).map_err(|e| e.to_string())?;
    std::fs::write(store_path(config_dir), json).map_err(|e| e.to_string())?;
    Ok(now)
}
