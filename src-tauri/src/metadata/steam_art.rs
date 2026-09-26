//! Adresses réelles des jaquettes et bannières Steam.
//!
//! 🔑 **L'ADRESSE « DÉDUITE DE L'APPID » NE MARCHE PLUS POUR LES JEUX RÉCENTS.** Torii
//! fabriquait `…/steam/apps/<appid>/library_600x900.jpg` sans rien demander à personne.
//! Steam range désormais les images sous un dossier à empreinte :
//! `store_item_assets/steam/apps/<appid>/<sha1>/library_capsule_2x.jpg`. Les anciens jeux
//! répondent encore à l'adresse déduite (compatibilité), les nouveaux NON : Aniimo
//! (4126040, sorti en septembre 2026) répond 404 sur les cinq noms classiques, alors que
//! ses images existent bel et bien. Relevé le 26 septembre 2026.
//!
//! Seule l'API de la boutique connaît l'empreinte : `IStoreBrowseService/GetItems`,
//! **sans clé**, jusqu'à 250 jeux par appel (vérifié), qui renvoie un champ `assets`.
//!
//! ⚠️ L'adresse déduite reste le repli : si l'API ne répond pas (hors ligne), ou ne
//! connaît pas le jeu (retiré de la vente), on ne touche à rien.

use crate::models::GameDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API: &str = "https://api.steampowered.com/IStoreBrowseService/GetItems/v1";
const HOTE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
/// 250 acceptés (vérifié) ; on garde de la marge.
const LOT: usize = 200;

/// Une jaquette trouvée est revérifiée au bout d'un mois : un éditeur peut changer son
/// visuel, et l'empreinte change avec lui (l'ancienne adresse finit par disparaître).
const DUREE_TROUVE: u64 = 30 * 86_400;
/// Un jeu sans visuel est redemandé le lendemain : c'est typiquement un jeu qui vient de
/// sortir, dont l'éditeur n'a pas encore déposé ses images de bibliothèque.
const DUREE_ABSENT: u64 = 86_400;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct Art {
    pub cover: Option<String>,
    pub hero: Option<String>,
    /// Horodatage Unix de la dernière réponse de Steam pour ce jeu.
    pub at: u64,
}

impl Art {
    fn perime(&self, maintenant: u64) -> bool {
        let duree = if self.cover.is_some() { DUREE_TROUVE } else { DUREE_ABSENT };
        maintenant.saturating_sub(self.at) >= duree
    }
}

type Cache = HashMap<u64, Art>;

fn cache_file(dir: &Path) -> PathBuf {
    dir.join("steam_art_cache_v1.json")
}

fn load_cache(dir: &Path) -> Cache {
    std::fs::read_to_string(cache_file(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_cache(dir: &Path, cache: &Cache) {
    if std::fs::create_dir_all(dir).is_ok() {
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = std::fs::write(cache_file(dir), json);
        }
    }
}

fn maintenant() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Oublie un jeu : sa jaquette sera redemandée au prochain scan (bouton « Actualiser »).
pub fn oublier(dir: &Path, appid: u64) {
    let mut cache = load_cache(dir);
    if cache.remove(&appid).is_some() {
        save_cache(dir, &cache);
    }
}

/// Remplace les adresses déduites des jeux Steam par les vraies, en s'appuyant sur le
/// cache et en n'interrogeant Steam que pour les jeux inconnus ou périmés.
pub fn appliquer(games: &mut [GameDto], dir: &Path) {
    let mut cache = load_cache(dir);
    let t = maintenant();

    let mut a_demander: Vec<u64> = games
        .iter()
        .filter_map(appid)
        .filter(|id| cache.get(id).map_or(true, |a| a.perime(t)))
        .collect();
    a_demander.sort_unstable();
    a_demander.dedup();

    if !a_demander.is_empty() {
        let mut modifie = false;
        for lot in a_demander.chunks(LOT) {
            // Lot en panne : on ne mémorise RIEN, sans quoi une coupure réseau passerait
            // pour « pas de visuel » pendant une journée.
            let Some(reponse) = demander(lot) else { continue };
            for id in lot {
                let art = reponse.get(id).cloned().unwrap_or_default();
                cache.insert(*id, Art { at: t, ..art });
            }
            modifie = true;
        }
        if modifie {
            save_cache(dir, &cache);
        }
    }

    for g in games.iter_mut() {
        let Some(art) = appid(g).and_then(|id| cache.get(&id)) else { continue };
        if let Some(c) = &art.cover {
            g.cover_url = Some(c.clone());
        }
        if let Some(h) = &art.hero {
            g.hero_url = Some(h.clone());
        }
    }
}

fn appid(g: &GameDto) -> Option<u64> {
    if g.platform != "steam" {
        return None;
    }
    g.id.strip_prefix("steam:")?.parse().ok()
}

/// Un appel `GetItems`. `None` = pas de réponse exploitable (panne, format inattendu).
fn demander(appids: &[u64]) -> Option<HashMap<u64, Art>> {
    let ids: Vec<Value> = appids.iter().map(|a| serde_json::json!({ "appid": a })).collect();
    let entree = serde_json::json!({
        "ids": ids,
        "context": { "language": "english", "country_code": "US" },
        "data_request": { "include_assets": true },
    });
    let racine: Value = ureq::get(API)
        .query("input_json", &entree.to_string())
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    let items = racine["response"]["store_items"].as_array()?;
    Some(items.iter().filter_map(lire_item).collect())
}

fn lire_item(item: &Value) -> Option<(u64, Art)> {
    let id = item["appid"].as_u64().or_else(|| item["id"].as_u64())?;
    let assets = &item["assets"];
    let format = assets["asset_url_format"].as_str();
    let adresse = |cles: &[&str]| -> Option<String> {
        let format = format?;
        let fichier = cles.iter().find_map(|c| assets[*c].as_str())?;
        Some(format!("{HOTE}{}", format.replace("${FILENAME}", fichier)))
    };
    Some((
        id,
        Art {
            cover: adresse(&["library_capsule_2x", "library_capsule"]),
            hero: adresse(&["library_hero", "library_hero_2x"]),
            at: 0,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_les_adresses_a_empreinte() {
        let item = serde_json::json!({
            "appid": 4126040,
            "assets": {
                "asset_url_format": "steam/apps/4126040/${FILENAME}?t=1789813151",
                "library_capsule": "d57c/library_capsule.jpg",
                "library_capsule_2x": "d57c/library_capsule_2x.jpg",
                "library_hero": "7094/library_hero.jpg"
            }
        });
        let (id, art) = lire_item(&item).unwrap();
        assert_eq!(id, 4126040);
        assert_eq!(
            art.cover.as_deref(),
            Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/4126040/d57c/library_capsule_2x.jpg?t=1789813151")
        );
        assert!(art.hero.unwrap().ends_with("/7094/library_hero.jpg?t=1789813151"));
    }

    /// Un jeu sans visuel de bibliothèque ne doit rien remplacer : l'adresse déduite reste.
    #[test]
    fn sans_assets_rien_n_est_propose() {
        let (_, art) = lire_item(&serde_json::json!({ "appid": 10 })).unwrap();
        assert_eq!(art.cover, None);
        assert_eq!(art.hero, None);
    }

    #[test]
    fn un_absent_perime_plus_vite() {
        let t = 10 * 86_400 * 30;
        let absent = Art { at: t - 2 * 86_400, ..Default::default() };
        let trouve = Art { cover: Some("x".into()), at: t - 2 * 86_400, ..Default::default() };
        assert!(absent.perime(t));
        assert!(!trouve.perime(t));
    }
}
