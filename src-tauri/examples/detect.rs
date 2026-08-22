//! Diagnostic de la détection des jeux **hors launcher** :
//! `cargo run --release --example detect`.
//!
//! Passe en revue la liste de jeux tenue par la Game Bar de Windows
//! (`HKCU\System\GameConfigStore\Children`), garde les exécutables encore présents sur
//! le disque, et montre ce que Torii en ferait : titre deviné, dossier surveillé, et
//! si le jeu est déjà connu de la bibliothèque (auquel cas il n'est jamais « détecté »).
//!
//! Aucun appel réseau, aucune écriture : c'est un miroir de la décision, pas la décision.

use std::collections::HashSet;

fn main() {
    let dir = std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.tompo.ludo");
    let games = ludo_lib::platforms::library_cache::load(&dir);

    // Dossiers déjà couverts par la bibliothèque : un exécutable qui tombe dessous est
    // reconnu par le surveillant sans passer par la détection.
    // 🔑 Même normalisation que le surveillant : minuscules, séparateurs Windows et
    // **sans `\` final** — les scanners de launcher en laissent un, pas les autres.
    let connus: Vec<String> = games
        .iter()
        .filter_map(|g| g.install_dir.clone())
        .map(|d| d.to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string())
        .filter(|d| !d.is_empty())
        .collect();

    let paths = game_bar_paths();
    let existants: Vec<String> = paths
        .iter()
        .filter(|p| std::path::Path::new(p).exists())
        .cloned()
        .collect();
    println!(
        "Game Bar : {} entrées avec chemin, {} encore installées\n",
        paths.len(),
        existants.len()
    );

    let mut vus: HashSet<String> = HashSet::new();
    let (mut adoptes, mut deja, mut rejetes) = (0, 0, 0);
    for exe in &existants {
        let bas = exe.to_lowercase().replace('/', "\\");
        if connus.iter().any(|d| bas.starts_with(&format!("{d}\\"))) {
            deja += 1;
            continue;
        }
        match ludo_lib::platforms::detected::identify(exe) {
            Some(jeu) => {
                adoptes += 1;
                if vus.insert(jeu.id.clone()) {
                    println!("  {:<32} {}", jeu.title, jeu.install_dir.unwrap_or_default());
                }
            }
            None => {
                rejetes += 1;
                println!("  [écarté] {exe}");
            }
        }
    }

    println!(
        "\n{adoptes} exécutables adoptés ({} jeux distincts), {deja} déjà dans la bibliothèque, {rejetes} écartés",
        vus.len()
    );
}

/// Chemins listés par la Game Bar. Dupliqué ici (et non exposé par la bibliothèque) :
/// c'est un détail d'implémentation de la détection, pas une API.
fn game_bar_paths() -> Vec<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(children) = hkcu.open_subkey(r"System\GameConfigStore\Children") else {
        return Vec::new();
    };
    children
        .enum_keys()
        .flatten()
        .filter_map(|child| {
            let entry = children.open_subkey(&child).ok()?;
            let path: String = entry.get_value("MatchedExeFullPath").ok()?;
            (!path.is_empty()).then_some(path)
        })
        .collect()
}
