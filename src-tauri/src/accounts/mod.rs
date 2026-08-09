pub mod battlenet;
pub mod ea;
pub mod epic;
pub mod friends_games;
pub mod gog;
pub mod secrets;
pub mod steam;

use crate::models::GameDto;
use std::path::Path;

/// Tente de régénérer la session communautaire Steam via le refresh token
/// (cookie web expiré ~24 h). Persiste le cookie frais et renvoie la biblio si OK.
fn refresh_steam_community(
    config_dir: &Path,
    creds: &secrets::Credentials,
    steam_id: Option<&str>,
) -> Option<Vec<GameDto>> {
    let rt = creds.steam_refresh_token.as_deref()?;
    let id = steam_id?;
    let fresh = steam::refresh_web_cookie(rt, id)?;
    let games = steam::owned_from_community(id, &fresh);
    if games.is_empty() {
        return None;
    }
    // Le cookie frais sert au store comme à la communauté → on persiste les deux.
    let mut updated = creds.clone();
    updated.steam_community = Some(fresh.clone());
    updated.steam_login_secure = Some(fresh);
    let _ = secrets::save(config_dir, &updated);
    Some(games)
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
