pub mod gog_store;
pub mod igdb;
pub mod instant_gaming;
pub mod steam_store;
pub mod store;

use crate::models::{GameDto, GameMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Cache = HashMap<String, GameMeta>;

fn cache_file(config_dir: &Path) -> PathBuf {
    // Suffixe versionné : à incrémenter quand le schéma `GameMeta` évolue OU la recherche
    // s'améliore, pour ignorer les anciennes entrées. v2 = ajout `size_gb` ; v3 = recherche
    // avec repli sans numéro final (résout OW2 & co) ; v4 = sources interrogées en français
    // (les entrées v3 contiennent des descriptions et des genres anglais) ; v5 = les DLC
    // sont refusés et le nom exact prime sur l'approchant (cf. `steam_store`).
    config_dir.join("metadata_cache_v5.json")
}

/// Le cache d'avant le refus des DLC : ses entrées devinées peuvent décrire un lot de
/// pièces ou un pass de combat au lieu du jeu.
fn cache_file_v4(config_dir: &Path) -> PathBuf {
    config_dir.join("metadata_cache_v4.json")
}

fn lire(chemin: &Path) -> Option<Cache> {
    std::fs::read_to_string(chemin)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

/// Vrai si l'entrée vient d'une source **autoritaire** et non d'une devinette.
///
/// 🔑 C'EST L'IDENTIFIANT QUI LE DIT. `fetch` n'a que trois chemins : Steam interroge
/// l'appid du jeu, GOG son identifiant produit — dans les deux cas on demande CE jeu-là,
/// la réponse ne peut pas désigner autre chose. Tout le reste (Epic, Battle.net, Riot,
/// manuel) est deviné en cherchant le TITRE sur le Steam Store, et c'est le seul chemin
/// qui ait jamais pu ramener un DLC. Les préfixes d'identifiant les séparent sans
/// ambiguïté, et permettent de ne jeter que ce qui est suspect.
fn source_sure(id: &str) -> bool {
    id.starts_with("steam:") || id.starts_with("gog:")
}

/// Lit le cache, en jetant au passage les entrées qu'une devinette a pu salir.
///
/// ⚠️ POURQUOI PAS UNE INVALIDATION SÈCHE. Repartir de zéro reprendrait la description,
/// l'image et la taille de CHAQUE jeu — des centaines d'appels au Steam Store pour
/// réparer une poignée d'entrées. Or les entrées fausses ne peuvent venir que du chemin
/// deviné : les conserver quand elles sont sûres coûte une condition et épargne tout le
/// reste. Le fichier v4 est laissé en place, comme filet si l'écriture du v5 est coupée.
fn load_cache(config_dir: &Path) -> Cache {
    if let Some(cache) = lire(&cache_file(config_dir)) {
        return cache;
    }
    match lire(&cache_file_v4(config_dir)) {
        Some(v4) => {
            let repare: Cache = v4.into_iter().filter(|(id, _)| source_sure(id)).collect();
            save_cache(config_dir, &repare);
            repare
        }
        None => Cache::default(),
    }
}

fn save_cache(config_dir: &Path, cache: &Cache) {
    if std::fs::create_dir_all(config_dir).is_ok() {
        if let Ok(json) = serde_json::to_string_pretty(cache) {
            let _ = std::fs::write(cache_file(config_dir), json);
        }
    }
}

/// Enrichit un **seul** jeu à la demande (ouverture de la vue détail), avec le
/// même cache disque que l'enrichissement en masse. Retour vide si rien trouvé.
pub fn enrich_one(game: &GameDto, config_dir: &Path) -> GameMeta {
    let mut cache = load_cache(config_dir);
    if let Some(meta) = cache.get(&game.id) {
        return meta.clone();
    }
    let fetched = fetch(game).unwrap_or_default();
    cache.insert(game.id.clone(), fetched.clone());
    save_cache(config_dir, &cache);
    fetched
}

/// Récupère les métadonnées d'un jeu selon sa plateforme.
fn fetch(game: &GameDto) -> Option<GameMeta> {
    match game.platform.as_str() {
        "steam" => {
            let mut meta = steam_store::appdetails(&game.launch_target)?;
            // Taille d'installation via un 2e appel (api.steamcmd.net), seulement
            // utile pour un jeu non installé (l'installé a déjà sa taille disque).
            if !game.installed {
                meta.size_gb = steam_store::install_size_gb(&game.launch_target);
            }
            Some(meta)
        }
        // L'id produit GOG est dans `id` (« gog:<id> ») : `launch_target` est le
        // chemin de l'exe pour un jeu installé, inutilisable pour l'API.
        "gog" => gog_store::product(game.id.strip_prefix("gog:").unwrap_or(&game.launch_target)),
        // Epic / manuel : on tente une correspondance par titre sur Steam.
        _ => {
            let appid = steam_store::search_appid(&game.title)?;
            let mut meta = steam_store::appdetails(&appid)?;
            // ⚠️ Le contenu est bien en français, mais le JEU est deviné : on retire le
            // drapeau pour que le front ne remplace PAS la description d'IGDB. Une
            // description française du mauvais jeu est pire qu'une bonne en anglais.
            meta.localized = false;
            Some(meta)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La migration v4 -> v5 ne doit jeter que les entrées DEVINÉES. Steam et GOG
    /// interrogent l'identifiant du jeu : leur réponse ne peut pas décrire autre chose,
    /// et les refaire coûterait des centaines d'appels pour rien.
    #[test]
    fn seules_les_entrees_devinees_sont_jetees() {
        let dir = std::env::temp_dir().join(format!("torii-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut v4 = Cache::new();
        for id in ["steam:730", "gog:1207658930", "battlenet:pro", "epic:xyz", "manual:1"] {
            v4.insert(id.to_string(), GameMeta::default());
        }
        std::fs::write(cache_file_v4(&dir), serde_json::to_string(&v4).unwrap()).unwrap();

        let migre = load_cache(&dir);
        assert!(migre.contains_key("steam:730"), "Steam est autoritaire");
        assert!(migre.contains_key("gog:1207658930"), "GOG aussi");
        assert!(!migre.contains_key("battlenet:pro"), "Battle.net est deviné");
        assert!(!migre.contains_key("epic:xyz"), "Epic est deviné");
        assert!(!migre.contains_key("manual:1"), "un jeu manuel aussi");
        assert_eq!(migre.len(), 2);

        assert!(cache_file(&dir).exists(), "la v5 est écrite dès la première lecture");
        assert!(cache_file_v4(&dir).exists(), "la v4 reste, filet en cas d'écriture coupée");
        assert_eq!(load_cache(&dir).len(), 2, "la relecture repart de la v5");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
