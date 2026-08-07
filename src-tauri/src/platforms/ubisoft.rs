use crate::models::GameDto;

/// Scanne les jeux Ubisoft Connect installés, listés dans le registre
/// (`Ubisoft\Launcher\Installs\<gameId>\InstallDir`). Ubisoft n'expose pas de
/// jaquette ni de temps de jeu aux tiers → scan installé + lancement seulement.
pub fn scan() -> Vec<GameDto> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let roots = [
            r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs",
            r"SOFTWARE\Ubisoft\Launcher\Installs",
        ];

        let mut games = Vec::new();
        for root in roots {
            let Ok(installs) = hklm.open_subkey(root) else {
                continue;
            };
            for game_id in installs.enum_keys().flatten() {
                let Ok(key) = installs.open_subkey(&game_id) else {
                    continue;
                };
                let dir: String = key.get_value("InstallDir").unwrap_or_default();
                let title = title_from_dir(&dir);
                if dir.is_empty() || title.is_empty() {
                    continue;
                }

                games.push(GameDto {
                    // gameId = cible du lancement `uplay://launch/<id>/0`.
                    id: format!("ubisoft:{game_id}"),
                    title,
                    platform: "ubisoft".into(),
                    installed: true,
                    size_gb: GameDto::bytes_to_gb(super::dir_size(std::path::Path::new(&dir))),
                    install_dir: Some(dir),
                    launch_target: game_id,
                    ..Default::default()
                });
            }
        }
        return games;
    }

    #[cfg(not(windows))]
    Vec::new()
}

/// Titre déduit du dossier d'installation (dernier segment, ex.
/// « E:/Games/Assassin's Creed Shadows/ » → « Assassin's Creed Shadows »).
#[cfg(windows)]
fn title_from_dir(dir: &str) -> String {
    dir.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// Bibliothèque possédée (jeux non installés) — SANS API ni login.
//
// Ubisoft Connect ne fournit pas d'API tierce pour lister les jeux possédés, mais
// son client maintient un cache local `configurations` (comme Playnite l'exploite) :
// un message **protobuf** `UplayCacheGameCollection { repeated UplayCacheGame games=1 }`
// où chaque `UplayCacheGame { uplay_id=1, install_id=2, game_info=3 }` associe l'id de
// lancement à un bloc **YAML** (nom, jaquettes, addons…). On le décode et on garde les
// jeux de base lançables (hors DLC / versions d'autres plateformes / non-lançables).
// ---------------------------------------------------------------------------

/// Base CDN des visuels Ubisoft (jaquette = ASSET_BASE + nom de fichier du YAML).
const ASSET_BASE: &str = "https://ubistatic3-a.akamaihd.net/orbit/uplay_launcher_3_0/assets/";

/// Jeux possédés lus depuis le cache local d'Ubisoft Connect.
pub fn owned() -> Vec<GameDto> {
    let Some(path) = config_cache_path() else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read(&path) else {
        return Vec::new();
    };

    // 1er passage : parse le YAML de chaque entrée + collecte les ids de DLC à ignorer
    // (les `addons` référencés par les jeux, et les entrées `is_ulc` elles-mêmes).
    let mut ignore: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut parsed: Vec<(u32, yaml_rust2::Yaml)> = Vec::new();
    for (id, info) in parse_cache(&data) {
        let Ok(docs) = yaml_rust2::YamlLoader::load_from_str(&info) else {
            continue;
        };
        let Some(doc) = docs.into_iter().next() else {
            continue;
        };
        let root = &doc["root"];
        if let Some(addons) = root["addons"].as_vec() {
            for a in addons {
                if let Some(aid) = a["id"].as_i64() {
                    ignore.insert(aid as u32);
                }
            }
        }
        if yaml_true(&root["is_ulc"]) {
            ignore.insert(id);
        }
        parsed.push((id, doc));
    }

    // 2e passage : on ne garde que les jeux de base lançables.
    let mut games = Vec::new();
    for (id, doc) in &parsed {
        let root = &doc["root"];
        if ignore.contains(id) // DLC
            || !root["third_party_platform"].is_badvalue() // version Steam/Epic/… d'un jeu
            || root["start_game"].is_badvalue()
        // pas d'exécutable → non lançable (méta/DLC)
        {
            continue;
        }

        // Certains libellés sont des clés de localisation → on résout via localizations.default.
        let loc = &doc["localizations"]["default"];
        let resolve = |key: &str| -> String {
            loc[key].as_str().unwrap_or(key).to_string()
        };

        let Some(raw_name) = root["name"].as_str() else {
            continue;
        };
        let title = strip_trademarks(&resolve(raw_name));
        if title.is_empty() {
            continue;
        }

        let cover_url = root["thumb_image"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| format!("{ASSET_BASE}{}", resolve(s)));

        games.push(GameDto {
            id: format!("ubisoft:{id}"),
            title,
            platform: "ubisoft".into(),
            installed: false,
            owned: true,
            cover_url,
            // Lancement possédé/non installé : `uplay://launch/<uplay_id>/0`.
            launch_target: id.to_string(),
            ..Default::default()
        });
    }
    games
}

/// `%LOCALAPPDATA%\Ubisoft Game Launcher\cache\configuration\configurations`.
fn config_cache_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let path = std::path::Path::new(&base)
        .join("Ubisoft Game Launcher")
        .join("cache")
        .join("configuration")
        .join("configurations");
    path.is_file().then_some(path)
}

