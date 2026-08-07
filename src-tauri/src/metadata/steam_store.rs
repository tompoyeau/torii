use crate::models::GameMeta;
use serde_json::Value;
use std::time::Duration;

const CDN: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps";

fn get_json(url: &str) -> Option<Value> {
    ureq::get(url)
        .timeout(Duration::from_secs(12))
        .call()
        .ok()?
        .into_json()
        .ok()
}

/// Récupère les métadonnées d'un jeu Steam via l'API publique `appdetails`.
pub fn appdetails(appid: &str) -> Option<GameMeta> {
    let url =
        format!("https://store.steampowered.com/api/appdetails?appids={appid}&l=english&cc=us");
    let root = get_json(&url)?;
    let entry = root.get(appid)?;
    if !entry["success"].as_bool().unwrap_or(false) {
        return None;
    }
    let data = entry.get("data")?;

    let genre = data["genres"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g["description"].as_str())
                .take(2)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty());

    let screenshots = data["screenshots"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s["path_full"].as_str().map(String::from))
                .take(4)
                .collect()
        })
        .unwrap_or_default();

    Some(GameMeta {
        name: data["name"].as_str().map(String::from),
        genre,
        description: data["short_description"].as_str().map(String::from),
        developer: data["developers"][0].as_str().map(String::from),
        year: data["release_date"]["date"].as_str().and_then(parse_year),
        cover_url: Some(format!("{CDN}/{appid}/library_600x900.jpg")),
        hero_url: Some(format!("{CDN}/{appid}/library_hero.jpg")),
        screenshots,
        app_type: data["type"].as_str().map(String::from),
        // Steam n'expose pas la taille d'installation via une API publique.
        size_gb: None,
    })
}

/// Estime la taille d'installation (Go) d'un jeu Steam. `appdetails` ne la fournit
/// pas ; on somme la taille des dépôts via l'API publique tierce api.steamcmd.net.
/// Heuristique (valeur indicative) : dépôts de la branche « public », Windows,
/// hors DLC et hors langues autres qu'anglais (évite de compter toutes les langues).
pub fn install_size_gb(appid: &str) -> Option<f64> {
    let root = get_json(&format!("https://api.steamcmd.net/v1/info/{appid}"))?;
    let depots = root["data"][appid]["depots"].as_object()?;

    let mut bytes: u64 = 0;
    for (id, depot) in depots {
        // On ne garde que les vrais dépôts (clés numériques), pas les métadonnées.
        if !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if depot.get("dlcappid").is_some() {
            continue;
        }
        if let Some(os) = depot["config"]["oslist"].as_str() {
            if !os.contains("windows") {
                continue;
            }
        }
        if let Some(lang) = depot["config"]["language"].as_str() {
            if !lang.eq_ignore_ascii_case("english") {
                continue;
            }
        }
        // Les tailles sont des chaînes d'octets dans le JSON.
        if let Some(size) = depot["manifests"]["public"]["size"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
        {
            bytes += size;
        }
    }

    (bytes > 0).then(|| (bytes as f64 / 1024_f64.powi(3) * 10.0).round() / 10.0)
}

/// URL de la jaquette portrait Steam, déterministe à partir de l'appid (CDN public).
pub fn library_cover(appid: &str) -> String {
    format!("{CDN}/{appid}/library_600x900.jpg")
}

/// Cherche l'appid Steam correspondant à un titre (pour enrichir Epic/GOG/manuel/…).
/// Conservateur : n'accepte qu'une correspondance de nom suffisamment proche.
///
/// Repli : si le titre complet ne matche pas et se termine par un nombre isolé, on
/// retente sans (ex. « Overwatch 2 » → « Overwatch » : le jeu de base est listé sur
/// Steam sous « Overwatch® », et les jeux free-to-play sont exclus du search complet).
pub fn search_appid(title: &str) -> Option<String> {
    if let Some(id) = search_exact(title) {
        return Some(id);
    }
    if let Some((base, num)) = title.rsplit_once(' ') {
        if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
            return search_exact(base);
        }
    }
    None
}

/// Recherche stricte : ne renvoie un appid que si un résultat a un nom assez proche.
fn search_exact(title: &str) -> Option<String> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&cc=us&l=english",
        percent_encode(title)
    );
    let root = get_json(&url)?;
    let want = normalize(title);
    for item in root["items"].as_array()? {
        let (Some(id), Some(name)) = (item["id"].as_u64(), item["name"].as_str()) else {
            continue;
        };
        let got = normalize(name);
        if got == want || (want.len() > 4 && got.contains(&want)) {
            return Some(id.to_string());
        }
    }
    None
}

/// Extrait une année (19xx / 20xx) d'une date texte libre.
fn parse_year(date: &str) -> Option<i32> {
    let bytes = date.as_bytes();
    for w in bytes.windows(4) {
        if (w[0] == b'1' && w[1] == b'9' || w[0] == b'2' && w[1] == b'0')
            && w[2].is_ascii_digit()
            && w[3].is_ascii_digit()
        {
            return std::str::from_utf8(w).ok()?.parse().ok();
        }
    }
    None
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "%20".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
