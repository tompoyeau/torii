//! Bibliothèque possédée EA (app EA / ex-Origin) via l'API GraphQL « Juno ».
//!
//! Flux (calqué sur Lutris) : login web dans une WebviewWindow → EA pose ses cookies ;
//! on récupère un `access_token` en naviguant vers `accounts.ea.com/connect/auth`
//! (`response_type=token`, capté dans la webview car ce host utilise une vieille
//! renégociation TLS que rustls refuse) ; puis on interroge l'API Juno (TLS moderne, OK
//! en Rust) : entitlements possédés → détails (titre + jaquettes) → cache disque.

use crate::models::GameDto;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API_URL: &str = "https://service-aggregation-layer.juno.ea.com/graphql";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Ludo/1.0";

/// Endpoint qui renvoie un `access_token` JSON quand la session est valide.
pub const TOKEN_ENDPOINT: &str = "https://accounts.ea.com/connect/auth\
?client_id=ORIGIN_JS_SDK&response_type=token&redirect_uri=nucleus:rest&prompt=none";

/// Page de login EA actuelle. Après connexion, la fenêtre revient sur `www.ea.com`
/// (l'ancienne URL Origin `client_id=ORIGIN_SPA_ID` est dépréciée → « Service limitations »).
pub fn login_url() -> String {
    "https://www.ea.com/login".to_string()
}

/// Récupère toute la bibliothèque possédée (jeux de base) à partir d'un access token.
pub fn fetch_library(token: &str) -> Vec<GameDto> {
    let offer_ids = entitlements(token);
    let mut games = Vec::new();
    // L'API accepte des lots d'offerIds ; on borne à 100 par requête.
    for chunk in offer_ids.chunks(100) {
        games.extend(game_details(token, chunk));
    }
    games
}

/// Liste des `originOfferId` possédés, filtrés sur les jeux de base (pas les DLC/éditions).
///
/// On interroge d'abord les storefronts EA **et ORIGIN** (les jeux d'avant l'app EA —
/// Dragon Age, Battlefront… — restent rattachés à ORIGIN). Repli sur EA seul si le
/// schéma refuse la valeur ORIGIN (requête en erreur → aucun résultat).
fn entitlements(token: &str) -> Vec<String> {
    let ids = collect_entitlements(token, &ent_query("[EA, ORIGIN]"));
    if !ids.is_empty() {
        return ids;
    }
    collect_entitlements(token, &ent_query("[EA]"))
}

/// Requête GraphQL des entitlements pour une liste de storefronts donnée.
fn ent_query(storefronts: &str) -> String {
    format!(
        r#"query getEntitlements($limit: Int, $next: String) {{
  me {{
    ownedGameProducts(
      locale: "DEFAULT"
      entitlementEnabled: true
      storefronts: {storefronts}
      type: [DIGITAL_FULL_GAME, PACKAGED_FULL_GAME]
      platforms: [PC]
      paging: {{ limit: $limit, next: $next }}
    ) {{
      next
      items {{ originOfferId product {{ baseItem {{ gameType }} }} }}
    }}
  }}
}}"#
    )
}

fn collect_entitlements(token: &str, query: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut next: Option<String> = None;
    // Pagination via le curseur `next` (limite ~50 pages de sécurité).
    for _ in 0..50 {
        let vars = json!({ "limit": 100, "next": next });
        let Some(resp) = graphql(token, query, vars) else {
            break;
        };
        let owned = &resp["data"]["me"]["ownedGameProducts"];
        if let Some(items) = owned["items"].as_array() {
            for it in items {
                let is_base = it["product"]["baseItem"]["gameType"].as_str() == Some("BASE_GAME");
                if let (true, Some(oid)) = (is_base, it["originOfferId"].as_str()) {
                    ids.push(oid.to_string());
                }
            }
        }
        match owned["next"].as_str() {
            Some(n) => next = Some(n.to_string()),
            None => break,
        }
    }
    ids
}

