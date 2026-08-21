//! Dernier scan de bibliothèque, persisté sur disque.
//!
//! Un scan complet n'est pas instantané : il lit le registre et les manifestes, puis
//! interroge le réseau pour chaque compte connecté (Steam, GOG, Epic…). Faire attendre
//! ce trajet avant d'afficher quoi que ce soit, c'est plusieurs secondes d'écran de
//! démarrage à chaque lancement — et une bibliothèque vide quand la machine est hors ligne.
//!
//! On garde donc le résultat du dernier scan : le front l'affiche immédiatement, puis
//! remplace la liste quand le vrai scan arrive. C'est un cache pur (le fichier porte
//! « cache » dans son nom, donc « Vider le cache » le supprime) : sa perte ne coûte
//! qu'un démarrage sur écran d'attente, comme avant.

use crate::models::GameDto;
use std::path::{Path, PathBuf};

fn file(config_dir: &Path) -> PathBuf {
    // Versionné : à incrémenter si le schéma `GameDto` change de façon incompatible.
    config_dir.join("library_cache_v1.json")
}

/// Bibliothèque du dernier scan (vide si aucun scan mémorisé ou fichier illisible).
pub fn load(config_dir: &Path) -> Vec<GameDto> {
    std::fs::read_to_string(file(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Mémorise le résultat d'un scan. Best-effort : un échec d'écriture ne doit jamais
/// faire échouer le scan lui-même.
pub fn save(config_dir: &Path, games: &[GameDto]) {
    if games.is_empty() || std::fs::create_dir_all(config_dir).is_err() {
        return;
    }
    let Ok(json) = serde_json::to_string(games) else {
        return;
    };
    // Écriture atomique : un cache tronqué serait relu comme « vide » au prochain
    // démarrage, ce qui annulerait tout l'intérêt du cache.
    let path = file(config_dir);
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("torii-libcache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(load(&dir).is_empty());
        let games = vec![GameDto {
            id: "steam:440".into(),
            title: "Team Fortress 2".into(),
            platform: "steam".into(),
            installed: true,
            ..Default::default()
        }];
        save(&dir, &games);
        let back = load(&dir);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, "steam:440");
        assert_eq!(back[0].title, "Team Fortress 2");
        assert!(back[0].installed);

        // Un scan vide n'écrase pas le cache (échec de scan ≠ bibliothèque vide).
        save(&dir, &[]);
        assert_eq!(load(&dir).len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
