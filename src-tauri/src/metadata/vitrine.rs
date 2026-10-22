//! Accueil de la Boutique : les rayons « façon Steam » (vedette, meilleures ventes,
//! nouveautés, jeux attendus, genres, free-to-play).
//!
//! 🔑 **DEUX SOURCES, DEUX RÔLES : STEAM CHOISIT LES JEUX, ITAD DONNE LE PRIX.**
//! ITAD sait comparer les prix de 60 boutiques, mais il ne sait pas ce qui se vend, ce
//! qui vient de sortir ou ce que les joueurs attendent : sa vitrine seule donnait une
//! liste de promos sans relief. Steam, lui, classe les jeux pays par pays — mais ne
//! connaît que son propre prix. Chaque rayon prend donc sa liste chez Steam, puis
//! chaque jeu reçoit la meilleure offre toutes boutiques (revendeurs masqués exclus,
//! via `store::cheapest`, comme partout ailleurs).
//!
//! Endpoints Steam, tous **sans clé** (relevés le 22 octobre 2026) :
//! - `IStoreQueryService/Query` : recherche filtrée et triée, avec visuels, tags, avis
//!   et description dans la langue demandée. C'est l'API qu'utilise la boutique Steam.
//! - `IStoreBrowseService/GetItems` : les mêmes données pour une liste d'appids.
//! - `store.steampowered.com/tagdata/populartags/<langue>` : le nom des tags.
//!
//! ⚠️ **`IStoreTopSellersService/GetWeeklyTopSellers` EST MONDIAL** : il renvoie la même
//! liste pour la France, les États-Unis et le Japon, quel que soit `country_code`, et
//! elle est figée pour la semaine. `Query` trié par ventes (`sort: 10`), lui, dépend du
//! pays (le Japon n'a presque rien en commun avec la France) — c'est lui qu'on prend.
//!
//! ⚠️ `sort` est une énumération que Steam ne documente pas. Relevé empirique : `10` =
//! ventes du moment dans le pays, `13` = popularité, qui classe mieux les jeux à venir
//! (les grosses sorties attendues d'abord, là où `10` remonte des précommandes de
//! niche). Les autres valeurs donnent des listes sans intérêt ici.
//!
//! ⚠️ `cc=` ne suit pas la région ailleurs dans Torii (`steam_store::CATALOGUE_STEAM`),
//! parce que Steam masque des jeux selon le pays. Ici c'est précisément ce qu'on veut :
//! les rayons montrent ce qui se vend là où l'utilisateur achète.

use super::store::{self, StoreItem};
use crate::locale;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const QUERY: &str = "https://api.steampowered.com/IStoreQueryService/Query/v1";
const GET_ITEMS: &str = "https://api.steampowered.com/IStoreBrowseService/GetItems/v1";
const TAGS: &str = "https://store.steampowered.com/tagdata/populartags";

/// Nombre de jeux renvoyés par rayon.
const PAR_RAYON: usize = 12;
/// Une « nouveauté » est sortie depuis moins de 75 jours.
const FENETRE_NOUVEAUTES: i64 = 75 * 86_400;
/// Seuil d'avis positifs pour la vedette : on ne met pas en avant un jeu mal reçu.
const AVIS_VEDETTE: u32 = 75;
/// Remise minimale de la meilleure offre pour figurer en vedette.
const REMISE_VEDETTE: u32 = 20;

/// Descripteurs de contenu Steam « contenu sexuel » (3) et « nudité » (4) : la boutique
/// Steam les masque par défaut, on fait pareil.
const ADULTE: [u64; 2] = [3, 4];
/// Tags jamais affichés sur une carte. Nudité, contenu sexuel, gore, violent : ils
/// décrivent un contenu, pas un jeu. Free-to-play : la carte dit déjà « Gratuit ».
const TAGS_MASQUES: [u64; 5] = [6650, 12095, 4345, 4667, 113];

/// Identifiant de Steam chez ITAD (`shop.id` dans toutes ses réponses).
const ITAD_STEAM: u32 = 61;
const ITAD_WORKERS: usize = 8;

// --- Ce que reçoit l'interface -------------------------------------------------

