//! Métadonnées via IGDB (base de données de jeux cross-plateforme), à travers notre
//! mini-proxy Cloudflare qui détient le token Twitch. Source UNIQUE des infos
//! descriptives (genre, description, captures, jaquette de repli, hero, studio, année)
//! pour TOUS les launchers — là où Steam Store est aveugle aux jeux hors-Steam
//! (Fortnite, Valorant, WoW…). Les données propres au joueur (temps de jeu, installé,
//! possédé) restent fournies par les launchers, tout comme leur jaquette native
//! (IGDB ne sert que de repli pour la jaquette, décision utilisateur).
//!
//! Deux chemins de correspondance (validés en test réel, 92 % de couverture) :
//!   - **Steam** : match EXACT par appid via `external_games` (`external_game_source = 1`),
//!     en masse (jusqu'à 500 jeux/requête).
//!   - **Autres launchers** : match exact du nom (`where name = "…"`), repli `search`
//!     avec sélection du nom normalisé (évite les DLC/jeux voisins remontés par `search`).

use crate::models::GameDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROXY_URL: &str = "https://torii-igdb-proxy.toriiapp.workers.dev";
const IMG: &str = "https://images.igdb.com/igdb/image/upload";
const STEAM_SOURCE: u8 = 1; // external_game_source : Steam
const CALL_DELAY_MS: u64 = 300; // < 4 req/s (limite IGDB)
const STEAM_CHUNK: usize = 400; // < 500 résultats/requête
const NONSTEAM_BATCH: usize = 12; // taille des lots émis au front

/// Champs IGDB récupérés pour chaque jeu (partagés entre les deux chemins).
const FIELDS: &str = "fields id, name, genres.name, summary, cover.image_id, \
artworks.image_id, screenshots.image_id, involved_companies.company.name, \
involved_companies.developer, first_release_date;";

/// Métadonnées descriptives d'un jeu, issues d'IGDB.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgdbMeta {
    pub genre: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    pub developer: Option<String>,
    pub year: Option<i64>,
    #[serde(default)]
    pub screenshots: Vec<String>,
}

/// id du jeu → métadonnées (None = cherché mais introuvable dans IGDB, pour ne pas re-chercher).
type MetaCache = HashMap<String, Option<IgdbMeta>>;

fn cache_file(dir: &Path) -> PathBuf {
    // Versionné : incrémenter si le schéma `IgdbMeta` ou la stratégie de correspondance change.
    dir.join("igdb_meta_cache_v1.json")
}

fn load_cache(dir: &Path) -> MetaCache {
    std::fs::read_to_string(cache_file(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_cache(dir: &Path, cache: &MetaCache) {
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

/// URL d'une image IGDB à une taille donnée (ex. « cover_big_2x », « 1080p »).
fn img(image_id: &str, size: &str) -> String {
    format!("{IMG}/t_{size}/{image_id}.jpg")
}

/// Abrège les libellés de genre entre parenthèses (« Real Time Strategy (RTS) » → « RTS »).
fn clean_genre(name: &str) -> String {
    if let (Some(o), Some(c)) = (name.find('('), name.rfind(')')) {
        if c > o + 1 {
            return name[o + 1..c].trim().to_string();
        }
    }
    name.to_string()
}

/// Année à partir d'un timestamp Unix (approximation suffisante pour un affichage).
fn unix_to_year(ts: i64) -> i64 {
    1970 + ts / 31_556_952 // secondes dans une année moyenne
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

/// Construit un `IgdbMeta` à partir d'un objet jeu IGDB.
fn parse_meta(g: &Value) -> IgdbMeta {
    let genre = g["genres"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|x| x["name"].as_str())
        .map(clean_genre);

    let description = g["summary"].as_str().map(String::from);

    let cover_url = g["cover"]["image_id"]
        .as_str()
        .map(|id| img(id, "cover_big_2x"));

    // Hero paysage : 1re artwork, à défaut 1re capture.
    let hero_url = g["artworks"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|x| x["image_id"].as_str())
        .or_else(|| {
            g["screenshots"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["image_id"].as_str())
        })
        .map(|id| img(id, "1080p"));

    let developer = g["involved_companies"]
        .as_array()
        .and_then(|a| a.iter().find(|c| c["developer"].as_bool().unwrap_or(false)))
        .and_then(|c| c["company"]["name"].as_str())
        .map(String::from);

    let year = g["first_release_date"].as_i64().map(unix_to_year);

    let screenshots = g["screenshots"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s["image_id"].as_str())
                .take(6)
                .map(|id| img(id, "1080p"))
                .collect()
        })
        .unwrap_or_default();

    IgdbMeta {
        genre,
        description,
        cover_url,
        hero_url,
        developer,
        year,
        screenshots,
    }
}

/// Métadonnées des jeux Steam en masse (appid → meta), via `external_games` puis `games`.
fn steam_metas(appids: &[String]) -> HashMap<String, IgdbMeta> {
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
        let body2 = format!("{FIELDS} where id = ({ids}); limit 500;");
        let Some(games) = query("games", &body2) else {
            continue;
        };
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));

        if let Some(arr) = games.as_array() {
            for g in arr {
                if let Some(gid) = g["id"].as_i64() {
                    if let Some(appid) = gid_to_appid.get(&gid) {
                        out.insert(appid.clone(), parse_meta(g));
                    }
                }
            }
        }
    }
    out
}

