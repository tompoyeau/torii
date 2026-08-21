//! Boutique de découverte via **IsThereAnyDeal (ITAD)** (agrégateur de prix, 60+
//! boutiques, prix régionaux natifs). Torii ne vend pas : il **agrège** et le bouton
//! « Acheter » ouvre la vraie boutique. Visuels/description enrichis via IGDB.
//!
//! ITAD exige une clé d'API → jamais embarquée : relayée par le **Worker Cloudflare**
//! (comme IGDB), qui l'injecte côté serveur. On appelle donc `<proxy>/itad/<endpoint>`.
//! Avantages vs CheapShark : **prix EUR natifs** (`country=FR`, pas de conversion),
//! **lien d'achat direct** par boutique, match par **appid Steam** exact, plus bas
//! historiques, et pas de blocage IP sauvage.
//!
//! Trois usages, tous à la demande :
//! - `deals(page, sort)` : la vitrine (`GET /deals/v2`).
//! - `search(query)` : recherche (`GET /games/search/v1` + `POST /games/prices/v3`).
//! - `game(id)` : fiche produit (`info` + `prices` + `overview`) + IGDB + Instant Gaming.

use crate::models::GameDto;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const PROXY: &str = "https://torii-igdb-proxy.toriiapp.workers.dev/itad";
/// Pays pour la tarification régionale (→ prix en EUR).
const COUNTRY: &str = "FR";
const PAGE_SIZE: u32 = 48;

/// Un jeu de la vitrine / des résultats de recherche (carte de grille).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoreItem {
    /// Identifiant ITAD du jeu (UUID ; clé de la fiche produit).
    pub game_id: String,
    pub title: String,
    /// Jaquette (ITAD boxart) si disponible, sinon le front met un dégradé.
    pub cover_url: Option<String>,
    /// Prix actuel le plus bas (EUR).
    pub price: f64,
    /// Prix normal (hors promo) ; == price si inconnu.
    pub normal_price: f64,
    /// Remise en % entier (0 = pas de promo / inconnu).
    pub savings: u32,
    /// Boutique de la meilleure offre (vide si non résolu).
    pub store_name: String,
    /// Lien d'achat direct.
    pub buy_url: String,
}

/// Suggestion d'autocomplétion (recherche instantanée, sans prix → rapide).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub game_id: String,
    pub title: String,
    pub cover_url: Option<String>,
}

/// Une offre d'une boutique (ligne du comparatif de la fiche produit).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StorePrice {
    pub store_name: String,
    pub price: f64,
    pub retail_price: f64,
    pub savings: u32,
    pub buy_url: String,
    /// `false` si l'offre est en rupture de stock (Instant Gaming). Les offres ITAD
    /// sont toujours considérées disponibles (elles proviennent de deals en cours).
    pub available: bool,
}

/// Fiche produit : comparatif de prix + métadonnée descriptive (IGDB).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoreGame {
    pub game_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    /// Prix le plus bas jamais atteint (EUR), si connu.
    pub cheapest_ever: Option<f64>,
    /// Offres par boutique, triées par prix croissant.
    pub prices: Vec<StorePrice>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub developer: Option<String>,
    pub year: Option<i64>,
    pub screenshots: Vec<String>,
}

// --- Accès HTTP au proxy ITAD -------------------------------------------------

fn get_json(path: &str) -> Option<Value> {
    ureq::get(&format!("{PROXY}/{path}"))
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?
        .into_json()
        .ok()
}

fn post_json(path: &str, body: &Value) -> Option<Value> {
    ureq::post(&format!("{PROXY}/{path}"))
        .timeout(Duration::from_secs(15))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .ok()?
        .into_json()
        .ok()
}

/// Lit un nombre qu'il soit encodé en JSON number ou en chaîne.
fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Nom de la boutique d'une offre ITAD.
fn shop_of(deal: &Value) -> &str {
    deal["shop"]["name"].as_str().unwrap_or_default()
}

