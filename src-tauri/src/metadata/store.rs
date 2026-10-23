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
//! Usages, tous à la demande :
//! - `deals(page, sort)` : la vitrine (`GET /deals/v2`), grille « Toutes les offres » et
//!   carrousel « Promos du moment ».
//! - `giveaways()` : les jeux offerts (même endpoint, trié par prix).
//! - `search(query)` : recherche (`GET /games/search/v1` + `POST /games/prices/v3`).
//! - `game(id)` : fiche produit (`info` + `prices` + `overview`) + IGDB + Instant Gaming.
//!
//! Les rayons de l'accueil (vedette, meilleures ventes, genres…) sont dans `vitrine.rs` :
//! Steam y choisit les jeux, ce module-ci leur donne leur prix.

use crate::models::GameDto;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

const PROXY: &str = "https://torii-igdb-proxy.toriiapp.workers.dev/itad";
/// Chemin de l'appel « prix », pour la région couramment choisie.
///
/// ⚠️ C'ÉTAIT UNE CONSTANTE (`const COUNTRY: &str = "FR"`). Ça ne pouvait plus tenir : la
/// région est désormais un réglage, et elle peut changer pendant que l'application
/// tourne. Une fonction relit donc la valeur à chaque appel plutôt que de la figer à la
/// compilation.
///
/// 🔑 C'est le pays qui détermine la **devise** renvoyée par l'API — l'interface ne la
/// suppose plus, elle lit celle qui accompagne chaque prix (voir `lib/format.ts`).
pub(super) fn chemin_prix() -> String {
    format!("games/prices/v3?country={}", crate::locale::region())
}
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
    /// Prix actuel le plus bas, dans la devise `currency`.
    pub price: f64,
    /// Prix normal (hors promo) ; == price si inconnu.
    pub normal_price: f64,
    /// Devise des montants ci-dessus (`EUR`, `USD`…), telle que renvoyée avec eux.
    ///
    /// 🔑 Lue dans la réponse, jamais supposée : c'est le pays interrogé qui la décide
    /// (`locale::region`), et l'interface ne formate plus un prix sans elle — un montant
    /// américain affiché « 59,99 € » serait un chiffre faux, pas une approximation.
    /// Vide si la source n'en a pas donné ; l'interface retombe alors sur la devise
    /// attendue pour la région.
    pub currency: String,
    /// Remise en % entier (0 = pas de promo / inconnu).
    pub savings: u32,
    /// Boutique de la meilleure offre (vide si non résolu).
    pub store_name: String,
    /// Lien d'achat direct.
    pub buy_url: String,
    /// Bannière paysage (ITAD `banner600`, 600×344) : carrousels de l'accueil.
    pub banner_url: Option<String>,
    /// L'offre touche le plus bas historique du jeu (drapeau ITAD `H`, ou `N` pour un
    /// nouveau record). Le drapeau `S` (plus bas de CETTE boutique) n'est pas retenu :
    /// annoncer un record qui n'en est un que chez un vendeur serait trompeur.
    pub record: bool,
    /// Fin annoncée de l'offre (ISO 8601), si le vendeur la donne.
    pub expiry: Option<String>,
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
    /// Devise des montants ci-dessus (`EUR`, `USD`…), telle que renvoyée avec eux.
    ///
    /// 🔑 Lue dans la réponse, jamais supposée : c'est le pays interrogé qui la décide
    /// (`locale::region`), et l'interface ne formate plus un prix sans elle — un montant
    /// américain affiché « 59,99 € » serait un chiffre faux, pas une approximation.
    /// Vide si la source n'en a pas donné ; l'interface retombe alors sur la devise
    /// attendue pour la région.
    pub currency: String,
    pub savings: u32,
    pub buy_url: String,
    /// `false` si l'offre est en rupture de stock (Instant Gaming). Les offres ITAD
    /// sont toujours considérées disponibles (elles proviennent de deals en cours).
    pub available: bool,
    /// Plateformes d'activation de l'offre selon ITAD (`Steam`, `GOG`, `Drm Free`…).
    pub drm: Vec<String>,
    /// Fin annoncée de l'offre (ISO 8601).
    pub expiry: Option<String>,
    /// Code promo à saisir chez le vendeur pour obtenir ce prix.
    pub voucher: Option<String>,
}