/// Métadonnées d'un jeu non-Steam par son titre : match exact puis repli `search`.
fn name_meta(title: &str) -> Option<IgdbMeta> {
    let clean = clean_title(title);
    if clean.is_empty() {
        return None;
    }
    let target = norm(&clean);

    // 1) Correspondance exacte du nom (précis quand la casse coïncide).
    let body = format!("{FIELDS} where name = \"{clean}\"; limit 3;");
    if let Some(Value::Array(arr)) = query("games", &body) {
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
        for g in &arr {
            if g["name"].as_str().map(norm).as_deref() == Some(target.as_str()) {
                return Some(parse_meta(g));
            }
        }
    }

    // 2) Repli `search` (tolérant à la casse), on ne garde qu'un nom normalisé identique.
    let body = format!("search \"{clean}\"; {FIELDS} limit 15;");
    if let Some(Value::Array(arr)) = query("games", &body) {
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
        for g in &arr {
            if g["name"].as_str().map(norm).as_deref() == Some(target.as_str()) {
                return Some(parse_meta(g));
            }
        }
    }

    None
}

/// Remplit la métadonnée descriptive de tous les jeux (Steam en masse + autres par nom),
/// en cache disque. `emit` reçoit des lots `(id, meta)` au fil de l'eau (fusion en direct
/// côté front). Renvoie l'ensemble des `(id, meta)` résolus.
pub fn fill_metadata(
    games: &[GameDto],
    config_dir: &Path,
    emit: impl Fn(&[(String, IgdbMeta)]),
) -> Vec<(String, IgdbMeta)> {
    let mut cache = load_cache(config_dir);
    let mut out: Vec<(String, IgdbMeta)> = Vec::new();
    let mut dirty = false;

    // Répartition : déjà en cache / Steam (appid) / autres.
    let mut cached_batch: Vec<(String, IgdbMeta)> = Vec::new();
    let mut steam_todo: Vec<(String, String)> = Vec::new(); // (id jeu, appid)
    let mut other_todo: Vec<&GameDto> = Vec::new();

    for g in games {
        if let Some(cached) = cache.get(&g.id) {
            if let Some(meta) = cached {
                cached_batch.push((g.id.clone(), meta.clone()));
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

    // Lot immédiat des métas déjà connues (2e lancement = tout ici).
    if !cached_batch.is_empty() {
        emit(&cached_batch);
        out.extend(cached_batch);
    }

    // Steam en masse.
    if !steam_todo.is_empty() {
        let appids: Vec<String> = steam_todo.iter().map(|(_, a)| a.clone()).collect();
        let by_appid = steam_metas(&appids);
        let mut batch = Vec::new();
        for (gid, appid) in &steam_todo {
            let meta = by_appid.get(appid).cloned();
            cache.insert(gid.clone(), meta.clone());
            dirty = true;
            if let Some(meta) = meta {
                batch.push((gid.clone(), meta));
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
        let meta = name_meta(&g.title);
        cache.insert(g.id.clone(), meta.clone());
        dirty = true;
        if let Some(meta) = meta {
            batch.push((g.id.clone(), meta));
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