/// Un jeu d'un rayon : la fiche Steam (visuels, tags, avis, sortie) et la meilleure
/// offre toutes boutiques.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub app_id: u64,
    /// Identifiant ITAD (ouvre la fiche produit) ; vide si ITAD ne connaît pas le jeu,
    /// l'interface ouvre alors la page Steam.
    pub game_id: String,
    pub title: String,
    /// Jaquette portrait (600×900).
    pub capsule_url: Option<String>,
    /// Bandeau paysage (460×215).
    pub header_url: Option<String>,
    /// Grande bannière (1920×620) : la vedette.
    pub hero_url: Option<String>,
    /// Description courte, dans la langue de l'interface.
    pub description: Option<String>,
    /// Trois tags au plus, dans la langue de l'interface.
    pub tags: Vec<String>,
    /// Appréciation Steam de 1 (extrêmement négatives) à 9 (extrêmement positives),
    /// 0 = pas encore d'avis. Le libellé est traduit côté interface.
    pub review_score: u32,
    pub review_percent: Option<u32>,
    pub review_count: u32,
    /// Date de sortie (Unix, secondes), si Steam en donne une.
    pub release_date: Option<i64>,
    /// Précision de la date : `date_full`, `date_month`, `date_quarter`, `date_year`,
    /// ou `text_comingsoon` (pas de date, voir `release_text`).
    pub release_display: String,
    /// Texte libre de l'éditeur (« Prochainement »), déjà dans la langue demandée.
    pub release_text: Option<String>,
    pub coming_soon: bool,
    pub is_free: bool,
    /// Meilleure offre actuelle (ITAD), dans la devise `currency` ; absente si aucune.
    pub price: Option<f64>,
    pub normal_price: Option<f64>,
    pub currency: String,
    pub savings: u32,
    pub store_name: String,
    pub buy_url: String,
    pub history_low: Option<f64>,
    /// L'offre touche le plus bas historique (voir `StoreItem::record`).
    pub record: bool,
    pub expiry: Option<String>,
}

/// Ce que l'accueil affiche d'emblée. Les autres rayons (onglets, genres, free-to-play)
/// sont demandés quand l'utilisateur y arrive (`shelf`).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoreHome {
    pub featured: Vec<ShelfItem>,
    pub top_sellers: Vec<ShelfItem>,
    /// La vitrine ITAD (tendances) : « Promos du moment ».
    pub deals: Vec<StoreItem>,
    pub giveaways: Vec<StoreItem>,
}

/// Un rayon demandé par l'interface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rayon {
    Vedette,
    Ventes,
    Nouveautes,
    AVenir,
    Genre(u32),
    Gratuits,
}

impl Rayon {
    /// Nom employé par l'interface (`StoreShelfKind` dans `types.ts`).
    pub fn depuis(nom: &str, tag: Option<u32>) -> Option<Rayon> {
        Some(match nom {
            "featured" => Rayon::Vedette,
            "topSellers" => Rayon::Ventes,
            "newReleases" => Rayon::Nouveautes,
            "upcoming" => Rayon::AVenir,
            "genre" => Rayon::Genre(tag?),
            "freeToPlay" => Rayon::Gratuits,
            _ => return None,
        })
    }
}

// --- Points d'entrée -----------------------------------------------------------

/// L'accueil : vedette, meilleures ventes, promos et jeux offerts, demandés en parallèle.
pub fn home(dir: &Path, excluded: &HashSet<String>) -> StoreHome {
    let (vedette, ventes, deals, giveaways, noms) = std::thread::scope(|s| {
        let vedette = s.spawn(|| brut(Rayon::Vedette));
        let ventes = s.spawn(|| brut(Rayon::Ventes));
        let deals = s.spawn(|| store::deals(0, "featured", excluded));
        let giveaways = s.spawn(|| store::giveaways(excluded));
        let noms = noms_tags(dir);
        (
            vedette.join().ok().flatten().unwrap_or_default(),
            ventes.join().ok().flatten().unwrap_or_default(),
            deals.join().unwrap_or_default(),
            giveaways.join().unwrap_or_default(),
            noms,
        )
    });

    let featured = lire_rayon(Rayon::Vedette, &vedette, &noms);
    let nb = featured.len();
    let mut tous = featured;
    tous.extend(lire_rayon(Rayon::Ventes, &ventes, &noms));
    completer(&mut tous, dir, excluded);
    let top_sellers = tous.split_off(nb);
    let featured = garder_vraies_promos(tous);
    StoreHome { featured, top_sellers, deals, giveaways }
}

/// La vedette promet une promo : la remise de Steam ne suffit pas, il faut que la
/// meilleure offre toutes boutiques en soit une. Mesuré : Disney Dreamlight Valley, en
/// promo sur Steam, ressortait à -4 % chez un autre vendeur — un « En promo » à -4 %.
fn garder_vraies_promos(jeux: Vec<ShelfItem>) -> Vec<ShelfItem> {
    jeux.into_iter().filter(|j| j.price.is_some() && j.savings >= REMISE_VEDETTE).collect()
}

