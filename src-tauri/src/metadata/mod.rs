pub mod gog_store;
pub mod igdb;
pub mod instant_gaming;
pub mod steam_art;
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
    //
    // 🔑 **UN FICHIER PAR LANGUE.** Une entrée contient une description et des genres dans
    // la langue où ils ont été demandés. Le problème s'est déjà posé une fois — c'est
    // exactement ce que raconte le passage à v4 ci-dessus, réglé alors en jetant tout le
    // cache. Le jeter à chaque changement de langue serait cette fois absurde : on
    // rebasculerait sans arrêt entre deux contenus qu'il faudrait retélécharger en entier.
    // Chaque langue garde donc ce qu'elle a déjà obtenu.
    //
    // ⚠️ **LE FRANÇAIS GARDE LE NOM HISTORIQUE**, sans suffixe. Les installations
    // existantes ont un `metadata_cache_v5.json` rempli de français — souvent plusieurs
    // centaines de fiches. Leur ajouter un suffixe les rendrait invisibles du jour au
    // lendemain et déclencherait un retéléchargement complet chez tout le monde, pour un
    // changement dont l'immense majorité des utilisateurs n'a que faire.
    match crate::locale::langue() {
        crate::locale::Langue::Fr => config_dir.join("metadata_cache_v5.json"),
        crate::locale::Langue::En => config_dir.join("metadata_cache_v5_en.json"),
    }
}

/// Le cache d'avant le refus des DLC : ses entrées devinées peuvent décrire un lot de
/// pièces ou un pass de combat au lieu du jeu.
///
/// ⚠️ Pas de variante par langue : la v4 n'a existé qu'en français, et la reprise
/// ci-dessous n'a donc de sens que vers le cache français.
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

/// Une fiche remplie est redemandée au bout d'un mois : descriptions, traductions et
/// captures évoluent, surtout dans les semaines qui suivent la sortie d'un jeu.
const DUREE_REMPLIE: u64 = 30 * 86_400;
/// Une fiche vide l'est dès le lendemain : un jeu qui vient de sortir n'a souvent pas
/// encore de page complète, et une panne réseau ressemble à s'y méprendre à « rien ».
const DUREE_VIDE: u64 = 86_400;

fn est_vide(meta: &GameMeta) -> bool {
    meta.description.is_none() && meta.cover_url.is_none() && meta.name.is_none()
}

fn maintenant() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 🔑 **LE CACHE EXPIRE.** Il ne le faisait pas : une fiche récupérée une fois l'était
/// pour toujours. Un jeu ouvert le jour de sa sortie gardait sa fiche vide, et un texte
/// que Steam a traduit depuis (Hide and Moo!, relevé le 26 septembre 2026) restait en
/// anglais indéfiniment. Les entrées d'avant ce changement (`fetched_at` = 0) sont
/// périmées d'office et se renouvellent une à une, à l'ouverture de leur fiche.
fn perimee(meta: &GameMeta, t: u64) -> bool {
    let duree = if est_vide(meta) { DUREE_VIDE } else { DUREE_REMPLIE };
    t.saturating_sub(meta.fetched_at) >= duree
}

/// Enrichit un **seul** jeu à la demande (ouverture de la vue détail), avec le
/// même cache disque que l'enrichissement en masse. Retour vide si rien trouvé.
pub fn enrich_one(game: &GameDto, config_dir: &Path) -> GameMeta {
    let mut cache = load_cache(config_dir);
    let t = maintenant();
    let ancienne = cache.get(&game.id).cloned();
    if let Some(meta) = &ancienne {
        if !perimee(meta, t) {
            return meta.clone();
        }
    }
    let meta = match (fetch(game, config_dir), ancienne) {
        (Some(neuve), _) => GameMeta { fetched_at: t, ..neuve },
        // ⚠️ Rien obtenu alors qu'on avait une fiche : panne probable. On garde l'ancienne
        // plutôt que de l'effacer, et on retentera demain (et non à chaque ouverture, qui
        // attendrait à chaque fois l'expiration du délai réseau).
        (None, Some(vieille)) if !est_vide(&vieille) => GameMeta {
            fetched_at: t - (DUREE_REMPLIE - DUREE_VIDE),
            ..vieille
        },
        (None, _) => GameMeta { fetched_at: t, ..GameMeta::default() },
    };
    cache.insert(game.id.clone(), meta.clone());
    save_cache(config_dir, &cache);
    meta
}

/// Oublie la fiche d'un jeu (bouton « Actualiser les infos »), dans la langue courante.
pub fn oublier(config_dir: &Path, id: &str) {
    let mut cache = load_cache(config_dir);
    if cache.remove(id).is_some() {
        save_cache(config_dir, &cache);
    }
}

/// Récupère les métadonnées d'un jeu selon sa plateforme.
fn fetch(game: &GameDto, config_dir: &Path) -> Option<GameMeta> {
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
        // Battle.net : identifiant Steam connu pour les jeux aussi vendus sur Steam, donc
        // aussi sûr qu'un jeu Steam (cf. `battlenet::steam_appid`). Les autres sont devinés.
        "battlenet" => {
            let code = game.id.strip_prefix("battlenet:").unwrap_or_default();
            match crate::accounts::battlenet::steam_appid(code) {
                Some(appid) => steam_store::appdetails(appid),
                None => deviner(game),
            }
        }
        // Epic : le catalogue Epic, par identifiant (compte connecté). À défaut, devinette.
        "epic" => {
            let app = game.id.strip_prefix("epic:").unwrap_or(&game.launch_target);
            crate::accounts::epic::fiche(config_dir, app).or_else(|| deviner(game))
        }
        // Manuel et autres : on tente une correspondance par titre sur Steam.
        _ => deviner(game),
    }
}

/// Devine le jeu sur le Steam Store par son titre.
fn deviner(game: &GameDto) -> Option<GameMeta> {
    let appid = steam_store::search_appid(&game.title)?;
    let mut meta = steam_store::appdetails(&appid)?;
    // ⚠️ Le contenu est bien en français, mais le JEU est deviné : on retire le
    // drapeau pour que le front ne remplace PAS la description d'IGDB. Une
    // description française du mauvais jeu est pire qu'une bonne en anglais —
    // « Control » (Epic) tombe ainsi sur CONTROL Resonant, un autre jeu.
    meta.localized = false;
    Some(meta)
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

    #[test]
    fn une_fiche_vide_expire_plus_vite() {
        let t = 100 * 86_400;
        let vide = GameMeta { fetched_at: t - 2 * 86_400, ..Default::default() };
        let remplie = GameMeta {
            description: Some("x".into()),
            fetched_at: t - 2 * 86_400,
            ..Default::default()
        };
        assert!(perimee(&vide, t));
        assert!(!perimee(&remplie, t));
        let ancienne = GameMeta { description: Some("x".into()), ..Default::default() };
        assert!(perimee(&ancienne, t), "une entrée d'avant l'expiration est périmée");
    }
}