/// Détails (titre + jaquettes + contentId de lancement) pour un lot d'offerIds.
fn game_details(token: &str, offer_ids: &[String]) -> Vec<GameDto> {
    const QUERY: &str = r#"query getOffers($offerIds: [String!]!) {
  legacyOffers(offerIds: $offerIds, locale: "DEFAULT") { offerId: id contentId }
  gameProducts(offerIds: $offerIds, locale: "DEFAULT") {
    items {
      id
      originOfferId
      gameSlug
      baseItem {
        keyArt { largestImage { path } }
        packArt { largestImage { path } }
        title
      }
    }
  }
}"#;

    let Some(resp) = graphql(token, QUERY, json!({ "offerIds": offer_ids })) else {
        return Vec::new();
    };
    let data = &resp["data"];
    let products: Vec<Value> = data["gameProducts"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Index par originOfferId et par id produit (le legacyOffer peut pointer l'un ou l'autre).
    let by_offer: HashMap<&str, &Value> = products
        .iter()
        .filter_map(|p| p["originOfferId"].as_str().map(|k| (k, p)))
        .collect();
    let by_id: HashMap<&str, &Value> = products
        .iter()
        .filter_map(|p| p["id"].as_str().map(|k| (k, p)))
        .collect();

    let mut games = Vec::new();
    let Some(legacy) = data["legacyOffers"].as_array() else {
        return games;
    };
    for lo in legacy {
        let Some(offer_id) = lo["offerId"].as_str() else {
            continue;
        };
        let content_id = lo["contentId"].as_str().unwrap_or("");
        let product = by_offer
            .get(offer_id)
            .copied()
            .or_else(|| (!content_id.is_empty()).then(|| by_id.get(content_id).copied()).flatten())
            .or_else(|| by_id.get(offer_id).copied());
        let Some(product) = product else {
            continue; // identifié mais sans fiche produit → on saute
        };
        let base = &product["baseItem"];
        let Some(title) = base["title"].as_str() else {
            continue;
        };
        // Cible de lancement : le contentId si présent, sinon l'offerId.
        let launch = if content_id.is_empty() { offer_id } else { content_id };

        games.push(GameDto {
            id: format!("ea:{launch}"),
            title: title.to_string(),
            platform: "ea".into(),
            installed: false,
            owned: true,
            cover_url: base["packArt"]["largestImage"]["path"]
                .as_str()
                .map(str::to_string),
            hero_url: base["keyArt"]["largestImage"]["path"]
                .as_str()
                .map(str::to_string),
            launch_target: launch.to_string(),
            ..Default::default()
        });
    }
    games
}

/// POST GraphQL authentifié vers l'API Juno.
fn graphql(token: &str, query: &str, variables: Value) -> Option<Value> {
    ureq::post(API_URL)
        .timeout(Duration::from_secs(30))
        .set("Authorization", &format!("Bearer {token}"))
        .set("AuthToken", token)
        .set("X-AuthToken", token)
        .set("User-Agent", UA)
        .send_json(json!({ "query": query, "variables": variables }))
        .ok()?
        .into_json()
        .ok()
}

// --- Cache disque de la bibliothèque (snapshot pris à la connexion) --------------------

fn library_path(config_dir: &Path) -> PathBuf {
    config_dir.join("ea_library.json")
}

/// Enregistre le snapshot de la bibliothèque EA.
pub fn save_library(config_dir: &Path, games: &[GameDto]) {
    if let Ok(json) = serde_json::to_string(games) {
        let _ = std::fs::write(library_path(config_dir), json);
    }
}

/// Charge le snapshot de la bibliothèque EA (vide si non connecté).
pub fn load_library(config_dir: &Path) -> Vec<GameDto> {
    std::fs::read_to_string(library_path(config_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Vrai si une bibliothèque EA a été récupérée (= « connecté »).
pub fn is_connected(config_dir: &Path) -> bool {
    library_path(config_dir).is_file()
}

/// Efface le snapshot (déconnexion).
pub fn disconnect(config_dir: &Path) {
    let _ = std::fs::remove_file(library_path(config_dir));
}