/// Un rayon seul (onglet, genre, free-to-play), prix compris.
pub fn shelf(rayon: Rayon, dir: &Path, excluded: &HashSet<String>) -> Vec<ShelfItem> {
    let items = brut(rayon).unwrap_or_default();
    let mut jeux = lire_rayon(rayon, &items, &noms_tags(dir));
    completer(&mut jeux, dir, excluded);
    jeux
}

// --- Steam ---------------------------------------------------------------------

fn contexte() -> Value {
    json!({ "language": locale::steam_langue(), "country_code": locale::region() })
}

fn donnees_completes() -> Value {
    json!({
        "include_assets": true,
        "include_release": true,
        "include_reviews": true,
        "include_basic_info": true,
        "include_tag_count": 6,
    })
}

/// Un appel à l'API de la boutique. `None` = panne ; une réponse sans jeu = liste vide.
fn steam(url: &str, entree: &Value) -> Option<Vec<Value>> {
    let racine: Value = ureq::get(url)
        .query("input_json", &entree.to_string())
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    let reponse = racine.get("response")?;
    Some(reponse["store_items"].as_array().cloned().unwrap_or_default())
}

/// `Query` limité aux jeux, contenu adulte exclu.
fn requete(tri: u32, nombre: u32, mut filtres: Value, donnees: Value) -> Option<Vec<Value>> {
    filtres["type_filters"] = json!({ "include_games": true });
    filtres["content_descriptors_excluded"] = json!(ADULTE);
    let entree = json!({
        "query": { "start": 0, "count": nombre, "sort": tri, "filters": filtres },
        "context": contexte(),
        "data_request": donnees,
    });
    steam(QUERY, &entree)
}

/// Les jeux bruts d'un rayon, tels que Steam les renvoie.
fn brut(rayon: Rayon) -> Option<Vec<Value>> {
    let payants = json!({ "exclude_free_items": true });
    match rayon {
        // Populaires, en promo d'au moins 20 %, et bien reçus (« plutôt positives » au
        // minimum côté Steam, puis 75 % d'avis positifs dans `lire_rayon`). On en
        // demande plus qu'on n'en montre : l'interface retire ceux déjà possédés.
        Rayon::Vedette => requete(
            10,
            30,
            json!({
                "released_only": true,
                "price_filters": { "exclude_free_items": true, "min_discount_percent": 20 },
                "review_score_filter": { "min_score": 6 },
            }),
            donnees_completes(),
        ),
        Rayon::Ventes => requete(10, 20, json!({ "price_filters": payants }), donnees_completes()),
        // ⚠️ `release_date_filter` existe dans le protocole mais Steam l'ignore sans un
        // `release_date_type` qu'on ne connaît pas (mesuré : 113 786 résultats « filtrés »).
        // On prend donc les meilleures ventes, sans rien d'autre que leur date, on garde
        // les sorties récentes, et on ne demande la fiche complète que pour celles-là.
        Rayon::Nouveautes => {
            let liste = requete(
                10,
                150,
                json!({ "released_only": true, "price_filters": payants }),
                json!({ "include_release": true }),
            )?;
            let appids = recents(&liste, maintenant(), PAR_RAYON + 2);
            if appids.is_empty() {
                return Some(Vec::new());
            }
            details(&appids)
        }
        Rayon::AVenir => requete(13, 20, json!({ "coming_soon_only": true }), donnees_completes()),
        Rayon::Genre(tag) => requete(
            10,
            20,
            json!({
                "released_only": true,
                "tagids_must_match": [{ "tagids": [tag] }],
                "price_filters": payants,
            }),
            donnees_completes(),
        ),
        Rayon::Gratuits => requete(
            10,
            16,
            json!({ "price_filters": { "only_free_items": true } }),
            donnees_completes(),
        ),
    }
}

/// Fiches complètes d'une liste d'appids, dans l'ordre demandé.
fn details(appids: &[u64]) -> Option<Vec<Value>> {
    let ids: Vec<Value> = appids.iter().map(|a| json!({ "appid": a })).collect();
    let items = steam(
        GET_ITEMS,
        &json!({ "ids": ids, "context": contexte(), "data_request": donnees_completes() }),
    )?;
    let mut par_id: HashMap<u64, Value> = items
        .into_iter()
        .filter_map(|i| Some((i["appid"].as_u64()?, i)))
        .collect();
    Some(appids.iter().filter_map(|a| par_id.remove(a)).collect())
}

