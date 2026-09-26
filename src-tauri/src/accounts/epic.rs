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
/// Dernier jeton d'accès obtenu, et quand il a été obtenu.
///
/// 🔑 **UN SEUL RENOUVELLEMENT À LA FOIS.** Epic fait tourner le jeton de renouvellement :
/// chaque échange en rend un nouveau, qu'on enregistre. Deux échanges simultanés (le scan
/// et l'ouverture d'une fiche) partiraient du même jeton, et le second enregistré pourrait
/// être celui qu'Epic vient d'invalider — il faudrait alors se reconnecter. Le verrou
/// sérialise les échanges, et le jeton d'accès (valable des heures) est réutilisé entre-temps.
static JETON: std::sync::Mutex<Option<(String, std::time::Instant)>> = std::sync::Mutex::new(None);

/// Durée pendant laquelle on réutilise un jeton d'accès. Epic en donne plusieurs heures ;
/// on reste bien en deçà.
const JETON_VALIDE: Duration = Duration::from_secs(3600);

/// Renouvelle la session (sous le verrou) et mémorise le jeton d'accès.
fn renouveler(
    config_dir: &Path,
    refresh_token: &str,
    garde: &mut Option<(String, std::time::Instant)>,
) -> Option<Tokens> {
    let tokens = refresh(refresh_token)?;
    persist_refresh(config_dir, &tokens.refresh_token);
    *garde = Some((tokens.access_token.clone(), std::time::Instant::now()));
    Some(tokens)
}

/// Un jeton d'accès valide : celui du dernier renouvellement s'il est récent, sinon un neuf.
fn jeton_acces(config_dir: &Path) -> Option<String> {
    let mut garde = JETON.lock().ok()?;
    if let Some((jeton, quand)) = garde.as_ref() {
        if quand.elapsed() < JETON_VALIDE {
            return Some(jeton.clone());
        }
    }
    // Relu sur le disque SOUS le verrou : c'est le dernier jeton tourné qui compte.
    let rt = super::secrets::load(config_dir).epic_refresh_token?;
    renouveler(config_dir, &rt, &mut garde).map(|t| t.access_token)
}

/// Fiche d'un jeu Epic dans la langue de l'interface, lue dans le catalogue Epic.
///
/// 🔑 PAR IDENTIFIANT, DONC SÛRE. Sans elle, un jeu Epic était deviné par son titre sur
/// le Steam Store — et « Control » y tombe sur CONTROL Resonant, un autre jeu. Le
/// catalogue est interrogé avec le namespace et l'id d'item du jeu possédé (relevés par
/// `owned_games` dans `epic_install_ids.json`) : la réponse ne peut désigner que lui.
///
/// ⚠️ Le catalogue exige une session (401 sans jeton) : seulement pour un compte connecté.
pub fn fiche(config_dir: &Path, app_name: &str) -> Option<crate::models::GameMeta> {
    let ids: HashMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(config_dir.join("epic_install_ids.json")).ok()?)
            .ok()?;
    let triplet = ids.get(app_name)?;
    let mut parts = triplet.split("%3A");
    let (namespace, catalog_id) = (parts.next()?, parts.next()?);

    let token = jeton_acces(config_dir)?;
    let locale = if crate::locale::en() { "en-US" } else { "fr" };
    let url = format!(
        "https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace/{namespace}/bulk/items?id={catalog_id}&includeMainGameDetails=true&country={}&locale={locale}",
        crate::locale::region()
    );
    let root = get_json_auth(&url, &token)?;
    let item = root.get(catalog_id)?;
    let titre = item["title"].as_str().unwrap_or_default().trim();
    // ⚠️ L'élément du catalogue est l'objet TECHNIQUE du jeu : pour les titres anciens, sa
    // « description » n'est que le titre répété (Control : « Control »). La vraie est sur
    // l'OFFRE de la boutique, lue alors en second appel.
    let description = description_utile(item["description"].as_str(), titre)
        .or_else(|| description_offre(&token, namespace, catalog_id, locale, titre));
    let screenshots = item["keyImages"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|i| i["type"].as_str() == Some("Screenshot"))
                .filter_map(|i| i["url"].as_str().map(String::from))
                .take(4)
                .collect()
        })
        .unwrap_or_default();

    Some(crate::models::GameMeta {
        name: item["title"].as_str().map(str::trim).map(String::from),
        developer: item["developer"].as_str().map(String::from),
        cover_url: key_image(item, &["DieselGameBoxTall", "OfferImageTall", "Thumbnail"]),
        hero_url: key_image(item, &["DieselGameBox", "DieselGameBoxWide", "OfferImageWide"]),
        // La description d'Epic est parfois vide : on ne la dit traduite que si elle existe,
        // pour ne pas effacer celle d'IGDB avec rien.
        localized: description.is_some(),
        description,
        screenshots,
        ..Default::default()
    })
}

