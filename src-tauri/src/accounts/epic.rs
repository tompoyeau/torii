use crate::models::GameDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Identifiants publics du client Epic Games Launcher (les mêmes que Legendary /
/// Heroic). Le Basic auth « id:secret » est constant → précalculé en base64.
const BASIC_AUTH: &str =
    "MzRhMDJjZjhmNDQxNGUyOWIxNTkyMTg3NmRhMzZmOWE6ZGFhZmJjY2M3Mzc3NDUwMzlkZmZlNTNkOTRmYzc2Y2Y=";
const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const UA: &str = "UELauncher/11.0.1-14907503+++Portal+Release-Live Windows/10.0.19041.1.256.64bit";

const TOKEN_URL: &str =
    "https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token";
const ASSETS_URL: &str =
    "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows?label=Live";

/// URL de la page de login à charger dans la fenêtre : après connexion, Epic
/// redirige vers `/id/api/redirect` qui renvoie un JSON contenant le `authorizationCode`.
pub fn login_url() -> String {
    let redirect = format!(
        "https://www.epicgames.com/id/api/redirect?clientId={CLIENT_ID}&responseType=code"
    );
    format!(
        "https://www.epicgames.com/id/login?redirectUrl={}",
        urlencode(&redirect)
    )
}

/// Réponse OAuth d'Epic.
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

/// Échange le code d'autorisation (capté dans la fenêtre de login) contre des jetons.
pub fn exchange_code(code: &str) -> Option<Tokens> {
    token_request(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("token_type", "eg1"),
    ])
}

fn refresh(refresh_token: &str) -> Option<Tokens> {
    token_request(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("token_type", "eg1"),
    ])
}

fn token_request(form: &[(&str, &str)]) -> Option<Tokens> {
    let json: Value = ureq::post(TOKEN_URL)
        .timeout(Duration::from_secs(20))
        .set("Authorization", &format!("Basic {BASIC_AUTH}"))
        .set("User-Agent", UA)
        .send_form(form)
        .ok()?
        .into_json()
        .ok()?;
    Some(Tokens {
        access_token: json["access_token"].as_str()?.to_string(),
        refresh_token: json["refresh_token"].as_str()?.to_string(),
        account_id: json["account_id"].as_str().unwrap_or_default().to_string(),
    })
}

/// Nombre de résolutions catalogue menées en parallèle (le 1er scan résout des
/// centaines d'items ; en séquentiel il bloquerait l'app plusieurs minutes).
const RESOLVE_WORKERS: usize = 16;

/// Bibliothèque Epic possédée. Rafraîchit le jeton, liste les assets possédés,
/// puis résout titres/jaquettes via le catalogue — **en parallèle** et caché sur
/// disque (une fois par jeu). Le 1er scan prend quelques secondes, ensuite instantané.
pub fn owned_games(config_dir: &Path, refresh_token: &str) -> Vec<GameDto> {
    let Some(tokens) = refresh(refresh_token) else {
        return Vec::new();
    };
    persist_refresh(config_dir, &tokens.refresh_token);

    // Assets possédés (hors Unreal Engine).
    let assets: Vec<Asset> = fetch_assets(&tokens.access_token)
        .into_iter()
        .filter(|a| a.namespace != "ue")
        .collect();

    let mut cache = load_cache(config_dir);
    // Items pas encore en cache → à résoudre (en parallèle).
    let todo: Vec<&Asset> = assets
        .iter()
        .filter(|a| !cache.contains_key(&a.catalog_item_id))
        .collect();
    if !todo.is_empty() {
        for (id, meta) in resolve_all(&todo, &tokens.access_token) {
            cache.insert(id, meta);
        }
        save_cache(config_dir, &cache);
    }

    // Temps de jeu par appName (un seul appel bulk).
    let playtime = fetch_playtime(&tokens.access_token, &tokens.account_id);

    // Construit la liste finale à partir du cache (jeux de base uniquement).
    // Dédup par catalogItemId : un asset marketplace UE/Fab a un exemplaire par
    // version de moteur (mêmes catalogItemId/titre) → sinon x10 doublons.
    let mut games = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for asset in &assets {
        let Some(meta) = cache.get(&asset.catalog_item_id) else {
            continue; // résolution échouée (réseau) → réessai au prochain scan
        };
        if !meta.is_game {
            continue;
        }
        if !seen.insert(asset.catalog_item_id.clone()) {
            continue; // déjà ajouté (autre version de moteur du même item)
        }
        games.push(GameDto {
            id: format!("epic:{}", asset.app_name),
            title: meta.title.clone().unwrap_or_else(|| asset.app_name.clone()),
            platform: "epic".into(),
            installed: false,
            owned: true,
            playtime_minutes: playtime.get(&asset.app_name).copied().filter(|&m| m > 0),
            cover_url: meta.cover.clone(),
            hero_url: meta.hero.clone(),
            launch_target: asset.app_name.clone(),
            app_type: Some("game".into()),
            ..Default::default()
        });
    }
    games
}