/// Une offre est retenue si sa boutique n'est pas masquée par l'utilisateur.
/// 🔑 Le choix de la « meilleure offre » se fait ICI, côté Rust : la vitrine, la
/// recherche et la wishlist ne renvoient qu'UN prix par jeu, donc le front n'a aucun
/// moyen de re-choisir. Sans ce filtre, un revendeur masqué continuait d'apparaître
/// comme meilleur prix partout sauf sur la fiche produit.
fn allowed(deal: &Value, excluded: &HashSet<String>) -> bool {
    excluded.is_empty() || !excluded.contains(shop_of(deal))
}

/// Meilleure offre (prix mini) d'une entrée `games/prices/v3`, hors revendeurs masqués.
fn cheapest<'a>(entry: &'a Value, excluded: &HashSet<String>) -> Option<&'a Value> {
    entry["deals"]
        .as_array()?
        .iter()
        .filter(|d| allowed(d, excluded))
        .min_by(|a, b| {
            num(&a["price"]["amount"])
                .unwrap_or(f64::MAX)
                .partial_cmp(&num(&b["price"]["amount"]).unwrap_or(f64::MAX))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Applique une offre à un item de grille (vitrine / recherche).
fn apply_deal(item: &mut StoreItem, deal: &Value) -> bool {
    let Some(price) = num(&deal["price"]["amount"]) else {
        return false;
    };
    item.price = price;
    item.normal_price = num(&deal["regular"]["amount"]).unwrap_or(price);
    item.savings = num(&deal["cut"]).unwrap_or(0.0).round() as u32;
    item.store_name = shop_of(deal).to_string();
    item.buy_url = deal["url"].as_str().unwrap_or_default().to_string();
    true
}

/// Traduit un critère de tri de la vitrine en `sort` ITAD.
fn sort_param(sort: &str) -> &'static str {
    match sort {
        "savings" => "-cut",
        "price" => "price",
        "recent" => "-time",
        "rating" => "-rating",
        _ => "-trending", // « mises en avant » par défaut
    }
}

/// Construit un `StoreItem` depuis un objet deal ITAD (`/deals/v2` `list[]`).
fn item_from_deal(it: &Value) -> Option<StoreItem> {
    // Le prix/la boutique sont sous `deal` ; certains champs (assets) au niveau du jeu.
    let deal = it.get("deal").unwrap_or(it);
    let price = num(&deal["price"]["amount"])?;
    let regular = num(&deal["regular"]["amount"]).unwrap_or(price);
    Some(StoreItem {
        game_id: it["id"].as_str()?.to_string(),
        title: it["title"].as_str()?.to_string(),
        cover_url: it["assets"]["boxart"].as_str().map(String::from),
        price,
        normal_price: regular,
        savings: num(&deal["cut"]).unwrap_or(0.0).round() as u32,
        store_name: deal["shop"]["name"].as_str().unwrap_or_default().to_string(),
        buy_url: deal["url"].as_str().unwrap_or_default().to_string(),
    })
}

/// Vitrine : une page de jeux mis en avant / en promo, selon le tri choisi. Prix EUR.
/// Les offres des revendeurs masqués sont remplacées par la meilleure offre autorisée
/// du même jeu (cf. `reprice_excluded`).
pub fn deals(page: u32, sort: &str, excluded: &HashSet<String>) -> Vec<StoreItem> {
    let offset = page * PAGE_SIZE;
    let path = format!(
        "deals/v2?country={COUNTRY}&offset={offset}&limit={PAGE_SIZE}&sort={}",
        sort_param(sort)
    );
    let Some(root) = get_json(&path) else {
        return Vec::new();
    };
    // Réponse : { list: [...] } (ou tableau nu selon versions).
    let list = root
        .get("list")
        .and_then(Value::as_array)
        .or_else(|| root.as_array())
        .cloned()
        .unwrap_or_default();
    let mut items: Vec<StoreItem> = list
        .iter()
        .filter(|it| it["type"].as_str() != Some("dlc")) // jeux/éditions, pas les DLC
        .filter_map(item_from_deal)
        .collect();
    reprice_excluded(&mut items, excluded);
    items
}

/// La vitrine ITAD impose UNE offre par jeu : si elle vient d'un revendeur masqué, on
/// redemande toutes les offres de ces jeux (un seul appel groupé) et on retient la
/// meilleure offre autorisée. Un jeu qui n'a plus aucune offre visible sort de la liste.
fn reprice_excluded(items: &mut Vec<StoreItem>, excluded: &HashSet<String>) {
    if excluded.is_empty() {
        return;
    }
    let ids: Vec<String> = items
        .iter()
        .filter(|i| excluded.contains(&i.store_name))
        .map(|i| i.game_id.clone())
        .collect();
    if ids.is_empty() {
        return;
    }
    let entries: std::collections::HashMap<String, Value> =
        post_json(&format!("games/prices/v3?country={COUNTRY}"), &json!(ids))
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|e| Some((e["id"].as_str()?.to_string(), e)))
            .collect();

    items.retain_mut(|item| {
        if !excluded.contains(&item.store_name) {
            return true;
        }
        match entries.get(&item.game_id).and_then(|e| cheapest(e, excluded)) {
            Some(deal) => apply_deal(item, deal),
            None => false, // plus que des revendeurs masqués : on retire le jeu
        }
    });
}