/// Une description qui dit quelque chose : ni vide, ni le titre répété.
fn description_utile(texte: Option<&str>, titre: &str) -> Option<String> {
    let t = texte?.trim();
    (t.chars().count() >= 40 && !t.eq_ignore_ascii_case(titre)).then(|| t.to_string())
}

/// Description de l'offre « jeu de base » du namespace, dans la langue demandée.
///
/// Le namespace d'un jeu contient aussi ses éditions, DLC, démos et offres internes
/// (« CallunaQAAudience »…) : on prend l'offre `BASE_GAME` qui contient notre item, à
/// défaut n'importe quelle `BASE_GAME`, puis une `EDITION` contenant l'item.
fn description_offre(token: &str, namespace: &str, catalog_id: &str, locale: &str, titre: &str) -> Option<String> {
    let url = format!(
        "https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace/{namespace}/offers?status=SUNSET%7CACTIVE&country={}&locale={locale}&start=0&count=50",
        crate::locale::region()
    );
    let offres = get_json_auth(&url, token)?["elements"].as_array()?.clone();
    let contient = |o: &Value| {
        o["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|i| i["id"].as_str() == Some(catalog_id)))
    };
    let type_ = |o: &Value, t: &str| o["offerType"].as_str() == Some(t);
    let choix = offres
        .iter()
        .find(|o| type_(o, "BASE_GAME") && contient(o))
        .or_else(|| offres.iter().find(|o| type_(o, "BASE_GAME")))
        .or_else(|| offres.iter().find(|o| type_(o, "EDITION") && contient(o)))?;
    description_utile(choix["description"].as_str(), titre)
}

pub fn owned_games(config_dir: &Path, refresh_token: &str) -> Vec<GameDto> {
    let tokens = {
        let Ok(mut garde) = JETON.lock() else {
            return Vec::new();
        };
        match renouveler(config_dir, refresh_token, &mut garde) {
            Some(t) => t,
            None => return Vec::new(),
        }
    };

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
    // Triplet d'installation `namespace%3AcatalogItemId%3AappName` par appName : le deeplink
    // Epic `action=install` l'exige (comme uninstall), et un jeu possédé NON installé n'a pas
    // de manifeste local. On le persiste ici pour le résoudre au clic sur « Installer ».
    let mut install_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
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
        install_ids.insert(
            asset.app_name.clone(),
            format!("{}%3A{}%3A{}", asset.namespace, asset.catalog_item_id, asset.app_name),
        );
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
    // Cache des triplets d'installation (best-effort : une écriture ratée n'est pas bloquante).
    if let Ok(json) = serde_json::to_string(&install_ids) {
        let _ = std::fs::write(config_dir.join("epic_install_ids.json"), json);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Le catalogue répète souvent le titre en guise de description : ce n'en est pas une.
    #[test]
    fn une_description_qui_repete_le_titre_est_ignoree() {
        assert_eq!(description_utile(Some("Control"), "Control"), None);
        assert_eq!(description_utile(Some("  "), "Control"), None);
        assert_eq!(description_utile(None, "Control"), None);
        let vraie = "Suite à l'invasion d'une agence secrète new-yorkaise par une force inconnue";
        assert_eq!(description_utile(Some(vraie), "Control").as_deref(), Some(vraie));
    }
}