/// Une note critique ou communautaire (ITAD `games/info/v2` → `reviews`).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreReview {
    /// `Steam`, `Metascore` ou `OpenCritic`.
    pub source: String,
    pub score: u32,
    pub count: u64,
    pub url: Option<String>,
}

/// Fiche produit : comparatif de prix + métadonnée descriptive (IGDB, Steam).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoreGame {
    pub game_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    /// Appid Steam, si le jeu y est vendu.
    pub app_id: Option<u64>,
    /// Date de sortie (`AAAA-MM-JJ`) selon ITAD.
    pub release_date: Option<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub reviews: Vec<StoreReview>,
    /// Joueurs Steam sur 24 h et record historique (ITAD `players`).
    pub players_day: Option<u64>,
    pub players_peak: Option<u64>,
    /// Personnes qui suivent ce jeu sur le comparateur.
    pub waitlisted: Option<u64>,
    /// Plus bas des 3 derniers mois et de l'année (même devise que `cheapest_ever`).
    pub low_3m: Option<f64>,
    pub low_1y: Option<f64>,
    /// Ce que Steam sait du jeu (captures, langues, configuration…), s'il y est vendu.
    pub steam: Option<super::fiche::FicheSteam>,
    /// Prix le plus bas jamais atteint, si connu.
    pub cheapest_ever: Option<f64>,
    /// Devise de `cheapest_ever` (les offres portent chacune la leur).
    pub currency: String,
    /// Offres par boutique, triées par prix croissant.
    pub prices: Vec<StorePrice>,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub developer: Option<String>,
    pub year: Option<i64>,
    pub screenshots: Vec<String>,
}

// --- Accès HTTP au proxy ITAD -------------------------------------------------

/// 🔑 **PAUSE APRÈS UN REFUS DU COMPARATEUR.** La clé ITAD est UNE pour tous les
/// utilisateurs de Torii (relayée par le proxy) : quand ITAD répond 429, chaque appel de
/// plus prolonge la saturation pour tout le monde. Mesuré le 22 octobre 2026 : deux
/// saturations de plusieurs minutes en une soirée d'essais de la nouvelle Boutique, et
/// pendant ce temps l'application continuait d'appeler (rayons, fiches, wishlist).
/// Après un 429, plus aucun appel au comparateur pendant `Retry-After` (60 s par défaut,
/// borné entre 10 s et 5 min) : les fonctions répondent « rien » sans réseau, comme pour
/// une panne, et l'interface affiche « réessaie dans une minute ».
static PAUSE_JUSQUA: AtomicU64 = AtomicU64::new(0);
const PAUSE_DEFAUT: u64 = 60;

fn secondes() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Le comparateur est-il en pause (refus récent) ?
pub(super) fn en_pause() -> bool {
    secondes() < PAUSE_JUSQUA.load(Ordering::Relaxed)
}

/// Retient un refus « trop de requêtes » : plus d'appel avant la fin de la pause.
fn noter_refus(e: &ureq::Error) {
    if let ureq::Error::Status(429, reponse) = e {
        let duree = reponse
            .header("retry-after")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(PAUSE_DEFAUT)
            .clamp(10, 300);
        PAUSE_JUSQUA.store(secondes() + duree, Ordering::Relaxed);
        eprintln!("[boutique] comparateur saturé (429) : pause de {duree} s");
    }
}

pub(super) fn get_json(path: &str) -> Option<Value> {
    if en_pause() {
        return None;
    }
    match ureq::get(&format!("{PROXY}/{path}")).timeout(Duration::from_secs(15)).call() {
        Ok(r) => r.into_json().ok(),
        Err(e) => {
            noter_refus(&e);
            None
        }
    }
}

pub(super) fn post_json(path: &str, body: &Value) -> Option<Value> {
    if en_pause() {
        return None;
    }
    match ureq::post(&format!("{PROXY}/{path}"))
        .timeout(Duration::from_secs(15))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(r) => r.into_json().ok(),
        Err(e) => {
            noter_refus(&e);
            None
        }
    }
}

/// Lit un nombre qu'il soit encodé en JSON number ou en chaîne.
pub(super) fn num(v: &Value) -> Option<f64> {
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
pub(super) fn cheapest<'a>(entry: &'a Value, excluded: &HashSet<String>) -> Option<&'a Value> {
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

/// La devise d'un montant ITAD (`{ amount, amountInt, currency }`), vide si absente.
pub(super) fn devise(montant: &Value) -> String {
    montant["currency"].as_str().unwrap_or_default().to_string()
}