/// Appids des jeux sortis dans la fenêtre des nouveautés, dans l'ordre des ventes.
fn recents(items: &[Value], maintenant: i64, nombre: usize) -> Vec<u64> {
    items
        .iter()
        .filter(|i| {
            let sortie = i["release"]["steam_release_date"].as_i64().unwrap_or(0);
            sortie > maintenant - FENETRE_NOUVEAUTES && sortie <= maintenant
        })
        .filter_map(|i| i["appid"].as_u64())
        .take(nombre)
        .collect()
}

/// Lit les jeux d'un rayon et applique ses règles propres.
fn lire_rayon(rayon: Rayon, items: &[Value], noms: &HashMap<u64, String>) -> Vec<ShelfItem> {
    let jeux = items.iter().filter_map(|i| lire_jeu(i, noms));
    match rayon {
        Rayon::Vedette => jeux
            .filter(|j| j.review_percent.unwrap_or(0) >= AVIS_VEDETTE && j.hero_url.is_some())
            .take(PAR_RAYON)
            .collect(),
        _ => jeux.take(PAR_RAYON).collect(),
    }
}

/// Un item de l'API de la boutique. `None` pour ce qui n'est pas un jeu à montrer :
/// réponse en échec, autre chose qu'un jeu (`type` 0), contenu adulte.
fn lire_jeu(item: &Value, noms: &HashMap<u64, String>) -> Option<ShelfItem> {
    if item["success"].as_u64().is_some_and(|s| s != 1) || item["visible"].as_bool() == Some(false) {
        return None;
    }
    if item["type"].as_u64().is_some_and(|t| t != 0) {
        return None;
    }
    let descripteurs = item["content_descriptorids"].as_array();
    if descripteurs.is_some_and(|d| d.iter().any(|c| c.as_u64().is_some_and(|c| ADULTE.contains(&c)))) {
        return None;
    }
    let app_id = item["appid"].as_u64().or_else(|| item["id"].as_u64())?;
    let title = item["name"].as_str()?.trim().to_string();
    if title.is_empty() {
        return None;
    }
    let assets = &item["assets"];
    let asset = |cles: &[&str]| super::steam_art::adresse_asset(assets, cles);
    let avis = &item["reviews"]["summary_filtered"];
    let sortie = &item["release"];
    let tags = item["tags"]
        .as_array()
        .map(|t| {
            t.iter()
                .filter_map(|t| t["tagid"].as_u64())
                .filter(|id| !TAGS_MASQUES.contains(id))
                .filter_map(|id| noms.get(&id).cloned())
                .take(3)
                .collect()
        })
        .unwrap_or_default();
    let nb_avis = avis["review_count"].as_u64().unwrap_or(0) as u32;
    Some(ShelfItem {
        app_id,
        title,
        capsule_url: asset(&["library_capsule_2x", "library_capsule"]),
        header_url: asset(&["header", "main_capsule"]),
        hero_url: asset(&["library_hero", "library_hero_2x"]),
        description: item["basic_info"]["short_description"]
            .as_str()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(String::from),
        tags,
        // Pas d'avis du tout : Steam renvoie un score à 0 % qu'il ne faut pas afficher.
        review_score: if nb_avis > 0 { avis["review_score"].as_u64().unwrap_or(0) as u32 } else { 0 },
        review_percent: (nb_avis > 0)
            .then(|| avis["percent_positive"].as_u64())
            .flatten()
            .map(|p| p as u32),
        review_count: nb_avis,
        release_date: sortie["steam_release_date"].as_i64(),
        release_display: sortie["coming_soon_display"].as_str().unwrap_or("date_full").to_string(),
        release_text: sortie["custom_release_date_message"].as_str().map(String::from),
        coming_soon: sortie["is_coming_soon"].as_bool().unwrap_or(false),
        is_free: item["is_free"].as_bool().unwrap_or(false),
        ..Default::default()
    })
}

// --- Noms des tags --------------------------------------------------------------

/// Les noms changent rarement : un mois de cache, par langue.
const DUREE_TAGS: u64 = 30 * 86_400;

#[derive(Serialize, Deserialize, Default)]
struct NomsTags {
    at: u64,
    noms: HashMap<u64, String>,
}

/// ⚠️ Le mot `cache` dans le nom n'est pas décoratif : « Vider le cache » (Paramètres)
/// supprime les `*cache*.json` du dossier de config.
fn fichier_tags(dir: &Path) -> PathBuf {
    dir.join("steam_tags_cache_v1.json")
}

