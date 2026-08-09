use serde::{Deserialize, Serialize};

/// Jeu détecté par un scanner de plateforme, envoyé tel quel au frontend.
/// Sérialisé en camelCase pour coller au type TypeScript `GameDto`.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameDto {
    /// Identifiant unique préfixé par la plateforme, ex: "steam:440".
    pub id: String,
    pub title: String,
    /// "steam" | "epic" | "gog" | "manual"
    pub platform: String,
    #[serde(default = "yes")]
    pub installed: bool,
    /// Présent dans la bibliothèque du compte (installé ou non).
    #[serde(default)]
    pub owned: bool,
    /// Jeu accessible via le partage familial Steam (possédé par un proche).
    #[serde(default)]
    pub family_shared: bool,
    /// SteamIDs des membres du groupe familial qui possèdent ce jeu (= nombre de
    /// copies dans la famille). Vide hors partage familial Steam.
    #[serde(default)]
    pub family_owners: Vec<String>,
    /// Temps de jeu total en minutes (compte Steam), si connu.
    #[serde(default)]
    pub playtime_minutes: Option<u32>,
    /// Taille sur disque en Go (base 1024³).
    #[serde(default)]
    pub size_gb: f64,
    #[serde(default)]
    pub install_dir: Option<String>,
    /// Jaquette portrait (grille) si disponible localement / via CDN.
    #[serde(default)]
    pub cover_url: Option<String>,
    /// Visuel paysage (hero, tuiles Salon, bannière détail).
    #[serde(default)]
    pub hero_url: Option<String>,
    /// Cible de lancement : appid Steam, AppName Epic, chemin d'exécutable…
    pub launch_target: String,
    /// Horodatage Unix de la dernière partie, si connu.
    #[serde(default)]
    pub last_played: Option<i64>,
    // --- Métadonnées enrichies (fournisseur en ligne) ---
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub developer: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    /// Type Steam ("game", "dlc", "music", "video", "demo"…) pour filtrer les non-jeux.
    #[serde(default)]
    pub app_type: Option<String>,
    /// Masqué par l'utilisateur (liste d'exclusion) : caché des vues normales.
    #[serde(default)]
    pub hidden: bool,
    /// Épinglé par l'utilisateur (filtre « Favoris »).
    #[serde(default)]
    pub favorite: bool,
}

impl GameDto {
    pub fn bytes_to_gb(bytes: u64) -> f64 {
        (bytes as f64 / 1024.0_f64.powi(3) * 10.0).round() / 10.0
    }
}

/// Métadonnées récupérées en ligne et mises en cache sur disque.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameMeta {
    /// Nom officiel (résout les titres provisoires « App <id> » des jeux possédés).
    pub name: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub developer: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub app_type: Option<String>,
    /// Taille de téléchargement en Go (jeux possédés non installés ; GOG).
    #[serde(default)]
    pub size_gb: Option<f64>,
}

fn yes() -> bool {
    true
}