/// Drapeau d'une offre ITAD : `H` = au plus bas historique, `N` = nouveau plus bas.
///
/// ⚠️ Seulement s'il y a une remise. Un jeu qui vient de sortir est « au plus bas
/// historique » à plein tarif — il n'a jamais coûté moins cher — et ITAD le marque `H` :
/// mesuré sur Call of Duty à 69,99 €, -0 %. Afficher « plus bas historique » sur un prix
/// de lancement serait vrai et parfaitement trompeur.
pub(super) fn est_record(deal: &Value) -> bool {
    matches!(deal["flag"].as_str(), Some("H" | "N")) && num(&deal["cut"]).unwrap_or(0.0) > 0.0
}

/// Fin annoncée d'une offre ITAD, si le vendeur la donne.
pub(super) fn fin_offre(deal: &Value) -> Option<String> {
    deal["expiry"].as_str().filter(|s| !s.is_empty()).map(String::from)
}

/// Applique une offre à un item de grille (vitrine / recherche).
fn apply_deal(item: &mut StoreItem, deal: &Value) -> bool {
    let Some(price) = num(&deal["price"]["amount"]) else {
        return false;
    };
    item.price = price;
    item.normal_price = num(&deal["regular"]["amount"]).unwrap_or(price);
    item.currency = devise(&deal["price"]);
    item.savings = num(&deal["cut"]).unwrap_or(0.0).round() as u32;
    item.store_name = shop_of(deal).to_string();
    item.buy_url = deal["url"].as_str().unwrap_or_default().to_string();
    item.record = est_record(deal);
    item.expiry = fin_offre(deal);
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
        currency: devise(&deal["price"]),
        savings: num(&deal["cut"]).unwrap_or(0.0).round() as u32,
        store_name: deal["shop"]["name"].as_str().unwrap_or_default().to_string(),
        buy_url: deal["url"].as_str().unwrap_or_default().to_string(),
        banner_url: it["assets"]["banner600"].as_str().map(String::from),
        record: est_record(deal),
        expiry: fin_offre(deal),
    })
}

/// Vitrine : une page de jeux mis en avant / en promo, selon le tri choisi. Prix EUR.
/// Les offres des revendeurs masqués sont remplacées par la meilleure offre autorisée
/// du même jeu (cf. `reprice_excluded`).
pub fn deals(page: u32, sort: &str, excluded: &HashSet<String>) -> Vec<StoreItem> {
    let offset = page * PAGE_SIZE;
    let path = format!(
        "deals/v2?country={}&offset={offset}&limit={PAGE_SIZE}&sort={}",
        crate::locale::region(),
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

/// Jeux offerts en ce moment : la vitrine triée par prix croissant commence par les
/// offres à 0 €. On ne garde que les **jeux** (`type: "game"`) : la même tête de liste
/// contient des DLC, des bandes-son et des démos à 0 € (`type` `dlc` ou nul), qui ne
/// sont pas des cadeaux au sens où l'entend un joueur. Un vendeur masqué n'est pas
/// re-tarifé ici : un jeu offert ailleurs que chez lui serait de toute façon dans la
/// liste sous l'autre vendeur.
pub fn giveaways(excluded: &HashSet<String>) -> Vec<StoreItem> {
    let path = format!(
        "deals/v2?country={}&offset=0&limit=60&sort=price",
        crate::locale::region()
    );
    let list = get_json(&path)
        .and_then(|root| root.get("list").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    lire_cadeaux(&list, excluded)
}

fn lire_cadeaux(list: &[Value], excluded: &HashSet<String>) -> Vec<StoreItem> {
    list.iter()
        .filter(|it| it["type"].as_str() == Some("game"))
        .filter_map(item_from_deal)
        .filter(|it| it.price == 0.0 && it.normal_price > 0.0 && !excluded.contains(&it.store_name))
        .take(8)
        .collect()
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
        post_json(&chemin_prix(), &json!(ids))
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
    let prices = post_json(&chemin_prix(), &json!(ids))
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
                currency: devise(&best["price"]),
                savings: num(&best["cut"]).unwrap_or(0.0).round() as u32,
                store_name: best["shop"]["name"].as_str().unwrap_or_default().to_string(),
                buy_url: best["url"].as_str().unwrap_or_default().to_string(),
                record: est_record(best),
                expiry: fin_offre(best),
                ..Default::default()
            })
        })
        .collect()
}

