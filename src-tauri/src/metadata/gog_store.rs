use crate::models::GameMeta;
use serde_json::Value;
use std::time::Duration;

fn get_json(url: &str) -> Option<Value> {
    ureq::get(url)
        .timeout(Duration::from_secs(12))
        .call()
        .ok()?
        .into_json()
        .ok()
}

/// Métadonnées d'un jeu GOG via l'API v2 publique (celle de GOG Galaxy).
/// Un seul appel fournit description, captures, développeur, année et genre.
pub fn product(product_id: &str) -> Option<GameMeta> {
    let root = get_json(&format!("https://api.gog.com/v2/games/{product_id}"))?;
    let emb = &root["_embedded"];

    let description = root["description"]
        .as_str()
        .map(strip_html)
        .filter(|s| !s.is_empty());

    let developer = emb["developers"][0]["name"].as_str().map(String::from);

    let year = emb["product"]["globalReleaseDate"]
        .as_str()
        .and_then(parse_year);

    // GOG n'expose pas de « genre » franc ; ses tags en tiennent lieu.
    let genre = emb["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["name"].as_str())
                .take(2)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty());

    // Les captures sont des URL « templatées » : on remplace {formatter} par la
    // taille voulue. La 1re sert aussi de visuel paysage (bannière détail).
    let raw: Vec<&str> = emb["screenshots"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s["_links"]["self"]["href"].as_str())
                .collect()
        })
        .unwrap_or_default();
    let screenshots: Vec<String> = raw
        .iter()
        .take(4)
        .map(|h| h.replace("{formatter}", "product_card_screenshot_748_2x"))
        .collect();
    let hero_url = raw.first().map(|h| h.replace("{formatter}", "1600"));

    // `size` est la taille de téléchargement en Mo (Witcher 3 = 81961 ≈ 80 Go).
    let size_gb = root["size"]
        .as_f64()
        .filter(|&mb| mb > 0.0)
        .map(|mb| (mb / 1024.0 * 10.0).round() / 10.0);

    Some(GameMeta {
        name: emb["product"]["title"].as_str().map(String::from),
        genre,
        description,
        developer,
        year,
        cover_url: None,
        hero_url,
        screenshots,
        app_type: Some("game".into()),
        size_gb,
    })
}

/// Année (aaaa) au début d'une date ISO « 2016-08-30T… ».
fn parse_year(date: &str) -> Option<i32> {
    date.get(0..4).and_then(|y| y.parse().ok())
}

/// Transforme un fragment HTML GOG en texte lisible : retire les balises,
/// décode les entités courantes, compacte les espaces et tronque proprement.
fn strip_html(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    // Compacte les espaces/sauts de ligne multiples.
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_on_word(&compact, 600)
}

/// Tronque à `max` caractères sans couper un mot ; ajoute « … » si tronqué.
fn truncate_on_word(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    let end = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..end].trim_end_matches([',', ';', ':', '.']))
}
