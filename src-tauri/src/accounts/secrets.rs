use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Identifiants stockés localement (dossier config de l'app).
/// ⚠️ En clair pour l'instant — à déplacer vers le trousseau OS plus tard.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    /// Cookie de session Steam côté store : "steamLoginSecure=…; sessionid=…".
    pub steam_login_secure: Option<String>,
    /// Cookie de session côté communauté (pour la page des jeux XML).
    pub steam_community: Option<String>,
    /// Clé API Steam (chemin avancé/optionnel, non exposé par défaut).
    pub steam_api_key: Option<String>,
    pub steam_id: Option<String>,
    /// Refresh token Steam (~200 j) capté au login. Permet de régénérer un cookie
    /// de session frais sans reconnexion (le cookie web expire en ~24 h).
    pub steam_refresh_token: Option<String>,
    /// Jeton de rafraîchissement GOG (les access tokens expirent en ~1 h ; on
    /// stocke le refresh token et on redérive l'access token à chaque sync).
    pub gog_refresh_token: Option<String>,
    /// Jeton de rafraîchissement Epic (même principe : access token ~8 h).
    pub epic_refresh_token: Option<String>,
}

fn file(config_dir: &Path) -> PathBuf {
    config_dir.join("credentials.json")
}

pub fn load(config_dir: &Path) -> Credentials {
    std::fs::read_to_string(file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, creds: &Credentials) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(creds).map_err(|e| e.to_string())?;
    std::fs::write(file(config_dir), json).map_err(|e| e.to_string())
}
