use crate::models::GameDto;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Identifiants publics du client GOG Galaxy (les mêmes que ceux embarqués dans
/// le client officiel — utilisés par tous les outils GOG open source). Ils ne
/// sont PAS secrets : ils identifient l'application, pas l'utilisateur.
const CLIENT_ID: &str = "46899977096215655";
const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
/// URL de redirection attendue après login : la fenêtre y arrive avec `?code=…`.
pub const REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// Réponse du point de terminaison OAuth `token` de GOG.
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
}

/// Échange un code d'autorisation (récupéré via la fenêtre de login) contre des
/// jetons d'accès et de rafraîchissement.
pub fn exchange_code(code: &str) -> Option<Tokens> {
    token_request(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
    ])
}

/// Redérive un access token frais à partir du refresh token (rotation incluse :
/// GOG renvoie à chaque fois un nouveau refresh token).
fn refresh(refresh_token: &str) -> Option<Tokens> {
    token_request(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ])
}

fn token_request(extra: &[(&str, &str)]) -> Option<Tokens> {
    let mut req = ureq::get("https://auth.gog.com/token")
        .timeout(Duration::from_secs(20))
        .query("client_id", CLIENT_ID)
        .query("client_secret", CLIENT_SECRET);
    for (k, v) in extra {
        req = req.query(k, v);
    }
    let json: Value = req.set("User-Agent", UA).call().ok()?.into_json().ok()?;
    // `user_id` peut arriver en chaîne ou en nombre (il dépasse 2^53).
    let user_id = json["user_id"]
        .as_str()
        .map(String::from)
        .or_else(|| json["user_id"].as_u64().map(|n| n.to_string()))
        .unwrap_or_default();
    Some(Tokens {
        access_token: json["access_token"].as_str()?.to_string(),
        refresh_token: json["refresh_token"].as_str()?.to_string(),
        user_id,
    })
}

/// Bibliothèque GOG possédée. Rafraîchit le jeton, persiste le refresh token
/// éventuellement renouvelé, pagine `getFilteredProducts` (jeux uniquement),
/// puis applique le temps de jeu / dernière session (un seul appel bulk).
pub fn owned_games(config_dir: &Path, refresh_token: &str) -> Vec<GameDto> {
    let Some(tokens) = refresh(refresh_token) else {
        return Vec::new();
    };
    persist_refresh(config_dir, &tokens.refresh_token);
    let mut games = fetch_all_products(&tokens.access_token);

    let stats = fetch_playtime(&tokens.access_token, &tokens.user_id);
    for game in &mut games {
        let Some(id) = game
            .id
            .strip_prefix("gog:")
            .and_then(|s| s.parse::<u64>().ok())
        else {
            continue;
        };
        if let Some(&(minutes, last)) = stats.get(&id) {
            if minutes > 0 {
                game.playtime_minutes = Some(minutes);
            }
            if last.is_some() {
                game.last_played = last;
            }
        }
    }
    games
}

/// Temps de jeu et dernière session par jeu (`gameplay.gog.com/.../statistics`).
/// Retourne une table id_produit → (minutes jouées, horodatage Unix dernière session).
fn fetch_playtime(access_token: &str, user_id: &str) -> HashMap<u64, (u32, Option<i64>)> {
    let url = format!("https://gameplay.gog.com/users/{user_id}/statistics");
    let Some(json) = get_json_auth(&url, access_token) else {
        return HashMap::new();
    };
    let Some(obj) = json.as_object() else {
        return HashMap::new();
    };
    obj.iter()
        .filter_map(|(gid, stat)| {
            let id = gid.parse::<u64>().ok()?;
            let minutes = stat["playtime"].as_u64().unwrap_or(0) as u32;
            let last = stat["last_session"].as_str().and_then(parse_iso_to_unix);
            Some((id, (minutes, last)))
        })
        .collect()
}

/// Convertit une date ISO 8601 UTC « 2026-07-02T21:03:49+00:00 » en horodatage
/// Unix (secondes). Format fixe renvoyé par GOG ; renvoie None si non conforme.
fn parse_iso_to_unix(s: &str) -> Option<i64> {
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
    // Jours écoulés depuis l'époque Unix (algorithme « days_from_civil »).
    let y = if mo <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + h * 3600 + mi * 60 + se)
}

fn fetch_all_products(access_token: &str) -> Vec<GameDto> {
    let mut games = Vec::new();
    let mut page = 1u32;
    loop {
        let url = format!(
            "https://embed.gog.com/account/getFilteredProducts?mediaType=1&page={page}"
        );
        let Some(json) = get_json_auth(&url, access_token) else {
            break;
        };
        if let Some(products) = json["products"].as_array() {
            games.extend(products.iter().filter_map(parse_product));
        }
        let total = json["totalPages"].as_u64().unwrap_or(1);
        if page as u64 >= total {
            break;
        }
        page += 1;
    }
    games
}

fn parse_product(product: &Value) -> Option<GameDto> {
    let id = product["id"].as_u64()?;
    let title = product["title"].as_str()?.trim().to_string();
    if title.is_empty() {
        return None;
    }
    // `image` est un préfixe protocol-relatif ("//images…/hash") sans extension ;
    // on lui ajoute la transformation "vertical cover" (jaquette portrait).
    let cover_url = product["image"]
        .as_str()
        .filter(|img| img.starts_with("//"))
        .map(|img| format!("https:{img}_glx_vertical_cover.jpg"));

    Some(GameDto {
        id: format!("gog:{id}"),
        title,
        platform: "gog".into(),
        installed: false,
        owned: true,
        cover_url,
        launch_target: id.to_string(),
        app_type: Some("game".into()),
        ..Default::default()
    })
}

fn get_json_auth(url: &str, token: &str) -> Option<Value> {
    ureq::get(url)
        .timeout(Duration::from_secs(20))
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", UA)
        .call()
        .ok()?
        .into_json()
        .ok()
}

/// Enregistre le refresh token renouvelé (rotation GOG), si différent.
fn persist_refresh(config_dir: &Path, new_token: &str) {
    let mut creds = super::secrets::load(config_dir);
    if creds.gog_refresh_token.as_deref() != Some(new_token) {
        creds.gog_refresh_token = Some(new_token.to_string());
        let _ = super::secrets::save(config_dir, &creds);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_iso_to_unix;

    #[test]
    fn parses_iso_utc_dates() {
        assert_eq!(parse_iso_to_unix("1970-01-01T00:00:00+00:00"), Some(0));
        // Année bissextile (29 février), passage à midi.
        assert_eq!(parse_iso_to_unix("2000-02-29T12:00:00+00:00"), Some(951_825_600));
        // Dernière session GOG réelle (Witcher 3).
        assert_eq!(parse_iso_to_unix("2026-07-02T21:03:49+00:00"), Some(1_783_026_229));
        assert_eq!(parse_iso_to_unix("pas une date"), None);
    }
}