/// Nom des tags Steam dans la langue de l'interface. Un cache périmé sert encore si
/// Steam ne répond pas : des tags d'il y a deux mois valent mieux que pas de tags.
fn noms_tags(dir: &Path) -> HashMap<u64, String> {
    let langue = locale::steam_langue();
    let mut cache: HashMap<String, NomsTags> = std::fs::read_to_string(fichier_tags(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let t = maintenant() as u64;
    if let Some(c) = cache.get(langue) {
        if t.saturating_sub(c.at) < DUREE_TAGS && !c.noms.is_empty() {
            return c.noms.clone();
        }
    }
    let frais: Option<HashMap<u64, String>> = ureq::get(&format!("{TAGS}/{langue}"))
        .timeout(Duration::from_secs(10))
        .call()
        .ok()
        .and_then(|r| r.into_json::<Value>().ok())
        .and_then(|v| {
            let noms: HashMap<u64, String> = v
                .as_array()?
                .iter()
                .filter_map(|t| Some((t["tagid"].as_u64()?, t["name"].as_str()?.to_string())))
                .collect();
            (!noms.is_empty()).then_some(noms)
        });
    match frais {
        Some(noms) => {
            cache.insert(langue.to_string(), NomsTags { at: t, noms: noms.clone() });
            if std::fs::create_dir_all(dir).is_ok() {
                if let Ok(json) = serde_json::to_string(&cache) {
                    let _ = std::fs::write(fichier_tags(dir), json);
                }
            }
            noms
        }
        None => cache.remove(langue).map(|c| c.noms).unwrap_or_default(),
    }
}

// --- Identifiants ITAD -----------------------------------------------------------

/// Un appid trouvé chez ITAD le reste : la correspondance ne bouge pas. On revérifie
/// quand même au bout de trois mois (fusion de fiches chez ITAD).
const DUREE_ID_TROUVE: u64 = 90 * 86_400;
/// Un appid inconnu d'ITAD est redemandé le lendemain : c'est en général un jeu qui
/// vient d'être annoncé, qu'ITAD référence quelques jours plus tard.
const DUREE_ID_ABSENT: u64 = 86_400;

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
struct IdItad {
    id: Option<String>,
    at: u64,
}

impl IdItad {
    fn perime(&self, t: u64) -> bool {
        let duree = if self.id.is_some() { DUREE_ID_TROUVE } else { DUREE_ID_ABSENT };
        t.saturating_sub(self.at) >= duree
    }
}

/// Plusieurs rayons peuvent se charger en même temps : sans verrou, deux écritures du
/// cache se marcheraient dessus et la seconde effacerait les ids trouvés par la première.
static VERROU_IDS: Mutex<()> = Mutex::new(());

fn fichier_ids(dir: &Path) -> PathBuf {
    dir.join("itad_ids_cache_v1.json")
}

/// appid Steam → identifiant ITAD, pour une liste de jeux.
///
/// 🔑 UN SEUL APPEL pour toute la liste (`POST lookup/id/shop/61/v1`), et un cache disque
/// qui rend la plupart des ouvertures gratuites. Si le proxy refuse l'appel groupé (un
/// proxy pas encore mis à jour répond 403), on retombe sur `games/lookup/v1` jeu par
/// jeu : plus lent, mais la Boutique marche avec n'importe quelle version du proxy.
fn ids_itad(appids: &[u64], dir: &Path) -> HashMap<u64, String> {
    let _verrou = VERROU_IDS.lock().unwrap_or_else(|e| e.into_inner());
    let mut cache: HashMap<u64, IdItad> = std::fs::read_to_string(fichier_ids(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let t = maintenant() as u64;
    let mut a_demander: Vec<u64> = appids
        .iter()
        .copied()
        .filter(|a| cache.get(a).map_or(true, |c| c.perime(t)))
        .collect();
    a_demander.sort_unstable();
    a_demander.dedup();

    if !a_demander.is_empty() {
        let trouves = lookup_groupe(&a_demander).unwrap_or_else(|| lookup_un_par_un(&a_demander));
        if !trouves.is_empty() {
            for (appid, id) in trouves {
                cache.insert(appid, IdItad { id, at: t });
            }
            if std::fs::create_dir_all(dir).is_ok() {
                if let Ok(json) = serde_json::to_string(&cache) {
                    let _ = std::fs::write(fichier_ids(dir), json);
                }
            }
        }
    }

    appids
        .iter()
        .filter_map(|a| Some((*a, cache.get(a)?.id.clone()?)))
        .collect()
}

/// L'appel groupé. `None` = refusé ou en panne (on bascule sur l'appel jeu par jeu).
fn lookup_groupe(appids: &[u64]) -> Option<Vec<(u64, Option<String>)>> {
    let corps: Vec<String> = appids.iter().map(|a| format!("app/{a}")).collect();
    let reponse = store::post_json(&format!("lookup/id/shop/{ITAD_STEAM}/v1"), &json!(corps))?;
    Some(lire_lookup_groupe(&reponse, appids))
}

/// Réponse `{ "app/220": "<uuid>", "app/137730": null }`. Un appid absent de la réponse
/// n'est pas tranché : il n'entre pas en cache (il sera redemandé).
fn lire_lookup_groupe(reponse: &Value, appids: &[u64]) -> Vec<(u64, Option<String>)> {
    let Some(objet) = reponse.as_object() else { return Vec::new() };
    appids
        .iter()
        .filter_map(|a| {
            let valeur = objet.get(&format!("app/{a}"))?;
            Some((*a, valeur.as_str().map(String::from)))
        })
        .collect()
}

/// Repli : un `games/lookup/v1` par jeu, sur plusieurs fils. Une panne n'est pas une
/// absence : seuls les jeux pour lesquels ITAD a répondu sont renvoyés.
fn lookup_un_par_un(appids: &[u64]) -> Vec<(u64, Option<String>)> {
    let suivant = AtomicUsize::new(0);
    std::thread::scope(|s| {
        let fils: Vec<_> = (0..ITAD_WORKERS.min(appids.len()))
            .map(|_| {
                s.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let i = suivant.fetch_add(1, Ordering::Relaxed);
                        let Some(&appid) = appids.get(i) else { break };
                        if let Some(r) = store::get_json(&format!("games/lookup/v1?appid={appid}")) {
                            let id = r["game"]["id"].as_str().map(String::from);
                            local.push((appid, id));
                        }
                    }
                    local
                })
            })
            .collect();
        fils.into_iter().flat_map(|f| f.join().unwrap_or_default()).collect()
    })
}

// --- Prix -------------------------------------------------------------------------

/// Donne à chaque jeu son identifiant ITAD et sa meilleure offre (un appel groupé par
/// tranche de 200 jeux, la limite d'ITAD).
fn completer(jeux: &mut [ShelfItem], dir: &Path, excluded: &HashSet<String>) {
    if jeux.is_empty() {
        return;
    }
    let appids: Vec<u64> = jeux.iter().map(|j| j.app_id).collect();
    let ids = ids_itad(&appids, dir);
    for j in jeux.iter_mut() {
        if let Some(id) = ids.get(&j.app_id) {
            j.game_id = id.clone();
        }
    }
    // Un jeu gratuit n'a pas d'offre à comparer : inutile de le faire tarifer.
    let mut a_tarifer: Vec<String> = jeux
        .iter()
        .filter(|j| !j.is_free && !j.game_id.is_empty())
        .map(|j| j.game_id.clone())
        .collect();
    a_tarifer.sort();
    a_tarifer.dedup();
    let mut entrees: HashMap<String, Value> = HashMap::new();
    for tranche in a_tarifer.chunks(200) {
        let Some(reponse) = store::post_json(&store::chemin_prix(), &json!(tranche)) else { continue };
        for e in reponse.as_array().cloned().unwrap_or_default() {
            if let Some(id) = e["id"].as_str() {
                entrees.insert(id.to_string(), e.clone());
            }
        }
    }
    for j in jeux.iter_mut() {
        if let Some(e) = entrees.get(&j.game_id) {
            appliquer_prix(j, e, excluded);
        }
    }
}

/// Applique une entrée `games/prices/v3` à un jeu.
fn appliquer_prix(jeu: &mut ShelfItem, entree: &Value, excluded: &HashSet<String>) {
    jeu.history_low = store::num(&entree["historyLow"]["all"]["amount"]);
    jeu.currency = store::devise(&entree["historyLow"]["all"]);
    let Some(offre) = store::cheapest(entree, excluded) else { return };
    let Some(prix) = store::num(&offre["price"]["amount"]) else { return };
    jeu.price = Some(prix);
    jeu.normal_price = Some(store::num(&offre["regular"]["amount"]).unwrap_or(prix));
    jeu.currency = store::devise(&offre["price"]);
    jeu.savings = store::num(&offre["cut"]).unwrap_or(0.0).round() as u32;
    jeu.store_name = offre["shop"]["name"].as_str().unwrap_or_default().to_string();
    jeu.buy_url = offre["url"].as_str().unwrap_or_default().to_string();
    jeu.record = store::est_record(offre);
    jeu.expiry = store::fin_offre(offre);
}

fn maintenant() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noms() -> HashMap<u64, String> {
        [(122, "RPG"), (1695, "Monde ouvert"), (6650, "Nudité"), (4182, "Solo"), (19, "Action")]
            .into_iter()
            .map(|(id, n)| (id, n.to_string()))
            .collect()
    }

    /// Item de `Query` tel que relevé le 22/10/2026 (raccourci).
    fn cyberpunk() -> Value {
        json!({
            "item_type": 0, "id": 1091500, "success": 1, "visible": true,
            "name": "Cyberpunk 2077", "appid": 1091500, "type": 0,
            "content_descriptorids": [1, 2, 5],
            "reviews": { "summary_filtered": { "review_count": 892730, "percent_positive": 87, "review_score": 8, "review_score_label": "très positives" } },
            "basic_info": { "short_description": "Cyberpunk 2077 est un JDR d'action-aventure en monde ouvert." },
            "release": { "steam_release_date": 1607558400 },
            "tags": [ { "tagid": 1695, "weight": 900 }, { "tagid": 6650, "weight": 850 }, { "tagid": 122, "weight": 800 }, { "tagid": 99999, "weight": 700 }, { "tagid": 4182, "weight": 600 }, { "tagid": 19, "weight": 500 } ],
            "assets": {
                "asset_url_format": "steam/apps/1091500/${FILENAME}?t=1",
                "header": "a1/header.jpg",
                "library_capsule_2x": "b2/library_capsule_2x.jpg",
                "library_hero": "c3/library_hero.jpg"
            }
        })
    }

    #[test]
    fn lit_un_jeu_de_la_boutique() {
        let j = lire_jeu(&cyberpunk(), &noms()).unwrap();
        assert_eq!(j.app_id, 1091500);
        assert_eq!(j.title, "Cyberpunk 2077");
        assert!(j.capsule_url.unwrap().ends_with("/b2/library_capsule_2x.jpg?t=1"));
        assert!(j.header_url.unwrap().ends_with("/a1/header.jpg?t=1"));
        assert!(j.hero_url.unwrap().ends_with("/c3/library_hero.jpg?t=1"));
        // La nudité est écartée, un tag sans nom aussi, et on s'arrête à trois.
        assert_eq!(j.tags, vec!["Monde ouvert", "RPG", "Solo"]);
        assert_eq!((j.review_score, j.review_percent, j.review_count), (8, Some(87), 892730));
        assert_eq!(j.release_date, Some(1607558400));
        assert!(!j.coming_soon && !j.is_free);
        assert!(j.description.unwrap().starts_with("Cyberpunk 2077 est"));
    }

    #[test]
    fn ecarte_le_contenu_adulte_et_ce_qui_n_est_pas_un_jeu() {
        let mut adulte = cyberpunk();
        adulte["content_descriptorids"] = json!([1, 3]);
        assert!(lire_jeu(&adulte, &noms()).is_none());

        let mut materiel = cyberpunk();
        materiel["type"] = json!(6);
        assert!(lire_jeu(&materiel, &noms()).is_none());

        let mut echec = cyberpunk();
        echec["success"] = json!(2);
        assert!(lire_jeu(&echec, &noms()).is_none());
    }

    /// Un jeu à venir sans avis : pas de faux « 0 % », et le texte de l'éditeur.
    #[test]
    fn jeu_a_venir_sans_avis() {
        let item = json!({
            "appid": 5, "type": 0, "name": "Sheep Tag 2",
            "reviews": { "summary_filtered": { "review_count": 0, "percent_positive": 0, "review_score": 0 } },
            "release": { "is_coming_soon": true, "custom_release_date_message": "Prochainement", "coming_soon_display": "text_comingsoon" }
        });
        let j = lire_jeu(&item, &noms()).unwrap();
        assert_eq!((j.review_score, j.review_percent), (0, None));
        assert!(j.coming_soon);
        assert_eq!(j.release_display, "text_comingsoon");
        assert_eq!(j.release_text.as_deref(), Some("Prochainement"));
    }

    #[test]
    fn la_vedette_veut_des_avis_et_une_banniere() {
        let bon = cyberpunk();
        let mut tiede = cyberpunk();
        tiede["appid"] = json!(2);
        tiede["reviews"]["summary_filtered"]["percent_positive"] = json!(61);
        let mut sans_banniere = cyberpunk();
        sans_banniere["appid"] = json!(3);
        sans_banniere["assets"] = json!({ "asset_url_format": "steam/apps/3/${FILENAME}", "header": "h.jpg" });
        let v = lire_rayon(Rayon::Vedette, &[tiede, bon, sans_banniere], &noms());
        assert_eq!(v.iter().map(|j| j.app_id).collect::<Vec<_>>(), vec![1091500]);
    }

    #[test]
    fn nouveautes_dans_la_fenetre() {
        let t = 2_000_000_000;
        let items = vec![
            json!({ "appid": 1, "release": { "steam_release_date": t - 10 * 86_400 } }),
            json!({ "appid": 2, "release": { "steam_release_date": t - 400 * 86_400 } }),
            json!({ "appid": 3, "release": { "steam_release_date": t + 86_400 } }), // précommande
            json!({ "appid": 4 }),
            json!({ "appid": 5, "release": { "steam_release_date": t - 74 * 86_400 } }),
        ];
        assert_eq!(recents(&items, t, 10), vec![1, 5]);
        assert_eq!(recents(&items, t, 1), vec![1]);
    }

    #[test]
    fn lit_la_correspondance_groupee() {
        let r = json!({ "app/220": "018d937f-aaaa", "app/137730": null });
        let mut lu = lire_lookup_groupe(&r, &[220, 137730, 42]);
        lu.sort();
        // 42 n'est pas dans la réponse : non tranché, donc pas renvoyé.
        assert_eq!(lu, vec![(220, Some("018d937f-aaaa".to_string())), (137730, None)]);
    }

    #[test]
    fn un_id_absent_perime_plus_vite() {
        let t = 1_000 * 86_400;
        assert!(IdItad { id: None, at: t - 2 * 86_400 }.perime(t));
        assert!(!IdItad { id: Some("x".into()), at: t - 2 * 86_400 }.perime(t));
    }

    #[test]
    fn applique_la_meilleure_offre_autorisee() {
        let entree = json!({
            "id": "g1",
            "historyLow": { "all": { "amount": 12.18, "currency": "EUR" } },
            "deals": [
                { "shop": { "name": "JoyBuggy" }, "price": { "amount": 9.99, "currency": "EUR" }, "regular": { "amount": 39.99 }, "cut": 75, "url": "https://joy", "flag": "N" },
                { "shop": { "name": "Steam" }, "price": { "amount": 13.99, "currency": "EUR" }, "regular": { "amount": 39.99 }, "cut": 65, "url": "https://steam", "flag": "S", "expiry": "2026-11-03T00:59:00+01:00" }
            ]
        });
        let mut j = ShelfItem::default();
        appliquer_prix(&mut j, &entree, &HashSet::new());
        assert_eq!((j.price, j.store_name.as_str(), j.record), (Some(9.99), "JoyBuggy", true));

        let masques: HashSet<String> = ["JoyBuggy".to_string()].into_iter().collect();
        let mut j = ShelfItem::default();
        appliquer_prix(&mut j, &entree, &masques);
        assert_eq!(j.price, Some(13.99));
        assert_eq!(j.normal_price, Some(39.99));
        assert_eq!(j.savings, 65);
        assert_eq!(j.currency, "EUR");
        assert_eq!(j.history_low, Some(12.18));
        // `S` = plus bas de cette boutique seulement : pas un record.
        assert!(!j.record);
        assert_eq!(j.expiry.as_deref(), Some("2026-11-03T00:59:00+01:00"));
    }

    #[test]
    fn plein_tarif_n_est_pas_un_record() {
        // Relevé réel : un jeu qui sort est « H » à 69,99 €, -0 %.
        let entree = json!({
            "id": "cod",
            "historyLow": { "all": { "amount": 69.99, "currency": "EUR" } },
            "deals": [ { "shop": { "name": "Steam" }, "price": { "amount": 69.99, "currency": "EUR" }, "regular": { "amount": 69.99 }, "cut": 0, "url": "https://steam", "flag": "H" } ]
        });
        let mut j = ShelfItem::default();
        appliquer_prix(&mut j, &entree, &HashSet::new());
        assert_eq!(j.price, Some(69.99));
        assert!(!j.record);
    }

    #[test]
    fn la_vedette_garde_les_vraies_promos() {
        let jeu = |app_id, price: Option<f64>, savings| ShelfItem { app_id, price, savings, ..Default::default() };
        let gardes = garder_vraies_promos(vec![jeu(1, Some(17.99), 70), jeu(2, Some(38.49), 4), jeu(3, None, 0)]);
        assert_eq!(gardes.iter().map(|j| j.app_id).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn noms_des_rayons() {
        assert_eq!(Rayon::depuis("topSellers", None), Some(Rayon::Ventes));
        assert_eq!(Rayon::depuis("genre", Some(122)), Some(Rayon::Genre(122)));
        assert_eq!(Rayon::depuis("genre", None), None);
        assert_eq!(Rayon::depuis("autre", None), None);
    }
}