/// Temps de jeu Epic par appName (`library-service/.../playtime/account/{id}/all`).
/// Renvoie une table appName → minutes jouées. Un seul appel bulk.
fn fetch_playtime(access_token: &str, account_id: &str) -> HashMap<String, u32> {
    if account_id.is_empty() {
        return HashMap::new();
    }
    let url = format!(
        "https://library-service.live.use1a.on.epicgames.com/library/api/public/playtime/account/{account_id}/all"
    );
    get_json_auth(&url, access_token)
        .and_then(|json| json.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let artifact = e["artifactId"].as_str()?;
                    // `totalTime` est en secondes → minutes.
                    let minutes = (e["totalTime"].as_u64().unwrap_or(0) / 60) as u32;
                    Some((artifact.to_string(), minutes))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Résout un lot d'items via le catalogue, réparti sur plusieurs threads.
/// Les échecs (réseau) ne sont pas renvoyés → non cachés → réessayés plus tard.
fn resolve_all(todo: &[&Asset], access_token: &str) -> Vec<(String, EpicMeta)> {
    let chunk = todo.len().div_ceil(RESOLVE_WORKERS).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = todo
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    slice
                        .iter()
                        .filter_map(|a| {
                            resolve_catalog(access_token, &a.namespace, &a.catalog_item_id)
                                .map(|m| (a.catalog_item_id.clone(), m))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

struct Asset {
    app_name: String,
    catalog_item_id: String,
    namespace: String,
}

fn fetch_assets(access_token: &str) -> Vec<Asset> {
    get_json_auth(ASSETS_URL, access_token)
        .and_then(|json| json.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(Asset {
                        app_name: a["appName"].as_str()?.to_string(),
                        catalog_item_id: a["catalogItemId"].as_str()?.to_string(),
                        namespace: a["namespace"].as_str().unwrap_or_default().to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Métadonnées Epic résolues et mises en cache (jeu ou non, titre, jaquettes).
#[derive(Serialize, Deserialize, Clone, Default)]
struct EpicMeta {
    is_game: bool,
    title: Option<String>,
    cover: Option<String>,
    hero: Option<String>,
}

/// Résout un item du catalogue Epic : détermine s'il s'agit d'un jeu de base
/// (pas un DLC, un mod ou un asset UE) et en extrait titre + jaquettes.
fn resolve_catalog(access_token: &str, namespace: &str, catalog_id: &str) -> Option<EpicMeta> {
    let url = format!(
        "https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace/\
         {namespace}/bulk/items?id={catalog_id}&includeMainGameDetails=true&country=US&locale=en-US"
    );
    let root = get_json_auth(&url, access_token)?;
    let item = root.get(catalog_id)?;

    let categories: Vec<&str> = item["categories"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|c| c["path"].as_str()).collect())
        .unwrap_or_default();

    // Un vrai jeu a la catégorie `games`. Les assets UE/Fab Marketplace (plugins,
    // contenu…) ne l'ont pas (`plugins`, `asset-format`…) → exclus. On écarte aussi
    // les DLC (rattachés à un jeu principal via `mainGameItem`) et les mods.
    let is_game = categories.contains(&"games")
        && item.get("mainGameItem").is_none()
        && !categories.contains(&"mods");

    Some(EpicMeta {
        is_game,
        title: item["title"].as_str().map(str::trim).map(String::from),
        cover: key_image(item, &["DieselGameBoxTall", "OfferImageTall", "Thumbnail"]),
        hero: key_image(item, &["DieselGameBox", "DieselGameBoxWide", "OfferImageWide"]),
    })
}

/// Première `keyImages` dont le `type` figure dans `wanted`, par ordre de préférence.
fn key_image(item: &Value, wanted: &[&str]) -> Option<String> {
    let images = item["keyImages"].as_array()?;
    for want in wanted {
        for img in images {
            if img["type"].as_str() == Some(want) {
                if let Some(url) = img["url"].as_str() {
                    return Some(url.to_string());
                }
            }
        }
    }
    None
}

fn get_json_auth(url: &str, token: &str) -> Option<Value> {
    ureq::get(url)
        .timeout(Duration::from_secs(20))
        .set("Authorization", &format!("bearer {token}"))
        .set("User-Agent", UA)
        .call()
        .ok()?
        .into_json()
        .ok()
}

// --- Cache disque du catalogue (résolution une seule fois par jeu) ---

type Cache = HashMap<String, EpicMeta>;

fn cache_file(config_dir: &Path) -> PathBuf {
    // Suffixe versionné : à incrémenter quand le filtre/schéma change (les anciennes
    // entrées sont alors ignorées et re-résolues). v2 = filtre catégorie `games`.
    config_dir.join("epic_catalog_cache_v2.json")
}

fn load_cache(config_dir: &Path) -> Cache {
    std::fs::read_to_string(cache_file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_cache(config_dir: &Path, cache: &Cache) {
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(cache_file(config_dir), json);
    }
}

fn persist_refresh(config_dir: &Path, new_token: &str) {
    let mut creds = super::secrets::load(config_dir);
    if creds.epic_refresh_token.as_deref() != Some(new_token) {
        creds.epic_refresh_token = Some(new_token.to_string());
        let _ = super::secrets::save(config_dir, &creds);
    }
}

/// Encodage URL minimal (composant de requête).
fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