/// Autocomplétion : suggestions de jeux au fil de la frappe (titre + jaquette),
/// **sans** récupérer les prix → un seul appel léger, adapté à chaque frappe.
pub fn suggest(query: &str) -> Vec<Suggestion> {
    let q = query.trim();
    if q.len() < 2 {
        return Vec::new();
    }
    let found = get_json(&format!("games/search/v1?title={}", urlencode(q)));
    let games = found
        .as_ref()
        .and_then(|v| v.get("results").and_then(Value::as_array).or_else(|| v.as_array()))
        .cloned()
        .unwrap_or_default();
    games
        .iter()
        .filter(|g| g["type"].as_str() != Some("dlc"))
        .filter_map(|g| {
            Some(Suggestion {
                game_id: g["id"].as_str()?.to_string(),
                title: g["title"].as_str()?.to_string(),
                cover_url: g["assets"]["boxart"].as_str().map(String::from),
            })
        })
        .take(8)
        .collect()
}

/// Recherche de jeux par titre : ids via `search`, puis meilleur prix via `prices`
/// (hors revendeurs masqués par l'utilisateur).
pub fn search(query: &str, excluded: &HashSet<String>) -> Vec<StoreItem> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let found = get_json(&format!("games/search/v1?title={}", urlencode(q)));
    let games = found
        .as_ref()
        .and_then(|v| v.get("results").and_then(Value::as_array).or_else(|| v.as_array()))
        .cloned()
        .unwrap_or_default();

    // On écarte les DLC (on garde jeux + éditions/« package ») et on limite le nombre.
    let kept: Vec<&Value> = games
        .iter()
        .filter(|g| g["type"].as_str() != Some("dlc"))
        .take(24)
        .collect();
    let ids: Vec<String> = kept
        .iter()
        .filter_map(|g| g["id"].as_str().map(String::from))
        .collect();
    if ids.is_empty() {
        return Vec::new();
    }
    // Titre + jaquette (boxart) par id, depuis les résultats de recherche.
    let meta: std::collections::HashMap<String, (String, Option<String>)> = kept
        .iter()
        .filter_map(|g| {
            Some((
                g["id"].as_str()?.to_string(),
                (
                    g["title"].as_str()?.to_string(),
                    g["assets"]["boxart"].as_str().map(String::from),
                ),
            ))
        })
        .collect();

    // Prix courants (meilleure offre par jeu) en un seul appel.
    let prices = post_json(&format!("games/prices/v3?country={COUNTRY}"), &json!(ids))
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    prices
        .iter()
        .filter_map(|entry| {
            let id = entry["id"].as_str()?.to_string();
            let best = cheapest(entry, excluded)?;
            let price = num(&best["price"]["amount"])?;
            let (title, cover_url) = meta.get(&id).cloned().unwrap_or_default();
            Some(StoreItem {
                title,
                game_id: id,
                cover_url,
                price,
                normal_price: num(&best["regular"]["amount"]).unwrap_or(price),
                savings: num(&best["cut"]).unwrap_or(0.0).round() as u32,
                store_name: best["shop"]["name"].as_str().unwrap_or_default().to_string(),
                buy_url: best["url"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// Fiche produit : comparatif multi-boutiques (ITAD) + plus bas historique + IGDB + IG.
pub fn game(game_id: &str, config_dir: &Path) -> Option<StoreGame> {
    // 1. Infos jeu (titre, appid Steam, jaquette).
    let info = get_json(&format!("games/info/v2?id={}", urlencode(game_id)))?;
    let title = info["title"].as_str()?.to_string();
    let steam_app_id = info["appid"].as_u64().map(|a| a.to_string());
    let boxart = info["assets"]["boxart"].as_str().map(String::from);

    // 2. Prix par boutique (comparatif) + plus bas historique, en un seul appel.
    let prices_root = post_json(
        &format!("games/prices/v3?country={COUNTRY}"),
        &json!([game_id]),
    );
    let entry = prices_root
        .as_ref()
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .cloned();
    let mut prices: Vec<StorePrice> = entry
        .as_ref()
        .and_then(|e| e["deals"].as_array())
        .map(|deals| {
            deals
                .iter()
                .filter_map(|d| {
                    let price = num(&d["price"]["amount"])?;
                    Some(StorePrice {
                        store_name: d["shop"]["name"].as_str().unwrap_or_default().to_string(),
                        price,
                        retail_price: num(&d["regular"]["amount"]).unwrap_or(price),
                        savings: num(&d["cut"]).unwrap_or(0.0).round() as u32,
                        buy_url: d["url"].as_str().unwrap_or_default().to_string(),
                        available: true, // offres ITAD = deals en cours
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // 3. Instant Gaming (prix EUR natif, absent d'ITAD) — best-effort scrape.
    if let Some(ig) = super::instant_gaming::price(&title) {
        prices.push(StorePrice {
            store_name: "Instant Gaming".into(),
            price: ig.price,
            retail_price: if ig.savings > 0 {
                (ig.price / (1.0 - ig.savings as f64 / 100.0) * 100.0).round() / 100.0
            } else {
                ig.price
            },
            savings: ig.savings,
            buy_url: ig.url,
            available: ig.available,
        });
    }
    prices.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

    // 4. Plus bas historique (all-time), fourni dans la réponse prices (`historyLow.all`).
    let cheapest_ever = entry
        .as_ref()
        .and_then(|e| num(&e["historyLow"]["all"]["amount"]));

    let mut out = StoreGame {
        game_id: game_id.to_string(),
        title: title.clone(),
        cover_url: boxart,
        hero_url: None,
        cheapest_ever,
        prices,
        ..Default::default()
    };

    // 5. Enrichissement descriptif via IGDB (cache disque partagé).
    let dto = match &steam_app_id {
        Some(appid) => GameDto {
            id: format!("steam:{appid}"),
            platform: "steam".into(),
            title: title.clone(),
            ..Default::default()
        },
        None => GameDto {
            id: format!("store:{game_id}"),
            platform: "store".into(),
            title: title.clone(),
            ..Default::default()
        },
    };
    if let Some((_, meta)) = super::igdb::fill_metadata(&[dto], config_dir, |_| {}).into_iter().next() {
        out.description = meta.description;
        out.hero_url = meta.hero_url;
        out.genre = meta.genre;
        out.developer = meta.developer;
        out.year = meta.year;
        out.screenshots = meta.screenshots;
        if out.cover_url.is_none() {
            out.cover_url = meta.cover_url;
        }
    }

    // 6. Repli visuel via le CDN Steam si appid connu.
    if let Some(appid) = &steam_app_id {
        let cdn = "https://cdn.cloudflare.steamstatic.com/steam/apps";
        if out.hero_url.is_none() {
            out.hero_url = Some(format!("{cdn}/{appid}/library_hero.jpg"));
        }
        if out.cover_url.is_none() {
            out.cover_url = Some(format!("{cdn}/{appid}/library_600x900.jpg"));
        }
    }

    Some(out)
}

// --- Wishlist Steam enrichie de prix (ITAD) -----------------------------------

const WISHLIST_WORKERS: usize = 8;

/// Un jeu de la wishlist Steam, avec son meilleur prix actuel et son plus bas historique.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct WishlistItem {
    pub app_id: u64,
    /// Identifiant ITAD (pour ouvrir la fiche produit) ; vide si non trouvé sur ITAD.
    pub game_id: String,
    pub title: String,
    /// Jaquette portrait Steam CDN (capsule officielle) quand l'appid est connu.
    /// ⚠️ Toutes les fiches Steam n'ont PAS de `library_600x900` : l'URL peut donner
    /// un 404, d'où la jaquette de repli ci-dessous.
    pub cover_url: String,
    /// Jaquette de repli (boxart ITAD), utilisée par le front si `cover_url` échoue.
    /// C'est ce qui manquait : sans elle, un jeu sans capsule Steam n'affichait qu'un
    /// dégradé dans la wishlist alors que sa fiche produit, elle, avait un visuel.
    pub cover_fallback_url: Option<String>,
    /// Meilleur prix actuel (EUR), `None` si aucune offre / non résolu.
    pub price: Option<f64>,
    pub normal_price: Option<f64>,
    pub savings: u32,
    pub store_name: String,
    pub buy_url: String,
    /// Plus bas prix historique (EUR), si connu.
    pub history_low: Option<f64>,
}

/// Résout un appid Steam en identifiant ITAD + titre + jaquette via `games/lookup/v1`
/// (réponse `{found, game:{id, title, assets:{boxart, …}}}`). La boxart est déjà dans
/// cette réponse : la récupérer ne coûte aucun appel supplémentaire.
fn lookup_appid(appid: u64) -> Option<(String, String, Option<String>)> {
    let root = get_json(&format!("games/lookup/v1?appid={appid}"))?;
    let g = root.get("game")?;
    Some((
        g["id"].as_str()?.to_string(),
        g["title"].as_str()?.to_string(),
        g["assets"]["boxart"].as_str().map(String::from),
    ))
}

/// Wishlist enrichie : pour chaque appid, résout l'id ITAD + le titre (en parallèle),
/// puis récupère prix courant (meilleure offre) + plus bas historique **en un seul appel
/// groupé**. Ordre d'entrée préservé (priorité de la wishlist). Jaquette = CDN Steam.
pub fn wishlist(appids: &[u64], excluded: &HashSet<String>) -> Vec<WishlistItem> {
    if appids.is_empty() {
        return Vec::new();
    }

    // 1) appid → (id ITAD, titre), réparti sur plusieurs threads.
    let next = AtomicUsize::new(0);
    let resolved: Vec<(u64, Option<(String, String, Option<String>)>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WISHLIST_WORKERS.min(appids.len().max(1)))
            .map(|_| {
                scope.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= appids.len() {
                            break;
                        }
                        local.push((appids[i], lookup_appid(appids[i])));
                    }
                    local
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    let mut by_appid: std::collections::HashMap<u64, (String, String, Option<String>)> = resolved
        .into_iter()
        .filter_map(|(a, r)| r.map(|v| (a, v)))
        .collect();

    // 2) Prix groupés pour tous les ids ITAD résolus (un seul POST).
    let ids: Vec<String> = appids
        .iter()
        .filter_map(|a| by_appid.get(a).map(|(id, ..)| id.clone()))
        .collect();
    let price_entries = if ids.is_empty() {
        Vec::new()
    } else {
        post_json(&format!("games/prices/v3?country={COUNTRY}"), &json!(ids))
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    };
    let prices: std::collections::HashMap<String, Value> = price_entries
        .into_iter()
        .filter_map(|e| {
            let id = e["id"].as_str()?.to_string();
            Some((id, e))
        })
        .collect();

    // 3) Items dans l'ordre de la wishlist.
    let cdn = "https://cdn.cloudflare.steamstatic.com/steam/apps";
    appids
        .iter()
        .map(|&appid| {
            let (game_id, title, boxart) = by_appid.remove(&appid).unwrap_or_default();
            let mut item = WishlistItem {
                app_id: appid,
                // Capsule officielle Steam d'abord (plus belle), boxart ITAD en repli
                // si elle n'existe pas — le front bascule à la première erreur de chargement.
                cover_url: format!("{cdn}/{appid}/library_600x900.jpg"),
                cover_fallback_url: boxart,
                title,
                game_id: game_id.clone(),
                ..Default::default()
            };
            if let Some(entry) = prices.get(&game_id) {
                item.history_low = num(&entry["historyLow"]["all"]["amount"]);
                if let Some(deal) = cheapest(entry, excluded) {
                    if let Some(p) = num(&deal["price"]["amount"]) {
                        item.price = Some(p);
                        item.normal_price = Some(num(&deal["regular"]["amount"]).unwrap_or(p));
                        item.savings = num(&deal["cut"]).unwrap_or(0.0).round() as u32;
                        item.store_name =
                            deal["shop"]["name"].as_str().unwrap_or_default().to_string();
                        item.buy_url = deal["url"].as_str().unwrap_or_default().to_string();
                    }
                }
            }
            item
        })
        .collect()
}

/// Résout l'appid Steam d'un jeu depuis son id ITAD (via `games/info/v2`). None si le
/// jeu n'est pas sur Steam. Permet de pousser vers la wishlist Steam même depuis une carte.
pub fn steam_appid_for(itad_id: &str) -> Option<u64> {
    let info = get_json(&format!("games/info/v2?id={}", urlencode(itad_id)))?;
    info["appid"].as_u64()
}

/// Enrichit des entrées de wishlist **Torii** (id ITAD + appid éventuel + titre + jaquette)
/// avec les prix ITAD (un seul POST). Utilisé pour les jeux ajoutés dans Torii (Steam ou non).
pub fn wishlist_custom(
    entries: &[(String, u64, String, Option<String>)],
    excluded: &HashSet<String>,
) -> Vec<WishlistItem> {
    if entries.is_empty() {
        return Vec::new();
    }
    let ids: Vec<String> = entries.iter().map(|(id, ..)| id.clone()).filter(|s| !s.is_empty()).collect();
    let price_entries = if ids.is_empty() {
        Vec::new()
    } else {
        post_json(&format!("games/prices/v3?country={COUNTRY}"), &json!(ids))
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    };
    let prices: std::collections::HashMap<String, Value> = price_entries
        .into_iter()
        .filter_map(|e| Some((e["id"].as_str()?.to_string(), e)))
        .collect();

    let cdn = "https://cdn.cloudflare.steamstatic.com/steam/apps";
    entries
        .iter()
        .map(|(id, appid, title, cover)| {
            // Jeu aussi présent sur Steam : capsule officielle d'abord, jaquette mémorisée
            // (boxart ITAD) en repli. Sinon la jaquette mémorisée suffit.
            let steam_capsule =
                (*appid > 0).then(|| format!("{cdn}/{appid}/library_600x900.jpg"));
            let (cover_url, cover_fallback_url) = match (steam_capsule, cover.clone()) {
                (Some(capsule), fallback) => (capsule, fallback),
                (None, stored) => (stored.unwrap_or_default(), None),
            };
            let mut item = WishlistItem {
                app_id: *appid,
                game_id: id.clone(),
                title: title.clone(),
                cover_url,
                cover_fallback_url,
                ..Default::default()
            };
            if let Some(entry) = prices.get(id) {
                item.history_low = num(&entry["historyLow"]["all"]["amount"]);
                if let Some(deal) = cheapest(entry, excluded) {
                    if let Some(p) = num(&deal["price"]["amount"]) {
                        item.price = Some(p);
                        item.normal_price = Some(num(&deal["regular"]["amount"]).unwrap_or(p));
                        item.savings = num(&deal["cut"]).unwrap_or(0.0).round() as u32;
                        item.store_name = deal["shop"]["name"].as_str().unwrap_or_default().to_string();
                        item.buy_url = deal["url"].as_str().unwrap_or_default().to_string();
                    }
                }
            }
            item
        })
        .collect()
}

/// Encodage minimal d'un paramètre de requête.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Objet deal calqué sur `/deals/v2` list[] (prix EUR natif, boutique + lien fournis).
    #[test]
    fn parse_deal_item() {
        let it: Value = serde_json::from_str(
            r#"{"id":"018d9…uuid","slug":"elden-ring","title":"Elden Ring",
                "assets":{"boxart":"https://x/box.jpg"},
                "deal":{"shop":{"id":61,"name":"Steam"},"price":{"amount":31.99,"currency":"EUR"},
                        "regular":{"amount":59.99},"cut":47,"url":"https://store.steampowered.com/app/1245620"}}"#,
        )
        .unwrap();
        let s = item_from_deal(&it).unwrap();
        assert_eq!(s.game_id, "018d9…uuid");
        assert_eq!(s.title, "Elden Ring");
        assert_eq!(s.cover_url.as_deref(), Some("https://x/box.jpg"));
        assert_eq!(s.price, 31.99);
        assert_eq!(s.normal_price, 59.99);
        assert_eq!(s.savings, 47);
        assert_eq!(s.store_name, "Steam");
        assert!(s.buy_url.contains("steampowered"));
    }

    #[test]
    fn num_handles_string_or_number() {
        assert_eq!(num(&json!(9.99)), Some(9.99));
        assert_eq!(num(&json!("9.99")), Some(9.99));
        assert_eq!(num(&json!(null)), None);
    }

    /// Entrée `games/prices/v3` à trois offres, de la moins chère à la plus chère.
    fn prices_entry() -> Value {
        serde_json::from_str(
            r#"{"id":"g1","deals":[
                {"shop":{"name":"JoyBuggy"},"price":{"amount":9.99},"regular":{"amount":39.99},"cut":75,"url":"https://joy/1"},
                {"shop":{"name":"Fanatical"},"price":{"amount":12.49},"regular":{"amount":39.99},"cut":69,"url":"https://fana/1"},
                {"shop":{"name":"Steam"},"price":{"amount":19.99},"regular":{"amount":39.99},"cut":50,"url":"https://steam/1"}]}"#,
        )
        .unwrap()
    }

    /// Le revendeur masqué ne doit JAMAIS être retenu comme meilleure offre — c'est le
    /// choix fait ici qui alimente la vitrine, la recherche et la wishlist (un seul prix
    /// par jeu côté front, donc rien à re-filtrer là-bas).
    #[test]
    fn cheapest_skips_excluded_shops() {
        let entry = prices_entry();
        let none = HashSet::new();
        assert_eq!(shop_of(cheapest(&entry, &none).unwrap()), "JoyBuggy");

        let excluded: HashSet<String> = ["JoyBuggy".to_string()].into_iter().collect();
        assert_eq!(shop_of(cheapest(&entry, &excluded).unwrap()), "Fanatical");

        let excluded: HashSet<String> = ["JoyBuggy", "Fanatical"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let best = cheapest(&entry, &excluded).unwrap();
        assert_eq!(shop_of(best), "Steam");
        assert_eq!(num(&best["price"]["amount"]), Some(19.99));

        // Tout masqué : aucune offre retenue (le jeu sort de la grille).
        let excluded: HashSet<String> = ["JoyBuggy", "Fanatical", "Steam"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(cheapest(&entry, &excluded).is_none());
    }

    /// `apply_deal` remplace bien prix, remise, boutique et lien d'achat de l'item.
    #[test]
    fn apply_deal_rewrites_item() {
        let entry = prices_entry();
        let excluded: HashSet<String> = ["JoyBuggy".to_string()].into_iter().collect();
        let mut item = StoreItem {
            store_name: "JoyBuggy".into(),
            price: 9.99,
            normal_price: 39.99,
            savings: 75,
            buy_url: "https://joy/1".into(),
            ..Default::default()
        };
        assert!(apply_deal(&mut item, cheapest(&entry, &excluded).unwrap()));
        assert_eq!(item.store_name, "Fanatical");
        assert_eq!(item.price, 12.49);
        assert_eq!(item.savings, 69);
        assert_eq!(item.buy_url, "https://fana/1");
    }

    #[test]
    fn sort_mapping() {
        assert_eq!(sort_param("savings"), "-cut");
        assert_eq!(sort_param("price"), "price");
        assert_eq!(sort_param("autre"), "-trending");
    }
}
