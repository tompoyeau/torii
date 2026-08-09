//! **Jeux en commun avec les amis Steam.**
//!
//! Pour chaque ami dont le profil expose « Détails des jeux » (public, ou amis-seulement
//! puisqu'on est amis), on lit sa bibliothèque possédée via `GetOwnedGames` (même chemin
//! sans clé que nos propres jeux : un WebAPIToken tiré de notre page communautaire). On
//! croise ensuite avec NOTRE bibliothèque pour ne garder que les jeux qu'au moins un ami
//! partage, en indiquant lesquels. Le frontend calcule l'intersection selon la sélection.
//!
//! Steam-only : aucun autre launcher n'expose la bibliothèque des amis (cf. panneau Amis).

use super::{secrets, steam};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Nombre de bibliothèques d'amis récupérées en parallèle (GetOwnedGames tolère les rafales).
const FETCH_WORKERS: usize = 8;
/// Fraîcheur du cache : au-delà, un rafraîchissement silencieux est refait.
const CACHE_MAX_AGE_SECS: u64 = 6 * 3600;
const CACHE_FILE: &str = "friends_games_cache.json";

/// Un ami et l'état de lecture de sa bibliothèque.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FriendLib {
    pub steam_id: String,
    pub name: String,
    pub avatar_url: String,
    /// Vrai si sa bibliothèque n'a pas pu être lue (profil privé) → non filtrable.
    pub private: bool,
    /// Nombre de jeux qu'il possède en commun avec moi.
    pub common_count: usize,
}

/// Un de MES jeux, avec la liste des amis qui le possèdent aussi.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommonGame {
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>,
    /// SteamIDs des amis (parmi ceux lisibles) qui possèdent ce jeu.
    pub owners: Vec<String>,
}

/// Charge utile renvoyée au frontend (+ sérialisée telle quelle dans le cache).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FriendsCommon {
    pub friends: Vec<FriendLib>,
    pub games: Vec<CommonGame>,
    /// Horodatage Unix (secondes) du calcul — sert à la fraîcheur du cache.
    pub fetched_at: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Point d'entrée de la commande. Renvoie le cache s'il est frais (sauf `force`),
/// sinon recalcule en direct et le persiste. `None` si Steam n'est pas connecté.
pub fn compute(config_dir: &Path, force: bool) -> Option<FriendsCommon> {
    let creds = secrets::load(config_dir);
    let steam_id = creds.steam_id.clone()?;
    let cookie = creds
        .steam_community
        .clone()
        .or_else(|| creds.steam_login_secure.clone())?;

    if !force {
        if let Some(cached) = load_cache(config_dir) {
            if now().saturating_sub(cached.fetched_at) < CACHE_MAX_AGE_SECS {
                return Some(cached);
            }
        }
    }

    let fresh = fetch_live(&steam_id, &cookie)?;
    save_cache(config_dir, &fresh);
    Some(fresh)
}

/// Récupère en direct : mes jeux + la bibliothèque de chaque ami, puis le croisement.
fn fetch_live(steam_id: &str, cookie: &str) -> Option<FriendsCommon> {
    let token = steam::web_api_token(steam_id, cookie)?;

    // 1) Mes jeux Steam possédés (référentiel de l'intersection).
    let mine = steam::owned_with_token(steam_id, &token);
    if mine.is_empty() {
        // Sans notre propre bibliothèque, aucun « en commun » possible.
        return Some(FriendsCommon {
            fetched_at: now(),
            ..Default::default()
        });
    }
    // appid (= GameDto.id) → (titre, jaquette), et compteur d'amis possesseurs.
    let mut games: HashMap<String, CommonGame> = HashMap::with_capacity(mine.len());
    for g in &mine {
        games.insert(
            g.id.clone(),
            CommonGame {
                id: g.id.clone(),
                title: g.title.clone(),
                cover_url: g.cover_url.clone(),
                owners: Vec::new(),
            },
        );
    }

    // 2) Bibliothèque de chaque ami, en parallèle.
    let friends = steam::friends(steam_id, cookie);
    let libs = fetch_friend_libs(&friends, &token);

    // 3) Croisement : pour chaque ami lisible, marquer les jeux communs.
    let mut friend_out = Vec::with_capacity(friends.len());
    for (f, owned_ids) in friends.iter().zip(libs.into_iter()) {
        let private = owned_ids.is_none();
        let mut common = 0usize;
        if let Some(ids) = owned_ids {
            for id in &ids {
                if let Some(game) = games.get_mut(id) {
                    game.owners.push(f.steam_id.clone());
                    common += 1;
                }
            }
        }
        friend_out.push(FriendLib {
            steam_id: f.steam_id.clone(),
            name: f.name.clone(),
            avatar_url: f.avatar_url.clone(),
            private,
            common_count: common,
        });
    }

    // On ne garde que MES jeux qu'au moins un ami partage.
    let mut games: Vec<CommonGame> = games.into_values().filter(|g| !g.owners.is_empty()).collect();
    games.sort_by(|a, b| {
        b.owners
            .len()
            .cmp(&a.owners.len())
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    // Amis les plus « compatibles » d'abord (privés en dernier).
    friend_out.sort_by(|a, b| {
        a.private
            .cmp(&b.private)
            .then_with(|| b.common_count.cmp(&a.common_count))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Some(FriendsCommon {
        friends: friend_out,
        games,
        fetched_at: now(),
    })
}

/// Récupère la liste d'appids possédés de chaque ami, en parallèle.
/// `None` pour un ami = bibliothèque illisible (profil privé).
fn fetch_friend_libs(friends: &[steam::Friend], token: &str) -> Vec<Option<Vec<String>>> {
    let mut out: Vec<Option<Vec<String>>> = (0..friends.len()).map(|_| None).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..FETCH_WORKERS.min(friends.len().max(1)))
            .map(|_| {
                scope.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= friends.len() {
                            break;
                        }
                        let games = steam::owned_with_token(&friends[i].steam_id, token);
                        // Liste vide = profil privé (indistinguable d'un compte sans jeu).
                        let ids = (!games.is_empty())
                            .then(|| games.into_iter().map(|g| g.id).collect::<Vec<_>>());
                        local.push((i, ids));
                    }
                    local
                })
            })
            .collect();
        for h in handles {
            for (i, ids) in h.join().unwrap_or_default() {
                out[i] = ids;
            }
        }
    });
    out
}

fn cache_path(config_dir: &Path) -> std::path::PathBuf {
    config_dir.join(CACHE_FILE)
}

fn load_cache(config_dir: &Path) -> Option<FriendsCommon> {
    let text = std::fs::read_to_string(cache_path(config_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_cache(config_dir: &Path, data: &FriendsCommon) {
    if let Ok(text) = serde_json::to_string(data) {
        let _ = std::fs::write(cache_path(config_dir), text);
    }
}
