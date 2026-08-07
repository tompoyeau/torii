//! Genres via IGDB (base de données de jeux cross-plateforme), à travers notre
//! mini-proxy Cloudflare qui détient le token Twitch. Permet de peupler le genre
//! de jeux absents de Steam (Fortnite, Valorant, WoW, Battle.net…), là où l'API
//! Steam Store est aveugle. Deux chemins de correspondance :
//!   - **Steam** (le gros de la biblio) : match EXACT par appid via `external_games`
//!     (`external_game_source = 1`), en masse (jusqu'à 500 jeux/requête).
//!   - **Autres launchers** : match exact du nom (`where name = "…"`), repli `search`
//!     avec sélection du nom normalisé (évite les DLC/jeux voisins que `search` remonte).
//! Résultats mis en cache disque : le 1er remplissage est lent (non-Steam throttlé
//! à <4 req/s), les suivants sont instantanés.

use crate::models::GameDto;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROXY_URL: &str = "https://torii-igdb-proxy.toriiapp.workers.dev";
const STEAM_SOURCE: u8 = 1; // external_game_source : Steam
const CALL_DELAY_MS: u64 = 300; // < 4 req/s (limite IGDB)
const STEAM_CHUNK: usize = 400; // < 500 résultats/requête
const NONSTEAM_BATCH: usize = 15; // taille des lots émis au front

/// id du jeu → genre résolu (None = cherché mais introuvable, pour ne pas re-chercher).
type GenreCache = HashMap<String, Option<String>>;

fn cache_file(dir: &Path) -> PathBuf {
    // Versionné : incrémenter si la stratégie de correspondance change.
    dir.join("igdb_genre_cache_v1.json")
}

fn load_cache(dir: &Path) -> GenreCache {
    std::fs::read_to_string(cache_file(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_cache(dir: &Path, cache: &GenreCache) {
    if std::fs::create_dir_all(dir).is_ok() {
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = std::fs::write(cache_file(dir), json);
        }
    }
}

/// POST une requête Apicalypse au proxy, renvoie le JSON.
fn query(endpoint: &str, body: &str) -> Option<Value> {
    ureq::post(&format!("{PROXY_URL}/{endpoint}"))
        .timeout(Duration::from_secs(15))
        .send_string(body)
        .ok()?
        .into_json()
        .ok()
}

/// Genre principal (1er) d'un objet jeu IGDB, nettoyé : « Role-playing (RPG) » → « RPG ».
fn primary_genre(game: &Value) -> Option<String> {
    let name = game["genres"].as_array()?.first()?["name"].as_str()?;
    Some(clean_genre(name))
}

/// Abrège les libellés IGDB entre parenthèses (« Real Time Strategy (RTS) » → « RTS »).
fn clean_genre(name: &str) -> String {
    if let (Some(o), Some(c)) = (name.find('('), name.rfind(')')) {
        if c > o + 1 {
            return name[o + 1..c].trim().to_string();
        }
    }
    name.to_string()
}

/// Normalise un nom pour comparaison : minuscules, alphanumérique seul.
fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Nettoie un titre pour l'insérer dans une chaîne Apicalypse (retire guillemets et ™®©).
fn clean_title(t: &str) -> String {
    t.chars()
        .filter(|c| !matches!(c, '"' | '™' | '®' | '©'))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Genres des jeux Steam en masse (appid → genre), via `external_games` puis `games`.
fn steam_genres(appids: &[String]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for chunk in appids.chunks(STEAM_CHUNK) {
        let uids = chunk
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            "fields game,uid; where external_game_source = {STEAM_SOURCE} & uid = ({uids}); limit 500;"
        );
        let Some(ext) = query("external_games", &body) else {
            continue;
        };
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));

        // id du jeu IGDB → appid Steam.
        let mut gid_to_appid: HashMap<i64, String> = HashMap::new();
        if let Some(arr) = ext.as_array() {
            for e in arr {
                if let (Some(g), Some(uid)) = (e["game"].as_i64(), e["uid"].as_str()) {
                    gid_to_appid.entry(g).or_insert_with(|| uid.to_string());
                }
            }
        }
        if gid_to_appid.is_empty() {
            continue;
        }

        let ids = gid_to_appid
            .keys()
            .map(|g| g.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let body2 = format!("fields id,genres.name; where id = ({ids}); limit 500;");
        let Some(games) = query("games", &body2) else {
            continue;
        };
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));

        if let Some(arr) = games.as_array() {
            for g in arr {
                if let (Some(gid), Some(genre)) = (g["id"].as_i64(), primary_genre(g)) {
                    if let Some(appid) = gid_to_appid.get(&gid) {
                        out.insert(appid.clone(), genre);
                    }
                }
            }
        }
    }
    out
}