/// Fiche produit : comparatif multi-boutiques (ITAD) + plus bas historique + IGDB + IG
/// + fiche Steam (captures, langues, configuration, fonctionnalités…).
///
/// 🔑 Une fois le titre et l'appid connus (`games/info/v2`), tout le reste part EN
/// PARALLÈLE : prix, Instant Gaming, IGDB, description traduite, fiche Steam. En série,
/// la fiche additionnait les allers-retours ; elle n'attend plus que le plus lent.
pub fn game(game_id: &str, config_dir: &Path) -> Option<StoreGame> {
    // 1. Infos jeu (titre, appid Steam, jaquette, notes, joueurs, sortie, studios).
    let info = get_json(&format!("games/info/v2?id={}", urlencode(game_id)))?;
    let title = info["title"].as_str()?.to_string();
    let app_id = info["appid"].as_u64();
    let steam_app_id = app_id.map(|a| a.to_string());
    let boxart = info["assets"]["boxart"].as_str().map(String::from);

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

    // 2. Tout le reste, en parallèle.
    let (entry, ig, igdb, description_traduite, steam) = std::thread::scope(|s| {
        // Prix par boutique (comparatif) + plus bas historiques, en un seul appel.
        let prix = s.spawn(|| {
            post_json(&chemin_prix(), &json!([game_id]))
                .and_then(|v| v.as_array().and_then(|a| a.first()).cloned())
        });
        // Instant Gaming (prix EUR natif, absent d'ITAD) — best-effort scrape.
        //
        // ⚠️ ZONE EURO SEULEMENT. Ce revendeur n'affiche que des euros, sans équivalent
        // local. Hors zone euro, sa ligne se retrouverait au milieu de prix en livres ou en
        // dollars sans qu'on puisse la comparer à rien — alors que comparer est exactement
        // ce qu'on vient faire ici. L'interface explique l'absence (`prix.revendeurIndisponible`)
        // plutôt que de laisser croire à une panne.
        let ig = s.spawn(|| {
            crate::locale::zone_euro()
                .then(|| super::instant_gaming::price(&title, app_id))
                .flatten()
        });
        // Enrichissement descriptif via IGDB (cache disque partagé).
        let igdb = s.spawn(|| {
            super::igdb::fill_metadata(std::slice::from_ref(&dto), config_dir, |_| {})
                .into_iter()
                .next()
                .map(|(_, meta)| meta)
        });
        // Description dans la langue de l'interface quand le jeu est identifié sur Steam.
        //
        // 🔑 IGDB ne localise rien : sa description est en anglais, toujours. Pour un jeu
        // portant un appid, `enrich_one` sait aller chercher la version française — et il
        // **partage le cache disque de la bibliothèque**, donc un jeu déjà possédé ne coûte
        // aucune requête, et consulter une fiche de la vitrine réchauffe le cache pour le
        // jour où on l'achètera.
        //
        // Le **genre** n'est volontairement PAS remplacé : celui d'IGDB est déjà traduit
        // (`igdb::genre_fr`) et c'est lui qui sert de clé au filtre par catégorie de la
        // bibliothèque. Prendre celui de Steam ici ferait dire deux choses différentes au
        // même jeu selon l'écran.
        let traduite = s.spawn(|| {
            let appid = steam_app_id.as_ref()?;
            let meta = super::enrich_one(
                &GameDto {
                    id: format!("steam:{appid}"),
                    platform: "steam".into(),
                    launch_target: appid.clone(),
                    title: title.clone(),
                    ..Default::default()
                },
                config_dir,
            );
            meta.localized.then_some(meta.description).flatten()
        });
        // Fiche Steam : captures, aperçu vidéo, langues, configuration… (sans clé).
        let steam = s.spawn(|| app_id.and_then(|a| super::fiche::fiche(a, config_dir)));
        (
            prix.join().ok().flatten(),
            ig.join().ok().flatten(),
            igdb.join().ok().flatten(),
            traduite.join().ok().flatten(),
            steam.join().ok().flatten(),
        )
    });

    let mut prices: Vec<StorePrice> = entry
        .as_ref()
        .and_then(|e| e["deals"].as_array())
        .map(|deals| deals.iter().filter_map(offre_fiche).collect())
        .unwrap_or_default();

    if let Some(ig) = ig {
        prices.push(StorePrice {
            store_name: "Instant Gaming".into(),
            price: ig.price,
            retail_price: if ig.savings > 0 {
                (ig.price / (1.0 - ig.savings as f64 / 100.0) * 100.0).round() / 100.0
            } else {
                ig.price
            },
            // Instant Gaming ne vend qu'en euros — et n'est proposé qu'en zone euro.
            currency: "EUR".into(),
            savings: ig.savings,
            buy_url: ig.url,
            available: ig.available,
            drm: Vec::new(),
            expiry: None,
            voucher: None,
        });
    }
    prices.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Plus bas historiques (tous temps, 1 an, 3 mois), fournis par la réponse prices.
    let bas = |periode: &str| entry.as_ref().and_then(|e| num(&e["historyLow"][periode]["amount"]));
    let currency = entry
        .as_ref()
        .map(|e| devise(&e["historyLow"]["all"]))
        .unwrap_or_default();

    let mut out = StoreGame {
        game_id: game_id.to_string(),
        title: title.clone(),
        cover_url: boxart,
        hero_url: None,
        app_id,
        release_date: info["releaseDate"].as_str().map(String::from),
        developers: noms(&info["developers"]),
        publishers: noms(&info["publishers"]),
        reviews: notes(&info["reviews"]),
        players_day: info["players"]["day"].as_u64().filter(|n| *n > 0),
        players_peak: info["players"]["peak"].as_u64().filter(|n| *n > 0),
        waitlisted: info["stats"]["waitlisted"].as_u64().filter(|n| *n > 0),
        cheapest_ever: bas("all"),
        low_1y: bas("y1"),
        low_3m: bas("m3"),
        currency,
        prices,
        steam,
        ..Default::default()
    };

    // 4. Descriptif IGDB, puis description traduite si Steam l'a.
    if let Some(meta) = igdb {
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
    if description_traduite.is_some() {
        out.description = description_traduite;
    }

    // 5. Repli visuel via le CDN Steam si appid connu.
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

/// Une ligne du comparatif de la fiche, depuis une offre `games/prices/v3`.
fn offre_fiche(d: &Value) -> Option<StorePrice> {
    let price = num(&d["price"]["amount"])?;
    Some(StorePrice {
        store_name: d["shop"]["name"].as_str().unwrap_or_default().to_string(),
        price,
        retail_price: num(&d["regular"]["amount"]).unwrap_or(price),
        currency: devise(&d["price"]),
        savings: num(&d["cut"]).unwrap_or(0.0).round() as u32,
        buy_url: d["url"].as_str().unwrap_or_default().to_string(),
        available: true, // offres ITAD = deals en cours
        drm: noms(&d["drm"]),
        expiry: fin_offre(d),
        voucher: d["voucher"].as_str().filter(|v| !v.is_empty()).map(String::from),
    })
}

/// Les `name` d'une liste d'objets ITAD (`developers`, `publishers`, `drm`).
fn noms(liste: &Value) -> Vec<String> {
    liste
        .as_array()
        .map(|l| l.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Notes retenues pour la fiche : Steam, Metacritic (critiques) et OpenCritic. La note
/// des joueurs de Metacritic est écartée : elle double celle de Steam, en moins fiable.
fn notes(reviews: &Value) -> Vec<StoreReview> {
    reviews
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|r| {
                    let source = r["source"].as_str()?;
                    matches!(source, "Steam" | "Metascore" | "OpenCritic").then(|| StoreReview {
                        source: source.to_string(),
                        score: num(&r["score"]).unwrap_or(0.0).round() as u32,
                        count: r["count"].as_u64().unwrap_or(0),
                        url: r["url"].as_str().filter(|u| u.starts_with("https://")).map(String::from),
                    })
                })
                .filter(|r| r.score > 0)
                .collect()
        })
        .unwrap_or_default()
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
    /// Meilleur prix actuel, `None` si aucune offre / non résolu.
    pub price: Option<f64>,
    /// Devise des montants ci-dessus (`EUR`, `USD`…), telle que renvoyée avec eux.
    ///
    /// 🔑 Lue dans la réponse, jamais supposée : c'est le pays interrogé qui la décide
    /// (`locale::region`), et l'interface ne formate plus un prix sans elle — un montant
    /// américain affiché « 59,99 € » serait un chiffre faux, pas une approximation.
    /// Vide si la source n'en a pas donné ; l'interface retombe alors sur la devise
    /// attendue pour la région.
    pub currency: String,
    pub normal_price: Option<f64>,
    pub savings: u32,
    pub store_name: String,
    pub buy_url: String,
    /// Plus bas prix historique, si connu (même devise que `price`).
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
        post_json(&chemin_prix(), &json!(ids))
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
                item.currency = devise(&entry["historyLow"]["all"]);
                if let Some(deal) = cheapest(entry, excluded) {
                    if let Some(p) = num(&deal["price"]["amount"]) {
                        item.price = Some(p);
                        item.currency = devise(&deal["price"]);
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
        post_json(&chemin_prix(), &json!(ids))
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
                item.currency = devise(&entry["historyLow"]["all"]);
                if let Some(deal) = cheapest(entry, excluded) {
                    if let Some(p) = num(&deal["price"]["amount"]) {
                        item.price = Some(p);
                        item.currency = devise(&deal["price"]);
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

    /// Tête réelle de `deals/v2?sort=price` (22/10/2026, raccourcie) : seuls les JEUX à
    /// 0 € sont des cadeaux — pas la démo (`type` nul) ni le pack de skins (`dlc`).
    #[test]
    fn cadeaux_seulement_les_jeux_a_zero() {
        let list: Vec<Value> = serde_json::from_str(
            r#"[
              {"id":"a","title":"Sker Ritual","type":"game","assets":{"boxart":"https://x/a.jpg","banner600":"https://x/a6.jpg"},
               "deal":{"shop":{"name":"Epic Game Store"},"price":{"amount":0,"currency":"EUR"},"regular":{"amount":19.99},"cut":100,"flag":"N","expiry":"2026-10-29T16:00:00+01:00","url":"https://epic/a"}},
              {"id":"b","title":"Stellar End Demo Version","type":null,
               "deal":{"shop":{"name":"Humble Store"},"price":{"amount":0},"regular":{"amount":4.99},"cut":100,"url":"https://h/b"}},
              {"id":"c","title":"Squad Weapon Skins","type":"dlc",
               "deal":{"shop":{"name":"Steam"},"price":{"amount":0},"regular":{"amount":2.99},"cut":100,"url":"https://s/c"}},
              {"id":"d","title":"Pongo","type":"game",
               "deal":{"shop":{"name":"Dreamgame"},"price":{"amount":0.06},"regular":{"amount":0.79},"cut":92,"url":"https://d/d"}},
              {"id":"e","title":"Terraspace","type":"game",
               "deal":{"shop":{"name":"GOG"},"price":{"amount":0},"regular":{"amount":14.99},"cut":100,"url":"https://gog/e"}}
            ]"#,
        )
        .unwrap();
        let tous = lire_cadeaux(&list, &HashSet::new());
        assert_eq!(tous.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), vec!["Sker Ritual", "Terraspace"]);
        assert_eq!(tous[0].banner_url.as_deref(), Some("https://x/a6.jpg"));
        assert_eq!(tous[0].expiry.as_deref(), Some("2026-10-29T16:00:00+01:00"));
        assert!(tous[0].record);

        let sans_gog: HashSet<String> = ["GOG".to_string()].into_iter().collect();
        assert_eq!(lire_cadeaux(&list, &sans_gog).len(), 1);
    }

    /// Un 429 met le comparateur en pause : plus aucun appel ne part avant la fin.
    /// (Une autre erreur, elle, n'en déclenche pas : une panne réseau n'est pas un refus.)
    #[test]
    fn un_refus_met_le_comparateur_en_pause() {
        PAUSE_JUSQUA.store(0, Ordering::Relaxed);
        noter_refus(&ureq::Error::Status(500, ureq::Response::new(500, "Erreur", "").unwrap()));
        assert!(!en_pause());
        noter_refus(&ureq::Error::Status(429, ureq::Response::new(429, "Too Many Requests", "").unwrap()));
        assert!(en_pause());
        let reste = PAUSE_JUSQUA.load(Ordering::Relaxed) - secondes();
        assert!((59..=60).contains(&reste), "pause par défaut de 60 s, reste {reste}");
        assert_eq!(get_json("deals/v2"), None, "aucun appel pendant la pause");
        PAUSE_JUSQUA.store(0, Ordering::Relaxed);
    }

    #[test]
    fn sort_mapping() {
        assert_eq!(sort_param("savings"), "-cut");
        assert_eq!(sort_param("price"), "price");
        assert_eq!(sort_param("autre"), "-trending");
    }
}
