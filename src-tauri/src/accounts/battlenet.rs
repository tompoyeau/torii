//! Bibliothèque possédée Battle.net (Blizzard).
//!
//! Blizzard a un catalogue petit et FIXE (pas d'API catalogue tierce). On récupère les
//! jeux possédés en lisant `account.battle.net/api/games-and-subs` (JSON authentifié par
//! cookie de session, comme le fait Playnite dans sa webview) : chaque `gameAccount` a un
//! `titleId` qu'on associe à un catalogue codé en dur (`CATALOG` : titleId → code produit,
//! nom, jaquette). Lancement via le protocole `battlenet://<productId>/`.

use crate::models::GameDto;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

const GAMES_URL: &str = "https://account.battle.net/api/games-and-subs";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36";

/// Une entrée du catalogue Blizzard fixe.
struct Cat {
    /// `titleId` renvoyé par `games-and-subs` (identifie le jeu possédé).
    api_id: u64,
    /// Code produit pour le deeplink `battlenet://<code>/`.
    product_id: &'static str,
    /// Codename interne (uid du registre `--uid=` / product.db) → détection des installés.
    internal_id: &'static str,
    name: &'static str,
    /// Jaquette CDN Blizzard (None → dégradé).
    cover: Option<&'static str>,
}

const fn c(api_id: u64, product_id: &'static str, internal_id: &'static str, name: &'static str, cover: Option<&'static str>) -> Cat {
    Cat { api_id, product_id, internal_id, name, cover }
}

/// Catalogue fixe (extrait de Playnite `BattleNetGames.cs` + codenames réels).
const CATALOG: &[Cat] = &[
    c(5730135, "WoW", "wow", "World of Warcraft", Some("http://bnetproduct-a.akamaihd.net//fab/a25ed0ddd3225929bc3ad5139ebc7483-prod-card-tall.jpg")),
    c(17459, "D3", "diablo3", "Diablo III", Some("http://bnetproduct-a.akamaihd.net//fbd/bafaafcfb7c6c620067662a04409ba66-prod-card-tall.jpg")),
    c(21298, "S2", "s2", "StarCraft II", Some("http://bnetproduct-a.akamaihd.net//fd8/18fb5862b6d5aea418ad4102ed48aa63-prod-card-tall.jpg")),
    c(21297, "S1", "s1", "StarCraft", Some("http://bnetproduct-a.akamaihd.net//f95/6d9453be1750dbf035f0ee574cff2c25-prod-card-tall.jpg")),
    c(1465140039, "WTCG", "hs_beta", "Hearthstone", Some("http://bnetproduct-a.akamaihd.net//f89/c074270c5024a5bb627d46cddf024dad-prod-card-tall.jpg")),
    c(1214607983, "Hero", "heroes", "Heroes of the Storm", Some("http://bnetproduct-a.akamaihd.net//f8c/0f2efeb8d64127edb647a95c236c92ba-prod-card-tall.jpg")),
    c(5272175, "Pro", "prometheus", "Overwatch 2", None),
    c(1447645266, "VIPR", "viper", "Call of Duty: Black Ops 4", None),
    c(1329875278, "ODIN", "odin", "Call of Duty: Modern Warfare", None),
    c(22323, "W3", "w3", "Warcraft III: Reforged", None),
    c(1279351378, "LAZR", "lazarus", "Call of Duty: Modern Warfare 2 Campaign Remastered", None),
    c(1514493267, "ZEUS", "zeus", "Call of Duty: Black Ops Cold War", None),
    c(1464615513, "WLBY", "wlby", "Crash Bandicoot 4", None),
    c(5198665, "OSI", "osi", "Diablo II: Resurrected", None),
    c(1381257807, "RTRO", "rtro", "Blizzard Arcade Collection", None),
    c(1179603525, "FORE", "fore", "Call of Duty: Vanguard", None),
    c(1095647827, "ANBS", "anbs", "Diablo Immortal", None),
    c(1096108883, "AUKS", "auks", "Call of Duty: Modern Warfare II", None),
    c(4613486, "Fen", "fenris", "Diablo IV", None),
    c(1146246220, "D1", "d1", "Diablo", None),
    c(5714258, "W1R", "w1r", "Warcraft: Remastered", None),
    c(5714514, "W2R", "w2r", "Warcraft II: Remastered", None),
    c(1463898673, "W1", "w1", "Warcraft: Orcs & Humans", None),
    c(1462911566, "W2", "w2", "Warcraft II: Battle.net Edition", None),
    c(4674137, "GRY", "gryphon", "Warcraft Rumble", None),
    c(1095911763, "ARIS", "aris", "Doom: The Dark Ages", None),
    c(1396920146, "SCOR", "scorpio", "Sea of Thieves", None),
    c(4280907, "ARK", "arkansas", "The Outer Worlds 2", None),
    c(1279414849, "LBRA", "libra", "Tony Hawk's Pro Skater 3 + 4", None),
    c(1095849281, "AQUA", "aqua", "Avowed", None),
];