/// Genre d'un jeu non-Steam par son titre : match exact puis repli `search`.
fn name_genre(title: &str) -> Option<String> {
    let clean = clean_title(title);
    if clean.is_empty() {
        return None;
    }
    let target = norm(&clean);

    // 1) Correspondance exacte du nom (précis quand la casse coïncide).
    let body = format!("fields name,genres.name; where name = \"{clean}\"; limit 3;");
    if let Some(Value::Array(arr)) = query("games", &body) {
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
        for g in &arr {
            if g["name"].as_str().map(norm).as_deref() == Some(target.as_str()) {
                if let Some(genre) = primary_genre(g) {
                    return Some(genre);
                }
            }
        }
    }

    // 2) Repli `search` (tolérant à la casse), on ne garde qu'un nom normalisé identique.
    let body = format!("search \"{clean}\"; fields name,genres.name; limit 15;");
    if let Some(Value::Array(arr)) = query("games", &body) {
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
        for g in &arr {
            if g["name"].as_str().map(norm).as_deref() == Some(target.as_str()) {
                return primary_genre(g);
            }
        }
    }

    None
}

/// Remplit le genre de tous les jeux (Steam en masse + autres par nom), en cache disque.
/// `emit` reçoit des lots `(id, genre)` au fil de l'eau (le front les fusionne en direct).
/// Renvoie l'ensemble des `(id, genre)` résolus.
pub fn fill_genres(
    games: &[GameDto],
    config_dir: &Path,
    emit: impl Fn(&[(String, String)]),
) -> Vec<(String, String)> {
    let mut cache = load_cache(config_dir);
    let mut out: Vec<(String, String)> = Vec::new();
    let mut dirty = false;

    // Répartition : déjà en cache / Steam (appid) / autres.
    let mut cached_batch: Vec<(String, String)> = Vec::new();
    let mut steam_todo: Vec<(String, String)> = Vec::new(); // (id jeu, appid)
    let mut other_todo: Vec<&GameDto> = Vec::new();

    for g in games {
        if let Some(cached) = cache.get(&g.id) {
            if let Some(genre) = cached {
                cached_batch.push((g.id.clone(), genre.clone()));
            }
            continue; // déjà résolu (Some ou None)
        }
        if g.platform == "steam" {
            if let Some(appid) = g.id.strip_prefix("steam:") {
                steam_todo.push((g.id.clone(), appid.to_string()));
                continue;
            }
        }
        other_todo.push(g);
    }

    // Lot immédiat des genres déjà connus (2e lancement = tout ici).
    if !cached_batch.is_empty() {
        emit(&cached_batch);
        out.extend(cached_batch);
    }

    // Steam en masse.
    if !steam_todo.is_empty() {
        let appids: Vec<String> = steam_todo.iter().map(|(_, a)| a.clone()).collect();
        let by_appid = steam_genres(&appids);
        let mut batch = Vec::new();
        for (gid, appid) in &steam_todo {
            let genre = by_appid.get(appid).cloned();
            cache.insert(gid.clone(), genre.clone());
            dirty = true;
            if let Some(genre) = genre {
                batch.push((gid.clone(), genre));
            }
        }
        if !batch.is_empty() {
            emit(&batch);
            out.extend(batch);
        }
    }

    // Non-Steam, un par un (throttlé), émis par petits lots.
    let mut batch = Vec::new();
    for g in other_todo {
        let genre = name_genre(&g.title);
        cache.insert(g.id.clone(), genre.clone());
        dirty = true;
        if let Some(genre) = genre {
            batch.push((g.id.clone(), genre));
        }
        if batch.len() >= NONSTEAM_BATCH {
            emit(&batch);
            out.extend(batch.drain(..));
        }
    }
    if !batch.is_empty() {
        emit(&batch);
        out.extend(batch);
    }

    if dirty {
        save_cache(config_dir, &cache);
    }
    out
}
