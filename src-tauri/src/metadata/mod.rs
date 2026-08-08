pub mod gog_store;
pub mod igdb;
pub mod instant_gaming;
pub mod steam_store;
pub mod store;

use crate::models::{GameDto, GameMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

// --- Jaquettes universelles (repli sans clé, tous launchers) --------------------------

/// Cache des jaquettes résolues par titre : titre normalisé → jaquette (None = introuvable
/// sur Steam, pour ne pas re-chercher). Partagé entre launchers (même titre = même jaquette).
type CoverCache = std::collections::HashMap<String, Option<String>>;

fn cover_cache_file(config_dir: &Path) -> PathBuf {
    // v2 : recherche améliorée (repli sans numéro final) → on ignore les échecs mémorisés.
    config_dir.join("cover_cache_v2.json")
}

fn cover_key(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Remplit les jaquettes MANQUANTES de n'importe quel jeu (Battle.net, EA, Epic…) via
/// une recherche Steam Store par titre (publique, sans clé) → `library_600x900`. Une seule
/// recherche par jeu, résultat mis en cache disque. Renvoie les `(id, jaquette)` résolus.
pub fn fill_covers(games: &[GameDto], config_dir: &Path) -> Vec<(String, String)> {
    let mut cache = load_cover_cache(config_dir);
    let mut out = Vec::new();
    let mut dirty = false;

    for game in games {
        if game.cover_url.is_some() {
            continue; // le launcher a déjà fourni une jaquette
        }
        let key = cover_key(&game.title);
        if key.is_empty() {
            continue;
        }
        let cover = match cache.get(&key) {
            Some(cached) => cached.clone(),
            None => {
                let resolved = steam_store::search_appid(&game.title)
                    .map(|appid| steam_store::library_cover(&appid));
                cache.insert(key, resolved.clone());
                dirty = true;
                std::thread::sleep(Duration::from_millis(200)); // politesse API
                resolved
            }
        };
        if let Some(url) = cover {
            out.push((game.id.clone(), url));
        }
    }

    if dirty {
        save_cover_cache(config_dir, &cache);
    }
    out
}

fn load_cover_cache(config_dir: &Path) -> CoverCache {
    std::fs::read_to_string(cover_cache_file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_cover_cache(config_dir: &Path, cache: &CoverCache) {
    if std::fs::create_dir_all(config_dir).is_ok() {
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = std::fs::write(cover_cache_file(config_dir), json);
        }
    }
}

/// Enrichit une liste de jeux avec des métadonnées en ligne.
/// Utilise un cache disque pour ne récupérer chaque jeu qu'une seule fois.
/// `progress(done, total)` est appelé après chaque jeu (pour l'UI).
pub fn enrich(games: &mut [GameDto], config_dir: &Path, progress: impl Fn(usize, usize)) {
    let mut cache = load_cache(config_dir);
    let mut dirty = false;
    let total = games.len();

    for (i, game) in games.iter_mut().enumerate() {
        let meta = match cache.get(&game.id) {
            Some(m) => m.clone(),
            None => {
                let fetched = fetch(game).unwrap_or_default();
                cache.insert(game.id.clone(), fetched.clone());
                dirty = true;
                // Politesse envers l'API publique.
                std::thread::sleep(Duration::from_millis(250));
                fetched
            }
        };
        apply(game, meta);
        progress(i + 1, total);
    }

    if dirty {
        save_cache(config_dir, &cache);
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

/// Applique les métadonnées sans écraser ce que le scan a déjà fourni.
fn apply(game: &mut GameDto, meta: GameMeta) {
    // Résout les titres provisoires « App <id> » des jeux possédés non installés.
    if let Some(name) = meta.name {
        if game.title.is_empty() || game.title.starts_with("App ") {
            game.title = name;
        }
    }
    if game.genre.is_none() {
        game.genre = meta.genre;
    }
    if game.description.is_none() {
        game.description = meta.description;
    }
    if game.developer.is_none() {
        game.developer = meta.developer;
    }
    if game.year.is_none() {
        game.year = meta.year;
    }
    if game.cover_url.is_none() {
        game.cover_url = meta.cover_url;
    }
    if game.hero_url.is_none() {
        game.hero_url = meta.hero_url;
    }
    if game.screenshots.is_empty() {
        game.screenshots = meta.screenshots;
    }
    if game.app_type.is_none() {
        game.app_type = meta.app_type;
    }
    // Taille de téléchargement (jeu non installé) : ne pas écraser une taille disque réelle.
    if game.size_gb == 0.0 {
        if let Some(size) = meta.size_gb {
            game.size_gb = size;
        }
    }
}