/// `is_ulc: yes` / `true` → vrai (yaml-rust2 ne mappe pas `yes` sur un booléen).
fn yaml_true(y: &yaml_rust2::Yaml) -> bool {
    y.as_bool() == Some(true) || y.as_str() == Some("yes")
}

/// Retire les symboles de marque (™ ® ©) et normalise les espaces.
fn strip_trademarks(s: &str) -> String {
    s.replace(['™', '®', '©'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Décodeur protobuf minimal du cache `configurations` → liste `(uplay_id, game_info)`.
fn parse_cache(data: &[u8]) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut top = Reader::new(data);
    while !top.eof() {
        let Some((field, wire)) = top.tag() else { break };
        // Collection.games = champ 1 (répété, délimité par longueur).
        if field == 1 && wire == 2 {
            let Some(msg) = top.len_delimited() else { break };
            let mut g = Reader::new(msg);
            let mut uplay_id = 0u32;
            let mut game_info = String::new();
            while !g.eof() {
                let Some((f, w)) = g.tag() else { break };
                match (f, w) {
                    (1, 0) => uplay_id = g.varint().unwrap_or(0) as u32, // uplay_id
                    (3, 2) => {
                        // game_info (string YAML)
                        if let Some(b) = g.len_delimited() {
                            game_info = String::from_utf8_lossy(b).into_owned();
                        }
                    }
                    (_, w) => {
                        if g.skip(w).is_none() {
                            break;
                        }
                    }
                }
            }
            if uplay_id != 0 && !game_info.is_empty() {
                out.push((uplay_id, game_info));
            }
        } else if top.skip(wire).is_none() {
            break;
        }
    }
    out
}

/// Lecteur protobuf bas-niveau (varints + champs délimités par longueur).
struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    fn eof(&self) -> bool {
        self.i >= self.b.len()
    }
    fn varint(&mut self) -> Option<u64> {
        let mut val = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = *self.b.get(self.i)?;
            self.i += 1;
            val |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(val);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
    }
    /// Lit un bloc préfixé par sa longueur (wire type 2).
    fn len_delimited(&mut self) -> Option<&'a [u8]> {
        let len = self.varint()? as usize;
        let slice = self.b.get(self.i..self.i.checked_add(len)?)?;
        self.i += len;
        Some(slice)
    }
    /// Lit un tag protobuf → (numéro de champ, wire type).
    fn tag(&mut self) -> Option<(u64, u64)> {
        let t = self.varint()?;
        Some((t >> 3, t & 0x7))
    }
    /// Saute un champ du wire type donné.
    fn skip(&mut self, wire: u64) -> Option<()> {
        match wire {
            0 => {
                self.varint()?;
            }
            1 => {
                self.b.get(self.i..self.i.checked_add(8)?)?;
                self.i += 8;
            }
            2 => {
                self.len_delimited()?;
            }
            5 => {
                self.b.get(self.i..self.i.checked_add(4)?)?;
                self.i += 4;
            }
            _ => return None,
        }
        Some(())
    }
}
