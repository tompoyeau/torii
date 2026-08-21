pub mod gog_store;
pub mod igdb;
pub mod instant_gaming;
pub mod steam_store;
pub mod store;

use crate::models::{GameDto, GameMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Cache = HashMap<String, GameMeta>;

fn cache_file(config_dir: &Path) -> PathBuf {
    // Suffixe versionné : à incrémenter quand le schéma `GameMeta` évolue OU la recherche
    // s'améliore, pour ignorer les anciennes entrées. v2 = ajout `size_gb` ; v3 = recherche
    // avec repli sans numéro final (résout OW2 & co).
    config_dir.join("metadata_cache_v3.json")
}

fn load_cache(config_dir: &Path) -> Cache {
    std::fs::read_to_string(cache_file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_cache(config_dir: &Path, cache: &Cache) {
    if std::fs::create_dir_all(config_dir).is_ok() {
        if let Ok(json) = serde_json::to_string_pretty(cache) {
            let _ = std::fs::write(cache_file(config_dir), json);
        }
    }
}

/// Enrichit un **seul** jeu à la demande (ouverture de la vue détail), avec le
/// même cache disque que l'enrichissement en masse. Retour vide si rien trouvé.
pub fn enrich_one(game: &GameDto, config_dir: &Path) -> GameMeta {
    let mut cache = load_cache(config_dir);
    if let Some(meta) = cache.get(&game.id) {
        return meta.clone();
    }
    let fetched = fetch(game).unwrap_or_default();
    cache.insert(game.id.clone(), fetched.clone());
    save_cache(config_dir, &cache);
    fetched
}

/// Récupère les métadonnées d'un jeu selon sa plateforme.
fn fetch(game: &GameDto) -> Option<GameMeta> {
    match game.platform.as_str() {
        "steam" => {
            let mut meta = steam_store::appdetails(&game.launch_target)?;
            // Taille d'installation via un 2e appel (api.steamcmd.net), seulement
            // utile pour un jeu non installé (l'installé a déjà sa taille disque).
            if !game.installed {
                meta.size_gb = steam_store::install_size_gb(&game.launch_target);
            }
            Some(meta)
        }
        // L'id produit GOG est dans `id` (« gog:<id> ») : `launch_target` est le
        // chemin de l'exe pour un jeu installé, inutilisable pour l'API.
        "gog" => gog_store::product(game.id.strip_prefix("gog:").unwrap_or(&game.launch_target)),
        // Epic / manuel : on tente une correspondance par titre sur Steam.
        _ => {
            let appid = steam_store::search_appid(&game.title)?;
            steam_store::appdetails(&appid)
        }
    }
}
