pub mod battlenet;
pub mod ea;
pub mod epic;
pub mod friends_games;
pub mod gog;
pub mod secrets;
pub mod steam;

use crate::models::GameDto;
use std::path::Path;

/// Régénère un **cookie web Steam frais** via le refresh token (~200 j) quand la session
/// communautaire (~24 h) est expirée, et le **persiste** (store + communauté). `None` si pas
/// de refresh token ou échec du rejeu de login. Un `Some` = cookie valide (le rejeu a réussi).
/// Partagé par tous les chemins qui dépendent de la session web (biblio, amis, jeux en commun).
pub(crate) fn refresh_community_cookie(
    config_dir: &Path,
    creds: &secrets::Credentials,
    steam_id: Option<&str>,
) -> Option<String> {
    let rt = creds.steam_refresh_token.as_deref()?;
    let id = steam_id?;
    let fresh = steam::refresh_web_cookie(rt, id)?;
    // Le cookie frais sert au store comme à la communauté → on persiste les deux.
    let mut updated = creds.clone();
    updated.steam_community = Some(fresh.clone());
    updated.steam_login_secure = Some(fresh.clone());
    let _ = secrets::save(config_dir, &updated);
    Some(fresh)
}

/// (owned_games) Régénère la session puis relit la biblio communautaire. `None` si vide.
fn refresh_steam_community(
    config_dir: &Path,
    creds: &secrets::Credentials,
    steam_id: Option<&str>,
) -> Option<Vec<GameDto>> {
    let id = steam_id?;
    let fresh = refresh_community_cookie(config_dir, creds, Some(id))?;
    let games = steam::owned_from_community(id, &fresh);
    (!games.is_empty()).then_some(games)
}

/// Liste d'amis Steam **avec régénération de session au besoin** : si le scrape renvoie vide
/// (cookie ~24 h expiré, symptôme = liste d'amis absente alors que le compte est « connecté »),
/// on régénère le cookie via le refresh token et on réessaie une fois. Sans ce repli, seul le
/// chemin biblio rafraîchissait la session → les amis restaient vides jusqu'au prochain scan.
pub fn steam_friends_list(config_dir: &Path) -> Vec<steam::Friend> {
    let creds = secrets::load(config_dir);
    let Some(steam_id) = creds.steam_id.clone().or_else(steam::detect_steam_id) else {
        return Vec::new();
    };
    if let Some(cookie) = creds
        .steam_community
        .clone()
        .or_else(|| creds.steam_login_secure.clone())
    {
        let list = steam::friends(&steam_id, &cookie);
        if !list.is_empty() {
            return list;
        }
    }
    match refresh_community_cookie(config_dir, &creds, Some(steam_id.as_str())) {
        Some(fresh) => steam::friends(&steam_id, &fresh),
        None => Vec::new(),
    }
}

/// Appids de la wishlist Steam, via le WebAPIToken (avec régénération de session au besoin,
/// comme les jeux possédés / les amis). Vide si Steam non connecté ou wishlist inaccessible.
pub fn steam_wishlist_appids(config_dir: &Path) -> Vec<u64> {
    let creds = secrets::load(config_dir);
    let Some(steam_id) = creds.steam_id.clone().or_else(steam::detect_steam_id) else {
        return Vec::new();
    };
    let token = creds
        .steam_community
        .clone()
        .or_else(|| creds.steam_login_secure.clone())
        .and_then(|c| steam::web_api_token(&steam_id, &c))
        .or_else(|| {
            let fresh = refresh_community_cookie(config_dir, &creds, Some(steam_id.as_str()))?;
            steam::web_api_token(&steam_id, &fresh)
        });
    match token {
        Some(t) => steam::wishlist(&steam_id, &t),
        None => Vec::new(),
    }
}

/// Récupère les jeux possédés via les comptes connectés (Steam, GOG et Epic).
pub fn owned_games(config_dir: &Path) -> Vec<GameDto> {
    let creds = secrets::load(config_dir);
    let mut games = Vec::new();

    // Steam, par ordre de préférence :
    // 1) page communautaire (jeux-only + noms + temps de jeu, un seul appel),
    // 2) dynamicstore via session (liste d'appids, inclut DLC — repli),
    // 3) clé API (chemin avancé).
    let steam_id = creds.steam_id.clone().or_else(steam::detect_steam_id);
    let community = creds
        .steam_community
        .as_deref()
        .zip(steam_id.as_deref())
        .map(|(cookie, id)| steam::owned_from_community(id, cookie))
        .unwrap_or_default();

    if !community.is_empty() {
        games.extend(community);
    } else if let Some(refreshed) = refresh_steam_community(config_dir, &creds, steam_id.as_deref())
    {
        // Session web expirée (~24 h) → régénérée via le refresh token, sans reconnexion.
        games.extend(refreshed);
    } else if let Some(cookie) = creds.steam_login_secure.as_deref() {
        games.extend(steam::owned_from_session(config_dir, cookie));
    } else if let Some(key) = creds.steam_api_key.as_deref() {
        if let Some(id) = &steam_id {
            games.extend(steam::owned_games(key, id));
        }
    }

    // GOG : refresh token OAuth → getFilteredProducts.
    if let Some(rt) = creds.gog_refresh_token.as_deref() {
        games.extend(gog::owned_games(config_dir, rt));
    }

    // Epic : refresh token OAuth → assets + catalogue.
    if let Some(rt) = creds.epic_refresh_token.as_deref() {
        games.extend(epic::owned_games(config_dir, rt));
    }

    // EA : snapshot de la bibliothèque pris à la connexion (API Juno) et mis en cache.
    games.extend(ea::load_library(config_dir));

    // Battle.net : snapshot pris à la connexion (API games-and-subs) et mis en cache.
    games.extend(battlenet::load_library(config_dir));

    games
}