/// Récupère les jeux possédés à partir du header Cookie de session `account.battle.net`.
/// Renvoie une liste vide tant que l'utilisateur n'est pas connecté (l'API répond sans jeux).
pub fn fetch_library(cookie: &str) -> Vec<GameDto> {
    let Some(json) = get_games_and_subs(cookie) else {
        return Vec::new();
    };
    let Some(accounts) = json["gameAccounts"].as_array() else {
        return Vec::new();
    };

    let mut games = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for acc in accounts {
        let Some(title_id) = acc["titleId"].as_u64() else {
            continue;
        };
        let Some(cat) = CATALOG.iter().find(|e| e.api_id == title_id) else {
            continue; // titre inconnu de notre catalogue (jeu très récent) → ignoré
        };
        if !seen.insert(cat.product_id) {
            continue; // dédoublonnage (plusieurs comptes de jeu pour un même titre)
        }
        games.push(GameDto {
            id: format!("battlenet:{}", cat.product_id),
            title: cat.name.to_string(),
            platform: "battlenet".into(),
            installed: false,
            owned: true,
            cover_url: cat.cover.map(str::to_string),
            // Lancement / installation via le protocole client : `battlenet://<code>/`.
            launch_target: cat.product_id.to_string(),
            ..Default::default()
        });
    }
    games
}

/// Appelle `games-and-subs` avec le cookie de session. `None` si non authentifié / erreur.
fn get_games_and_subs(cookie: &str) -> Option<Value> {
    let resp = ureq::get(GAMES_URL)
        .timeout(Duration::from_secs(20))
        .set("Cookie", cookie)
        .set("User-Agent", UA)
        .set("Accept", "application/json")
        .set("Referer", "https://account.battle.net/")
        .call()
        .ok()?;
    resp.into_json().ok()
}

// --- Cache disque (snapshot pris à la connexion) --------------------------------------

fn library_path(config_dir: &Path) -> PathBuf {
    config_dir.join("battlenet_library.json")
}

pub fn save_library(config_dir: &Path, games: &[GameDto]) {
    if let Ok(json) = serde_json::to_string(games) {
        let _ = std::fs::write(library_path(config_dir), json);
    }
}

pub fn load_library(config_dir: &Path) -> Vec<GameDto> {
    let mut games: Vec<GameDto> = std::fs::read_to_string(library_path(config_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Statut « installé » depuis le registre (live), croisé par code produit.
    let installed = installed_products();
    for game in &mut games {
        if let Some(path) = installed.get(&game.launch_target) {
            game.installed = true;
            game.size_gb = GameDto::bytes_to_gb(crate::platforms::dir_size(Path::new(path)));
            game.install_dir = Some(path.clone());
        }
    }
    games
}

/// Jeux Battle.net installés → table `code produit -> chemin d'installation`.
///
/// Lus depuis les entrées de désinstallation Windows dont la commande contient
/// `Battle.net … --uid=<codename>` ; le codename est mappé au code produit via le catalogue.
fn installed_products() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let roots = [
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ];
        for root in roots {
            let Ok(uninstall) = hklm.open_subkey(root) else {
                continue;
            };
            for sub in uninstall.enum_keys().flatten() {
                let Ok(key) = uninstall.open_subkey(&sub) else {
                    continue;
                };
                let cmd: String = key.get_value("UninstallString").unwrap_or_default();
                let dir: String = key.get_value("InstallLocation").unwrap_or_default();
                // Extrait le codename de `… --uid=<codename> …`.
                let Some(uid) = cmd
                    .split("--uid=")
                    .nth(1)
                    .map(|s| s.split([' ', '"']).next().unwrap_or("").trim())
                    .filter(|s| !s.is_empty() && *s != "battle.net")
                else {
                    continue;
                };
                if dir.is_empty() {
                    continue;
                }
                if let Some(cat) = CATALOG
                    .iter()
                    .find(|e| e.internal_id.eq_ignore_ascii_case(uid))
                {
                    map.insert(cat.product_id.to_string(), dir);
                }
            }
        }
    }
    map
}

pub fn is_connected(config_dir: &Path) -> bool {
    library_path(config_dir).is_file()
}

pub fn disconnect(config_dir: &Path) {
    let _ = std::fs::remove_file(library_path(config_dir));
}
